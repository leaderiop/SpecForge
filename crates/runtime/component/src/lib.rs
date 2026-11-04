//! The component-model runtime (wasmtime): the production adapter of
//! [`WasmRuntime`]. Every extension call (the declaration, commands, passes,
//! collectors, custom rules, scanners, migration hooks) reaches a guest
//! through that trait, here or in the in-process test adapter
//! (`specforge_wasm::testing::InProcessRuntime`).
//!
//! The guest contract is the `specforge:bridge/bridge` world: one `call`
//! export dispatching the JSON wire protocol by export name. Guests are
//! `wasm32-wasip2` components (rustc emits the component encoding directly
//! for that target).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use specforge_protocol_types::Operation;
use specforge_wasm::runtime::{ExtensionSettings, LoadError, WasmCallResult, WasmRuntime};
use specforge_wasm::sandbox::Limits;
use specforge_wasm::{PortCall, PortSession, Provenance};
use wasmtime::component::{Component, Linker, Resource};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store};
use wasmtime_wasi::p2::DynPollable;
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "wit",
    world: "bridge",
});

pub mod builtins;
mod calls;
use calls::GuestClock;
#[cfg(feature = "testing")]
pub mod committed;
#[cfg(feature = "testing")]
pub mod fixtures;
mod imports;
mod ports;

/// Per-instance state: the WASI context (no capability) and the memory
/// ceiling the limiter enforces. Builtins are pure-compute, but wasip2
/// targets import `wasi:io/poll` from std, so the linker always provides it.
struct HostState {
    table: wasmtime_wasi::ResourceTable,
    wasi: WasiCtx,
    memory: MemoryCeiling,
    /// What this call may do through a `specforge:host` port: pure between calls.
    ports: PortSession,
    /// The call's time, against the extension's limit.
    clock: GuestClock,
}

/// Refuses any growth that takes the instance's memories and tables, all
/// counted together, past `bytes`, trapping the call; records the refusal
/// so the call's trap is named `memory_limit_exceeded` whatever way
/// wasmtime wraps the error (ADR 0037).
struct MemoryCeiling {
    bytes: usize,
    /// What every memory and table of the instance holds now.
    used: usize,
    refused: Option<usize>,
}

/// What one table element is counted as: a function reference is a
/// pointer.
const TABLE_ELEMENT_BYTES: usize = size_of::<usize>();

impl MemoryCeiling {
    fn of(limits: Limits) -> Self {
        MemoryCeiling {
            bytes: (limits.memory_mb as usize) << 20,
            used: 0,
            refused: None,
        }
    }

    /// Admits one memory or table going from `current` to `desired` bytes
    /// within the instance's total, or traps.
    fn grow(&mut self, current: usize, desired: usize) -> wasmtime::Result<()> {
        let total = self.used.saturating_sub(current).saturating_add(desired);
        if total > self.bytes {
            self.refused = Some(total);
            wasmtime::bail!(
                "memory limit exceeded: {total} bytes in all asked, {} MiB allowed",
                self.bytes >> 20
            );
        }
        self.used = total;
        Ok(())
    }
}

impl wasmtime::ResourceLimiter for MemoryCeiling {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if maximum.is_some_and(|max| desired > max) {
            return Ok(false);
        }
        self.grow(current, desired)?;
        Ok(true)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if maximum.is_some_and(|max| desired > max) {
            return Ok(false);
        }
        self.grow(
            current.saturating_mul(TABLE_ELEMENT_BYTES),
            desired.saturating_mul(TABLE_ELEMENT_BYTES),
        )?;
        Ok(true)
    }
}

/// The capabilities a guest gets: none. No preopened directory,
/// environment, arguments or stdin (closed); stdout and stderr discarded;
/// TCP, UDP and name lookup refused (every socket address is denied by
/// default as well). Clocks and randomness are WASI's own (wasip2 std needs
/// them). Spelled out so that a future wasmtime default that allows
/// something cannot grant it silently (ADR 0037).
fn no_capabilities() -> WasiCtx {
    WasiCtx::builder()
        .allow_tcp(false)
        .allow_udp(false)
        .allow_ip_name_lookup(false)
        .build()
}

