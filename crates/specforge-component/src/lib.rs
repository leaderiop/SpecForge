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
use specforge_wasm::sandbox::default_sandbox_policy;
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
    /// Wall-clock budget for each call into this plugin, in milliseconds.
    /// Enforced with wasmtime epoch interruption; `set_epoch_deadline` is
    /// refreshed from this value before every `call`.
    deadline_ms: u64,
}

/// Deterministic per-call instruction budget, shared by every surface.
pub const DEFAULT_FUEL_LIMIT: u64 = 30_000 * 20_000_000;

/// Granularity of the epoch ticker: a plugin's `max_execution_ms` deadline is
/// enforced with a worst-case overshoot of this much wall-clock time. 10 ms
/// keeps deadline precision well under any meaningful budget while the ticker
/// thread costs one wakeup per interval per process.
pub const EPOCH_TICK_MS: u64 = 10;

/// Converts a wall-clock millisecond budget into a number of epoch ticks
/// (rounded up), so any positive budget gets at least one tick.
fn ms_to_ticks(deadline_ms: u64) -> u64 {
    deadline_ms.div_ceil(EPOCH_TICK_MS)
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
    /// Ceiling for per-call wall-clock budgets: a plugin's declared
    /// `max_execution_ms` is clamped to this.
    default_deadline_ms: u64,
    /// Drives epoch interruption; must outlive every `Store`.
    _ticker: EpochTicker,
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
        if let Some(dir) = &cache_dir
            && let Err(e) = enable_compile_cache(&mut config, dir)
        {
            eprintln!(
                "warning: wasm compile cache disabled ({}: {e})",
                dir.display()
            );
        }
        let engine = Engine::new(&config).expect("engine initializes");
        let default_deadline_ms = u64::from(
            default_sandbox_policy()
                .max_execution_ms
                .unwrap_or(u32::MAX),
        );
        Self {
            _ticker: EpochTicker::spawn(engine.clone()),
            engine,
            plugins: Mutex::new(HashMap::new()),
            fuel: DEFAULT_FUEL_LIMIT,
            default_deadline_ms,
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
        // zero ticks and would trap immediately — arm the plugin's default
        // wall-clock budget before any guest code can run.
        store.set_epoch_deadline(ms_to_ticks(self.default_deadline_ms));

        let bindings = Bridge::instantiate(&mut store, &component, &linker)
            .map_err(|e| format!("failed to instantiate component {name}: {e}"))?;

        let mut plugins = self.plugins.lock().map_err(|e| e.to_string())?;
        plugins.insert(
            name.to_string(),
            Arc::new(Mutex::new(PluginInstance {
                store,
                bindings,
                deadline_ms: self.default_deadline_ms,
            })),
        );
        Ok(())
    }

    /// Applies a plugin-declared wall-clock budget (its handshake
    /// `sandbox_policy.max_execution_ms`) to the named extension's
    /// subsequent calls. The budget is clamped to the host's
    /// deny-by-default ceiling: a plugin may tighten its own deadline but
    /// never extend it past the host default.
    pub fn set_execution_deadline_ms(&self, name: &str, max_execution_ms: u64) {
        let effective = self.default_deadline_ms.min(max_execution_ms);
        let plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(_) => return,
        };
        if let Some(plugin) = plugins.get(name)
            && let Ok(mut plugin) = plugin.lock()
        {
            plugin.deadline_ms = effective;
        }
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
            deadline_ms,
        } = &mut *instance;
        store.set_epoch_deadline(ms_to_ticks(*deadline_ms));
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
    fn ms_to_ticks_rounds_up_per_tick_period() {
        assert_eq!(ms_to_ticks(0), 0, "zero budget means trap immediately");
        assert_eq!(ms_to_ticks(1), 1, "any positive budget gets a tick");
        assert_eq!(ms_to_ticks(EPOCH_TICK_MS), 1);
        assert_eq!(ms_to_ticks(EPOCH_TICK_MS + 1), 2, "partial ticks round up");
        assert_eq!(ms_to_ticks(30_000), 3_000);
    }
}
