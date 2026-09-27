use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use extism::{Manifest, Plugin, PluginBuilder, Wasm};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};

use crate::host_context::{self, HostContext};

/// Deterministic instruction budget per millisecond of guest work.
///
/// ~20M wasm instructions per millisecond on current Apple Silicon; the
/// exact figure is a policy constant, not a measured contract — what matters
/// is that it is deterministic (R-6) and applied by the engine (C7-10),
/// replacing the previously unenforced `max_execution_ms`.
pub const FUEL_PER_MS: u64 = 20_000_000;

/// Default per-plugin fuel budget: 30 s of guest work at `FUEL_PER_MS`,
/// matching `default_sandbox_policy()`'s 30 s timeout.
pub const DEFAULT_FUEL_LIMIT: u64 = 30_000 * FUEL_PER_MS;

struct LoadedPlugin {
    plugin: Plugin,
}

/// A `WasmRuntime` backed by Extism (Wasmtime) for loading real .wasm extensions.
///
/// All extensions loaded by a given runtime share the same `HostContext`,
/// meaning they write diagnostics to the same collector, see the same graph, etc.
///
/// Every plugin is instantiated with a deterministic fuel budget
/// ([`DEFAULT_FUEL_LIMIT`], overridable per load) enforced by the engine —
/// a guest that loops forever traps instead of hanging the host (C7-10).
pub struct ExtismRuntime {
    plugins: Mutex<HashMap<String, LoadedPlugin>>,
    compile_cache_dir: Option<PathBuf>,
    host_context: HostContext,
}

impl ExtismRuntime {
    pub fn new() -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            compile_cache_dir: None,
            host_context: HostContext::default(),
        }
    }

    pub fn with_host_context(ctx: HostContext) -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            compile_cache_dir: None,
            host_context: ctx,
        }
    }

    /// Enable Wasmtime's on-disk compilation cache under `dir`.
    ///
    /// Compiled native code is cached across processes (keyed by module bytes
    /// and engine version by Wasmtime itself), cutting the ~80–110 ms
    /// Cranelift compile per blob on warm invocations. Wasmtime's cache is
    /// configured through a TOML file, so this writes a minimal config into
    /// `dir` and points the engine at it. Callers should pass a per-user
    /// cache directory; the runtime falls back to no cache when the
    /// directory or config file cannot be created.
    pub fn with_compile_cache(mut self, dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!(
                "warning: wasm compile cache disabled ({}: {})",
                dir.display(),
                e
            );
            return self;
        }
        let config_path = dir.join("config.toml");
        let config = format!("[cache]\ndirectory = \"{}\"\n", dir.to_string_lossy());
        if let Err(e) = std::fs::write(&config_path, config) {
            eprintln!(
                "warning: wasm compile cache disabled ({}: {})",
                config_path.display(),
                e
            );
            return self;
        }
        self.compile_cache_dir = Some(config_path);
        self
    }

    /// Load a Wasm module under an explicit extension name (instead of deriving from filename).
    pub fn load_module_as(
        &self,
        name: &str,
        wasm_path: &Path,
        _aot_cache_path: Option<&Path>,
    ) -> Result<(), String> {
        let wasm_bytes = self.read_and_validate(wasm_path)?;
        self.instantiate(name, &wasm_bytes, DEFAULT_FUEL_LIMIT)
    }

    /// Load a Wasm module from raw bytes (for embedded/bundled extensions).
    pub fn load_module_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        self.load_module_bytes_with_limits(name, wasm_bytes, DEFAULT_FUEL_LIMIT)
    }

    /// Load a Wasm module from raw bytes with an explicit fuel budget.
    pub fn load_module_bytes_with_limits(
        &self,
        name: &str,
        wasm_bytes: &[u8],
        fuel: u64,
    ) -> Result<(), String> {
        self.instantiate(name, wasm_bytes, fuel)
    }

    fn read_and_validate(&self, wasm_path: &Path) -> Result<Vec<u8>, String> {
        if !wasm_path.exists() {
            return Err(format!("Wasm file not found: {}", wasm_path.display()));
        }

        let wasm_bytes = std::fs::read(wasm_path)
            .map_err(|e| format!("Failed to read Wasm file {}: {}", wasm_path.display(), e))?;

        if wasm_bytes.len() < 8 || &wasm_bytes[0..4] != b"\x00asm" {
            return Err(format!(
                "Invalid Wasm binary at {}: missing magic bytes",
                wasm_path.display()
            ));
        }

        Ok(wasm_bytes)
    }

    fn instantiate(&self, name: &str, wasm_bytes: &[u8], fuel: u64) -> Result<(), String> {
        let functions = host_context::build_host_functions(self.host_context.clone());
        let manifest = Manifest::new([Wasm::data(wasm_bytes.to_vec())]);
        let mut builder = PluginBuilder::new(manifest)
            .with_wasi(true)
            .with_functions(functions)
            .with_fuel_limit(fuel);
        if let Some(dir) = &self.compile_cache_dir {
            builder = builder.with_cache_config(dir);
        }
        let plugin = builder
            .build()
            .map_err(|e| format!("Failed to instantiate Wasm plugin {}: {}", name, e))?;

        let mut plugins = self.plugins.lock().map_err(|e| e.to_string())?;
        plugins.insert(name.to_string(), LoadedPlugin { plugin });
        Ok(())
    }
}

impl Default for ExtismRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime for ExtismRuntime {
    fn load_module(&self, wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
        let wasm_bytes = self.read_and_validate(wasm_path)?;
        let extension_name = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.instantiate(&extension_name, &wasm_bytes, DEFAULT_FUEL_LIMIT)
    }

    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        let mut plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(e) => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "lock_poisoned".to_string(),
                    message: e.to_string(),
                    export_name: export_name.to_string(),
                });
            }
        };

        let loaded = match plugins.get_mut(extension_name) {
            Some(p) => p,
            None => {
                return WasmCallResult::Trap(WasmTrapInfo {
                    kind: "extension_not_found".to_string(),
                    message: format!("Extension '{}' not loaded", extension_name),
                    export_name: export_name.to_string(),
                });
            }
        };

        match loaded.plugin.call::<&[u8], Vec<u8>>(export_name, input) {
            Ok(output) => WasmCallResult::Ok(output),
            Err(e) => WasmCallResult::Trap(WasmTrapInfo {
                kind: "call_failed".to_string(),
                message: e.to_string(),
                export_name: export_name.to_string(),
            }),
        }
    }

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
        // The byte-copy ".aot" cache never fed compilation (audit C7-02);
        // warm-compile reuse is Wasmtime's on-disk compilation cache, enabled
        // via [`ExtismRuntime::with_compile_cache`], not this check.
        false
    }
}