/// A guest waiting on a timer would block the host thread out of the
/// epoch deadline's reach (it fires only while guest code runs), holding
/// the extension's lock for as long as it asks, and only the time a port
/// takes is not the guest's. So the two calls that make
/// a timer trap instead: a guest has nothing else to wait on, as it holds no
/// stream or socket that can block.
fn refuse_timers(linker: &mut Linker<HostState>) -> wasmtime::Result<()> {
    linker.allow_shadowing(true);
    // shortcut: the version wasmtime-wasi 49 links; a wasmtime upgrade that
    // changes it fails `an_extension_waiting_on_a_timer_does_not_block_the_host`.
    let mut clock = linker.instance("wasi:clocks/monotonic-clock@0.2.12")?;
    for subscribe in ["subscribe-duration", "subscribe-instant"] {
        clock.func_wrap(
            subscribe,
            |_, (_,): (u64,)| -> wasmtime::Result<(Resource<DynPollable>,)> {
                wasmtime::bail!("the sandbox has no timer to wait on (ADR 0037)")
            },
        )?;
    }
    linker.allow_shadowing(false);
    Ok(())
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
struct PluginInstance {
    store: Store<HostState>,
    bindings: Bridge,
    /// What each call into this plugin is held to. The budget is the guest's own time, enforced with wasmtime epoch
    /// interruption (`set_epoch_deadline` is refreshed from it before every `call`): a deadline that passes while a
    /// port ran is moved by what the port took ([`GuestClock`]).
    limits: Limits,
    /// What the instance was made from, to make a fresh one after a trap:
    /// a component instance that trapped cannot be entered again.
    component: Component,
    fuel: u64,
    /// What each call carries (ADR 0288): held with the plugin, not its store, so a restarted instance carries
    /// it too.
    settings: Option<ExtensionSettings>,
    /// The `specforge:host` interfaces the component imports, in import order.
    host_imports: Vec<String>,
}

/// The host's reference rate, in instructions per millisecond: what turns
/// a time budget into a work budget.
pub const FUEL_PER_MS: u64 = 20_000_000;

/// Deterministic instruction budget, given whole to every call (refilled
/// before each), shared by every surface: 30 s at [`FUEL_PER_MS`].
pub const DEFAULT_FUEL_LIMIT: u64 = 30_000 * FUEL_PER_MS;

/// The environment variable that holds every call to a work budget
/// ([`ComponentRuntime::with_work_budget`]): a positive number of
/// instructions.
pub const WORK_BUDGET_ENV: &str = "SPECFORGE_WASM_FUEL";

/// The work budget `value` of [`WORK_BUDGET_ENV`] selects: `None` when it
/// is unset, the value back when it is no positive number.
fn work_budget(value: Option<std::ffi::OsString>) -> Result<Option<u64>, std::ffi::OsString> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.to_str().and_then(|s| s.parse::<u64>().ok()) {
        Some(fuel) if fuel > 0 => Ok(Some(fuel)),
        _ => Err(value),
    }
}

/// Granularity of the epoch ticker: a plugin's `max_execution_ms` deadline is
/// never enforced early, and overshoots by at most two of these intervals
/// (plus scheduling delay). 10 ms keeps deadline precision well under any
/// meaningful budget while the ticker thread costs one wakeup per interval
/// per process.
pub const EPOCH_TICK_MS: u64 = 10;

/// Converts a wall-clock millisecond budget into a number of epoch ticks.
///
/// The ticker runs continuously, so the first tick after a call starts can
/// arrive at any point within one interval — even immediately. Consecutive
/// ticks are at least [`EPOCH_TICK_MS`] apart (`sleep` never wakes early), so
/// `n` ticks guarantee only `(n - 1)` full intervals. A positive budget
/// therefore gets one tick more than it covers: the call is never
/// interrupted before `deadline_ms` has elapsed. Zero stays zero (trap at the
/// first checkpoint).
fn ms_to_ticks(deadline_ms: u64) -> u64 {
    if deadline_ms == 0 {
        0
    } else {
        deadline_ms.div_ceil(EPOCH_TICK_MS) + 1
    }
}

