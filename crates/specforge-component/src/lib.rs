//! Component-model runtime (wasmtime 49 direct) — hardening-plan Phase W2.
//!
//! Implements the same [`WasmRuntime`] trait as `ExtismRuntime`, so every
//! consumer (ProtocolHost, scanner dispatch, analyze pass dispatch, custom
//! rules) keeps working unchanged while the engine underneath moves to the
//! Component Model.
//!
//! The guest contract is the `specforge:bridge/bridge` world: one `call`
//! export dispatching the JSON wire protocol by export name. Guests are
//! `wasm32-wasip2` components (rustc emits the component encoding directly
//! for that target — see `spike/w0-component/`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use specforge_wasm::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};
use specforge_wasm::sandbox::Limits;
use wasmtime::component::{Component, Linker};
use wasmtime::{Cache, CacheConfig, Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "wit",
    world: "bridge",
});

pub mod builtins;
pub mod project;

pub use project::project_runtime;

/// Per-process WASI context for guest components. Builtins are pure-compute,
/// but wasip2 targets import `wasi:io/poll` from std, so the linker always
/// provides it.
struct HostState {
    table: wasmtime_wasi::ResourceTable,
    wasi: WasiCtx,
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
    /// What each call into this plugin is held to. The wall-clock budget
    /// is enforced with wasmtime epoch interruption; `set_epoch_deadline`
    /// is refreshed from it before every `call`.
    limits: Limits,
    /// What the instance was made from, to make a fresh one after a trap:
    /// a component instance that trapped cannot be entered again.
    component: Component,
    fuel: u64,
}

/// Deterministic per-call instruction budget, shared by every surface.
pub const DEFAULT_FUEL_LIMIT: u64 = 30_000 * 20_000_000;

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
    fuel: u64,
    /// Drives epoch interruption; must outlive every `Store`.
    _ticker: EpochTicker,
    /// Why an extension the project enables failed to load (a missing or
    /// tampered installed binary), by name: compile reports it.
    load_failures: Mutex<HashMap<String, specforge_common::Diagnostic>>,
    /// The extension each `.wasm` file entry of `specforge.json` loaded
    /// as (the name its component declares), by the entry.
    file_entries: Mutex<HashMap<String, String>>,
}

impl ComponentRuntime {
    /// Runtime without a compilation cache (unit tests, one-shot tooling).
    pub fn new() -> Self {
        Self::construct(None)
    }

    /// Runtime with wasmtime's on-disk compilation cache enabled
    /// (`SPECFORGE_WASMTIME_CACHE` selection happens in `project_runtime`).
    ///
    /// The cache MUST be configured before the `Engine` is built — wasmtime
    /// reads the cache setting at construction — so this is a constructor,
    /// not a post-hoc setter (C7-02: the previous `.aot` side cache was a
    /// byte copy that no runtime ever consumed).
    pub fn new_with_compile_cache(dir: PathBuf) -> Self {
        Self::construct(Some(dir))
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
        if let Some(dir) = &cache_dir
            && let Err(e) = enable_compile_cache(&mut config, dir)
        {
            eprintln!(
                "warning: wasm compile cache disabled ({}: {e})",
                dir.display()
            );
        }
        let engine = Engine::new(&config).expect("engine initializes");
        Self {
            _ticker: EpochTicker::spawn(engine.clone()),
            engine,
            plugins: Mutex::new(HashMap::new()),
            fuel: DEFAULT_FUEL_LIMIT,
            load_failures: Mutex::new(HashMap::new()),
            file_entries: Mutex::new(HashMap::new()),
        }
    }

