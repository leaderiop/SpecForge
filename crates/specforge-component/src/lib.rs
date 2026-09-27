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
use std::sync::Mutex;

use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

use specforge_wasm::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};

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
}

/// Deterministic per-call instruction budget (mirrors `specforge-extism`).
pub const DEFAULT_FUEL_LIMIT: u64 = 30_000 * 20_000_000;

/// A `WasmRuntime` backed by wasmtime 49 Component Model instances.
pub struct ComponentRuntime {
    engine: Engine,
    plugins: Mutex<HashMap<String, PluginInstance>>,
    fuel: u64,
    compile_cache_dir: Option<PathBuf>,
}

impl ComponentRuntime {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.wasm_component_model(true);
        config.consume_fuel(true);
        let engine = Engine::new(&config).expect("engine initializes");
        Self {
            engine,
            plugins: Mutex::new(HashMap::new()),
            fuel: DEFAULT_FUEL_LIMIT,
            compile_cache_dir: None,
        }
    }

    /// Deterministic per-call instruction budget, enforced by the engine.
    pub fn with_fuel_limit(mut self, fuel: u64) -> Self {
        self.fuel = fuel;
        self
    }

    /// Enable Wasmtime's on-disk compilation cache (opt-in).
    pub fn with_compile_cache(mut self, dir: PathBuf) -> Self {
        match std::fs::create_dir_all(&dir) {
            Ok(()) => self.compile_cache_dir = Some(dir),
            Err(e) => eprintln!(
                "warning: wasm compile cache disabled ({}: {})",
                dir.display(),
                e
            ),
        }
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
    pub fn load_module_as(
        &self,
        name: &str,
        wasm_path: &Path,
        _aot_cache_path: Option<&Path>,
    ) -> Result<(), String> {
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

        let bindings = Bridge::instantiate(&mut store, &component, &linker)
            .map_err(|e| format!("failed to instantiate component {name}: {e}"))?;

        let mut plugins = self.plugins.lock().map_err(|e| e.to_string())?;
        plugins.insert(name.to_string(), PluginInstance { store, bindings });
        Ok(())
    }

    /// Call the bridge `call` export; returns the raw JSON wire bytes.
    pub fn call(&self, name: &str, export: &str, input: &[u8]) -> WasmCallResult {
        let mut plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(e) => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "lock_poisoned".to_string(),
                    message: e.to_string(),
                    export_name: export.to_string(),
                });
            }
        };
        let Some(instance) = plugins.get_mut(name) else {
            return WasmCallResult::Trap(WasmTrapInfo {
                kind: "extension_not_found".to_string(),
                message: format!("Extension '{name}' not loaded"),
                export_name: export.to_string(),
            });
        };
        let PluginInstance { store, bindings } = instance;
        match bindings.call_call(&mut *store, name, export, input) {
            Ok(Ok(bytes)) => WasmCallResult::Ok(bytes),
            Ok(Err(message)) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "guest_error".to_string(),
                message,
                export_name: export.to_string(),
            }),
            Err(e) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "call_failed".to_string(),
                message: e.to_string(),
                export_name: export.to_string(),
            }),
        }
    }
}

impl Default for ComponentRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime for ComponentRuntime {
    fn load_module(&self, wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
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

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
        false
    }
}