/// Background thread that increments the engine epoch every
/// [`EPOCH_TICK_MS`]. This is wasmtime's documented mechanism for epoch
/// interruption: stores set a deadline in ticks and trap once the engine's
/// epoch (advanced only by this thread) passes it. Stopped and joined when
/// the owning [`ComponentRuntime`] drops.
struct EpochTicker {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl EpochTicker {
    fn spawn(engine: Engine) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            while !stop_flag.load(Ordering::Acquire) {
                std::thread::sleep(std::time::Duration::from_millis(EPOCH_TICK_MS));
                engine.increment_epoch();
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for EpochTicker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A `WasmRuntime` backed by wasmtime 49 Component Model instances.
pub struct ComponentRuntime {
    engine: Engine,
    /// Loaded plugins. The map lock guards membership only; each plugin has
    /// its own lock so calls into DIFFERENT extensions run concurrently while
    /// calls into the SAME extension (its `Store` is single-threaded state)
    /// still serialize (audit C7-10).
    plugins: Mutex<HashMap<String, Arc<Mutex<PluginInstance>>>>,
    /// Each loaded plugin's provenance (ADR 0335 D2.6), beside it under the same name.
    provenances: Mutex<HashMap<String, Provenance>>,
    fuel: u64,
    /// Whether `fuel` replaces the time budget a caller gives a call
    /// ([`Self::with_work_budget`]).
    work_bound: bool,
    /// Drives epoch interruption; must outlive every `Store`.
    _ticker: EpochTicker,
}

impl ComponentRuntime {
    /// Runtime without a compilation cache (unit tests, one-shot tooling).
    pub fn new() -> Self {
        Self::construct(None)
    }

    /// Runtime with wasmtime's on-disk compilation cache enabled.
    ///
    /// The cache MUST be configured before the `Engine` is built — wasmtime
    /// reads the cache setting at construction — so this is a constructor,
    /// not a post-hoc setter (C7-02: the previous `.aot` side cache was a
    /// byte copy that no runtime ever consumed).
    pub fn new_with_compile_cache(dir: PathBuf) -> Self {
        Self::construct(Some(dir))
    }

    /// The runtime a project's extensions load into: with the per-user
    /// compilation cache ([`user_compile_cache_dir`]), or without when it is
    /// switched off. It loads nothing: the environment's extension load
    /// does (`Installed::load`).
    pub fn with_user_cache() -> Self {
        match user_compile_cache_dir() {
            Some(dir) => Self::new_with_compile_cache(dir),
            None => Self::new(),
        }
    }

    fn construct(cache_dir: Option<PathBuf>) -> Self {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        // Epoch interruption enforces each plugin's `max_execution_ms`
        // wall-clock budget (audit C7-10): stores get a deadline in ticks
        // and the [`EpochTicker`] thread advances the engine epoch so long
        // loops trap instead of pinning a host thread past their budget.
        config.epoch_interruption(true);
        // Unix signals, not Mach ports, catch guest faults on macOS too.
        // wasmtime's Mach-port handler thread aborts the whole process when
        // its `mach_msg` wait is interrupted (MACH_RCV_INTERRUPTED), which
        // happens whenever a caught signal (e.g. a SIGCHLD handler some
        // library installs) is delivered to that thread. Signal-based traps
        // have no such thread and are what Linux uses already.
        config.macos_use_mach_ports(false);
        // No native unwind info (`.eh_frame`) for guest code: macOS's libunwind registers and
        // deregisters it one frame description at a time, each a linear search, so freeing a
        // compiled component was most of a session's reload (#159). Nothing unwinds through a
        // guest: traps and wasm backtraces walk frame pointers, and a host panic under a guest
        // is caught and resumed on the host side.
        config.native_unwind_info(false);
        if let Some(dir) = &cache_dir
            && let Err(e) = enable_compile_cache(&mut config, dir)
        {
            eprintln!(
                "warning: wasm compile cache disabled ({}: {e})",
                dir.display()
            );
        }
        let engine = Engine::new(&config).expect("engine initializes");
        let runtime = Self {
            _ticker: EpochTicker::spawn(engine.clone()),
            engine,
            plugins: Mutex::new(HashMap::new()),
            provenances: Mutex::new(HashMap::new()),
            fuel: DEFAULT_FUEL_LIMIT,
            work_bound: false,
        };
        match work_budget(std::env::var_os(WORK_BUDGET_ENV)) {
            Ok(Some(fuel)) => runtime.with_work_budget(fuel),
            Ok(None) => runtime,
            Err(value) => {
                eprintln!(
                    "warning: {WORK_BUDGET_ENV}={value:?} is not a positive number of \
                     instructions; it is ignored"
                );
                runtime
            }
        }
    }

    /// Deterministic per-call instruction budget, enforced by the engine.
    pub fn with_fuel_limit(mut self, fuel: u64) -> Self {
        self.fuel = fuel;
        self
    }

    /// Hold every call to `fuel` instructions (at most
    /// [`DEFAULT_FUEL_LIMIT`]) in place of any clock: the time budget its
    /// caller gives it (a hook's `timeout_ms`) is lifted, and the
    /// extension's own limit (`max_execution_ms`) holds as work, at
    /// [`FUEL_PER_MS`]. Whether such a call answers then depends on its
    /// input, not on how busy the machine is. What [`WORK_BUDGET_ENV`]
    /// selects: a test bounds a guest's work with it on a loaded machine.
    pub fn with_work_budget(mut self, fuel: u64) -> Self {
        self.fuel = fuel.min(DEFAULT_FUEL_LIMIT);
        self.work_bound = true;
        self
    }

    /// Names of the currently loaded extensions, sorted.
    pub fn loaded_names(&self) -> Vec<String> {
        match self.plugins.lock() {
            Ok(plugins) => {
                let mut names: Vec<String> = plugins.keys().cloned().collect();
                names.sort();
                names
            }
            Err(_) => Vec::new(),
        }
    }
    fn instantiate_with_fuel(
        &self,
        name: &str,
        component: Component,
        fuel: u64,
        host_imports: Vec<String>,
    ) -> Result<(), String> {
        let (store, bindings) = self.fresh_instance(&component, fuel, Limits::CEILING)?;
        let mut plugins = self.plugins.lock().map_err(|e| e.to_string())?;
        plugins.insert(
            name.to_string(),
            Arc::new(Mutex::new(PluginInstance {
                store,
                bindings,
                limits: Limits::CEILING,
                component,
                fuel,
                settings: None,
                host_imports,
            })),
        );
        Ok(())
    }

    /// What holds one entry into a plugin under `limits`: its instructions
    /// and its deadline, in milliseconds (`max_ms` is its caller's time
    /// budget). Under a work budget no clock runs: the extension's limit is
    /// held as the work it allows at [`FUEL_PER_MS`].
    fn budget(&self, fuel: u64, limits: Limits, max_ms: u32) -> (u64, u32) {
        if self.work_bound {
            let limit = u64::from(limits.execution_ms).saturating_mul(FUEL_PER_MS);
            (fuel.min(limit), u32::MAX)
        } else {
            (fuel, limits.execution_ms.min(max_ms))
        }
    }

    /// A new instance of `component`, with `fuel`, `limits`' memory ceiling
    /// and its budget armed.
    fn fresh_instance(
        &self,
        component: &Component,
        fuel: u64,
        limits: Limits,
    ) -> Result<(Store<HostState>, Bridge), String> {
        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| format!("failed to add WASI to linker: {e}"))?;
        refuse_timers(&mut linker).map_err(|e| format!("failed to refuse WASI timers: {e}"))?;
        ports::link(&mut linker).map_err(|e| format!("failed to link the host ports: {e}"))?;

        let table = wasmtime_wasi::ResourceTable::new();
        let mut store = Store::new(
            &self.engine,
            HostState {
                table,
                wasi: no_capabilities(),
                memory: MemoryCeiling::of(limits),
                ports: PortSession::pure(),
                clock: GuestClock::of(self.budget(fuel, limits, u32::MAX).1),
            },
        );
        store.limiter(|state| &mut state.memory);
        store.epoch_deadline_callback(|store| {
            let state = store.data();
            Ok(state.clock.at_deadline(state.ports.time_in_ports()))
        });
        let (fuel, deadline_ms) = self.budget(fuel, limits, u32::MAX);
        store
            .set_fuel(fuel)
            .map_err(|e| format!("failed to set fuel: {e}"))?;
        // With epoch interruption enabled, stores start with a deadline of
        // zero ticks and would trap immediately — arm the plugin's
        // budget before any guest code can run.
        store.set_epoch_deadline(ms_to_ticks(u64::from(deadline_ms)));

        let bindings = Bridge::instantiate(&mut store, component, &linker)
            .map_err(|e| format!("failed to instantiate component: {e}"))?;
        Ok((store, bindings))
    }
}

impl Default for ComponentRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime for ComponentRuntime {
    /// Compile a component from bytes and register it under `name`,
    /// atomically replacing any existing plugin (hot reload / H1).
    fn load(&self, name: &str, bytes: &[u8], provenance: Provenance) -> Result<(), LoadError> {
        let component = Component::from_binary(&self.engine, bytes)
            .map_err(|e| LoadError::Unloadable(format!("failed to compile component: {e}")))?;
        // Read before instantiating: the linker would refuse an unknown port as a missing implementation.
        imports::admit(
            component
                .component_type()
                .imports(&self.engine)
                .map(|(name, _)| name),
        )?;
        let host_imports = component
            .component_type()
            .imports(&self.engine)
            .map(|(name, _)| name)
            .filter(|name| name.starts_with("specforge:host/"))
            .map(str::to_string)
            .collect();
        self.instantiate_with_fuel(name, component, self.fuel, host_imports)
            .map_err(LoadError::Unloadable)?;
        if let Ok(mut provenances) = self.provenances.lock() {
            provenances.insert(name.to_string(), provenance);
        }
        Ok(())
    }