    /// Record why `name` could not be loaded, for [`WasmRuntime::load_failure`].
    pub fn record_load_failure(&self, name: &str, diagnostic: specforge_common::Diagnostic) {
        self.load_failures
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name.to_string(), diagnostic);
    }

    /// Record that the `.wasm` file entry `entry` loaded as `extension`,
    /// for [`WasmRuntime::file_entry_extension`].
    pub(crate) fn record_file_entry(&self, entry: &str, extension: &str) {
        self.file_entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(entry.to_string(), extension.to_string());
    }

    /// Register the extension loaded as `from` under `to` instead, without
    /// compiling or instantiating it again. False when `from` is not loaded.
    pub(crate) fn rename(&self, from: &str, to: &str) -> bool {
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

    /// Deterministic per-call instruction budget, enforced by the engine.
    pub fn with_fuel_limit(mut self, fuel: u64) -> Self {
        self.fuel = fuel;
        self
    }

    /// Compile a component from bytes and register it under `name`,
    /// atomically replacing any existing plugin (hot reload / H1).
    pub fn load_module_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        self.load_module_bytes_with_limits(name, wasm_bytes, self.fuel)
    }

    /// Compile a component from bytes with an explicit fuel budget.
    pub fn load_module_bytes_with_limits(
        &self,
        name: &str,
        wasm_bytes: &[u8],
        fuel: u64,
    ) -> Result<(), String> {
        let component = Component::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| format!("failed to compile component {name}: {e}"))?;
        self.instantiate_with_fuel(name, component, fuel)
    }

    /// Compile a component from a file and register it under `name`.
    pub fn load_module_as(&self, name: &str, wasm_path: &Path) -> Result<(), String> {
        let component = Component::from_file(&self.engine, wasm_path)
            .map_err(|e| format!("failed to compile component {name}: {e}"))?;
        self.instantiate(name, component)
    }

    /// Atomically replace a loaded extension's component (hot reload / H1).
    pub fn reload_module_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        self.load_module_bytes(name, wasm_bytes)
    }

    /// Unload an extension. Returns true when it was loaded.
    pub fn unload(&self, name: &str) -> bool {
        match self.plugins.lock() {
            Ok(mut plugins) => plugins.remove(name).is_some(),
            Err(_) => false,
        }
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
    fn instantiate(&self, name: &str, component: Component) -> Result<(), String> {
        self.instantiate_with_fuel(name, component, self.fuel)
    }

    fn instantiate_with_fuel(
        &self,
        name: &str,
        component: Component,
        fuel: u64,
    ) -> Result<(), String> {
        let (store, bindings) = self.fresh_instance(name, &component, fuel, Limits::CEILING)?;
        let mut plugins = self.plugins.lock().map_err(|e| e.to_string())?;
        plugins.insert(
            name.to_string(),
            Arc::new(Mutex::new(PluginInstance {
                store,
                bindings,
                limits: Limits::CEILING,
                component,
                fuel,
            })),
        );
        Ok(())
    }

    /// A new instance of `component`, with `fuel` and `limits`' wall-clock
    /// budget armed.
    fn fresh_instance(
        &self,
        name: &str,
        component: &Component,
        fuel: u64,
        limits: Limits,
    ) -> Result<(Store<HostState>, Bridge), String> {
        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
            .map_err(|e| format!("failed to add WASI to linker: {e}"))?;

        let table = wasmtime_wasi::ResourceTable::new();
        let wasi = wasmtime_wasi::WasiCtx::builder().build();
        let mut store = Store::new(&self.engine, HostState { table, wasi });
        store
            .set_fuel(fuel)
            .map_err(|e| format!("failed to set fuel for {name}: {e}"))?;
        // With epoch interruption enabled, stores start with a deadline of
        // zero ticks and would trap immediately — arm the plugin's
        // wall-clock budget before any guest code can run.
        store.set_epoch_deadline(ms_to_ticks(u64::from(limits.execution_ms)));

        let bindings = Bridge::instantiate(&mut store, component, &linker)
            .map_err(|e| format!("failed to instantiate component {name}: {e}"))?;
        Ok((store, bindings))
    }

    /// Call the bridge `call` export; returns the raw JSON wire bytes.
    pub fn call(&self, name: &str, export: &str, input: &[u8]) -> WasmCallResult {
        // Look up the plugin and release the map lock immediately: holding
        // it across the guest call would serialize every extension behind
        // one mutex (the C7-10 finding). Only the target plugin's own lock
        // is held for the duration of the call.
        let plugin = {
            let plugins = match self.plugins.lock() {
                Ok(p) => p,
                Err(e) => {
                    return WasmCallResult::Trap(WasmTrapInfo {
                        kind: "lock_poisoned".to_string(),
                        message: e.to_string(),
                        export_name: export.to_string(),
                    });
                }
            };
            match plugins.get(name) {
                Some(plugin) => Arc::clone(plugin),
                None => {
                    return WasmCallResult::Trap(WasmTrapInfo {
                        kind: "extension_not_found".to_string(),
                        message: format!("Extension '{name}' not loaded"),
                        export_name: export.to_string(),
                    });
                }
            }
        };
        let mut instance = match plugin.lock() {
            Ok(instance) => instance,
            Err(e) => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "lock_poisoned".to_string(),
                    message: e.to_string(),
                    export_name: export.to_string(),
                });
            }
        };
        let PluginInstance {
            store,
            bindings,
            limits,
            component,
            fuel,
        } = &mut *instance;
        store.set_epoch_deadline(ms_to_ticks(u64::from(limits.execution_ms)));
        match bindings.call_call(&mut *store, name, export, input) {
            Ok(Ok(bytes)) => WasmCallResult::Ok(bytes),
            Ok(Err(message)) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message,
                export_name: export.to_string(),
            }),
            Err(e) => {
                // Epoch-deadline expiry surfaces as `Trap::Interrupt`; map it
                // to a distinct kind so callers can tell a wall-clock
                // timeout apart from other call failures.
                let deadline_hit = matches!(
                    e.downcast_ref::<wasmtime::Trap>(),
                    Some(wasmtime::Trap::Interrupt)
                );
                // An instance that trapped cannot be entered again: the
                // extension's next call gets a fresh one, as a guest that
                // panics in one call must not take the extension down for
                // the rest of the process (an MCP session).
                if let Ok((fresh_store, fresh_bindings)) =
                    self.fresh_instance(name, component, *fuel, *limits)
                {
                    *store = fresh_store;
                    *bindings = fresh_bindings;
                }
                WasmCallResult::Trap(WasmTrapInfo {
                    kind: if deadline_hit {
                        "deadline_exceeded"
                    } else {
                        "call_failed"
                    }
                    .to_string(),
                    message: e.to_string(),
                    export_name: export.to_string(),
                })
            }
        }
    }
}

impl Default for ComponentRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime for ComponentRuntime {
    fn load_module(&self, wasm_path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(wasm_path).map_err(|e| e.to_string())?;
        let name = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.load_module_bytes(&name, &bytes)
    }

    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        self.call(extension_name, export_name, input)
    }

    fn load_module_named(&self, extension_name: &str, wasm_path: &Path) -> Result<(), String> {
        ComponentRuntime::load_module_as(self, extension_name, wasm_path)
    }

    fn apply_limits(&self, extension_name: &str, limits: Limits) {
        let Ok(plugins) = self.plugins.lock() else {
            return;
        };
        if let Some(plugin) = plugins.get(extension_name)
            && let Ok(mut plugin) = plugin.lock()
        {
            plugin.limits = limits;
        }
    }

    fn load_failure(&self, extension_name: &str) -> Option<specforge_common::Diagnostic> {
        self.load_failures
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(extension_name)
            .cloned()
    }

    fn file_entry_extension(&self, entry: &str) -> Option<String> {
        self.file_entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(entry)
            .cloned()
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
    use super::{EPOCH_TICK_MS, ms_to_ticks};

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