    fn provenance(&self, name: &str) -> Option<Provenance> {
        self.provenances.lock().ok()?.get(name).cloned()
    }

    fn rename(&self, from: &str, to: &str) -> bool {
        if let Ok(mut provenances) = self.provenances.lock()
            && let Some(provenance) = provenances.remove(from)
        {
            provenances.insert(to.to_string(), provenance);
        }
        let Ok(mut plugins) = self.plugins.lock() else {
            return false;
        };
        match plugins.remove(from) {
            Some(plugin) => {
                plugins.insert(to.to_string(), plugin);
                true
            }
            None => false,
        }
    }

    fn unload(&self, name: &str) -> bool {
        if let Ok(mut provenances) = self.provenances.lock() {
            provenances.remove(name);
        }
        match self.plugins.lock() {
            Ok(mut plugins) => plugins.remove(name).is_some(),
            Err(_) => false,
        }
    }

    /// Call the bridge `call` export; returns the raw JSON wire bytes.
    fn call_export(&self, name: &str, export: &str, input: &[u8]) -> WasmCallResult {
        self.call_export_within(name, export, input, u32::MAX)
    }

    fn call_export_within(
        &self,
        name: &str,
        export: &str,
        input: &[u8],
        max_ms: u32,
    ) -> WasmCallResult {
        self.call_in_session(name, export, input, max_ms, |_| PortSession::pure())
    }

    fn call_with_ports(
        &self,
        name: &str,
        export: &str,
        input: &[u8],
        kind: Operation,
        lent: Option<PortCall>,
    ) -> WasmCallResult {
        let session = |limits| PortSession::effectful(name, kind, lent, limits);
        let plugin = match self.plugin(name, export) {
            Ok(plugin) => plugin,
            Err(trap) => return trap,
        };
        // What an own instance is made from, copied under the lock and only then let go: a port can take minutes, and
        // every other call of this extension waits on that lock.
        let own = match plugin.lock() {
            Ok(instance) if !instance.host_imports.is_empty() => {
                Some((instance.component.clone(), instance.limits, instance.fuel))
            }
            _ => None,
        };
        match own {
            Some((component, limits, fuel)) => {
                self.call_on_own_instance(name, export, input, &component, limits, fuel, session)
            }
            None => self.call_in_session(name, export, input, u32::MAX, session),
        }
    }

    fn host_imports(&self, extension_name: &str) -> Vec<String> {
        let Ok(plugins) = self.plugins.lock() else {
            return Vec::new();
        };
        plugins
            .get(extension_name)
            .and_then(|plugin| plugin.lock().ok().map(|p| p.host_imports.clone()))
            .unwrap_or_default()
    }

    fn apply_limits(&self, extension_name: &str, limits: Limits) {
        let Ok(plugins) = self.plugins.lock() else {
            return;
        };
        if let Some(plugin) = plugins.get(extension_name)
            && let Ok(mut plugin) = plugin.lock()
        {
            plugin.limits = limits;
            plugin.store.data_mut().memory.bytes = MemoryCeiling::of(limits).bytes;
        }
    }

    fn apply_settings(&self, extension_name: &str, settings: ExtensionSettings) {
        let Ok(plugins) = self.plugins.lock() else {
            return;
        };
        if let Some(plugin) = plugins.get(extension_name)
            && let Ok(mut plugin) = plugin.lock()
        {
            plugin.settings = Some(settings);
        }
    }

    fn settings_of(&self, extension_name: &str) -> Option<ExtensionSettings> {
        let plugin = Arc::clone(self.plugins.lock().ok()?.get(extension_name)?);
        let plugin = plugin.lock().ok()?;
        plugin.settings.clone()
    }
}

/// Per-user Wasmtime compilation cache directory.
///
/// `$SPECFORGE_WASMTIME_CACHE` overrides; setting it to `off` disables the
/// cache. Default: `$HOME/.cache/specforge/wasmtime` (no cache when `HOME`
/// is unset). The test drivers read it, so a child running under another `HOME` shares the cache of the test process
/// (a cache of its own would compile every builtin again).
pub fn user_compile_cache_dir() -> Option<PathBuf> {
    match std::env::var_os("SPECFORGE_WASMTIME_CACHE") {
        Some(v) if v == "off" => None,
        Some(v) => Some(PathBuf::from(v)),
        None => std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".cache")
                .join("specforge")
                .join("wasmtime")
        }),
    }
}

/// Configure wasmtime's native on-disk compilation cache (true AOT: compiled
/// machine code is cached and deserialized on later runs, keyed by input
/// bytes + engine config). Covers components — `compile_component` goes
/// through wasmtime's `ModuleCacheEntry` like modules do. The soft size cap
/// keeps the governance constraint (500 MB, size-based eviction) true.
fn enable_compile_cache(config: &mut Config, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut cache_config = CacheConfig::new();
    cache_config.with_directory(dir);
    cache_config.with_files_total_size_soft_limit(500 * 1024 * 1024);
    let cache = Cache::new(cache_config).map_err(|e| e.to_string())?;
    config.cache(Some(cache));
    Ok(())
}

#[cfg(test)]
mod deadline_tests {
    use super::{EPOCH_TICK_MS, ms_to_ticks, work_budget};

    #[test]
    fn a_work_budget_is_a_positive_number_of_instructions() {
        assert_eq!(work_budget(None), Ok(None));
        assert_eq!(
            work_budget(Some("4000000000".into())),
            Ok(Some(4_000_000_000))
        );
        for refused in ["0", "-1", "", "2e9", "lots"] {
            assert_eq!(
                work_budget(Some(refused.into())),
                Err(refused.into()),
                "{refused}"
            );
        }
    }

    #[test]
    fn ms_to_ticks_never_undercounts_the_budget() {
        assert_eq!(ms_to_ticks(0), 0, "zero budget means trap immediately");
        // One tick for the unknown phase of the first tick, plus the
        // intervals the budget covers (rounded up).
        assert_eq!(ms_to_ticks(1), 2);
        assert_eq!(ms_to_ticks(EPOCH_TICK_MS), 2);
        assert_eq!(ms_to_ticks(EPOCH_TICK_MS + 1), 3, "partial ticks round up");
        assert_eq!(ms_to_ticks(30_000), 3_001);
        // n ticks guarantee (n - 1) full intervals: always at least the budget.
        for ms in [1, 9, 10, 11, 49, 50, 51, 5_000] {
            assert!((ms_to_ticks(ms) - 1) * EPOCH_TICK_MS >= ms, "{ms} ms");
        }
    }
}
