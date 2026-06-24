use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use extism::{Manifest, Plugin, PluginBuilder, Wasm};
use specforge_wasm::runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};

use crate::host_context::{self, HostContext};

struct LoadedPlugin {
    plugin: Plugin,
}

/// A `WasmRuntime` backed by Extism (Wasmtime) for loading real .wasm extensions.
///
/// All extensions loaded by a given runtime share the same `HostContext`,
/// meaning they write diagnostics to the same collector, see the same graph, etc.
pub struct ExtismRuntime {
    plugins: Mutex<HashMap<String, LoadedPlugin>>,
    aot_cache_dir: Option<PathBuf>,
    host_context: HostContext,
}

impl ExtismRuntime {
    pub fn new() -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            aot_cache_dir: None,
            host_context: HostContext::default(),
        }
    }

    pub fn with_host_context(ctx: HostContext) -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            aot_cache_dir: None,
            host_context: ctx,
        }
    }

    pub fn with_aot_cache_dir(mut self, dir: PathBuf) -> Self {
        self.aot_cache_dir = Some(dir);
        self
    }

    /// Load a Wasm module under an explicit extension name (instead of deriving from filename).
    pub fn load_module_as(
        &self,
        name: &str,
        wasm_path: &Path,
        aot_cache_path: Option<&Path>,
    ) -> Result<(), String> {
        let wasm_bytes = self.read_and_validate(wasm_path)?;
        self.instantiate(name, &wasm_bytes, aot_cache_path)
    }

    /// Load a Wasm module from raw bytes (for embedded/bundled extensions).
    pub fn load_module_bytes(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        self.instantiate(name, wasm_bytes, None)
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

    fn instantiate(
        &self,
        name: &str,
        wasm_bytes: &[u8],
        _aot_cache_path: Option<&Path>,
    ) -> Result<(), String> {
        let functions = host_context::build_host_functions(self.host_context.clone());
        let manifest = Manifest::new([Wasm::data(wasm_bytes.to_vec())]);
        let plugin = PluginBuilder::new(manifest)
            .with_wasi(true)
            .with_functions(functions)
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
    fn load_module(&self, wasm_path: &Path, aot_cache_path: Option<&Path>) -> Result<(), String> {
        let wasm_bytes = self.read_and_validate(wasm_path)?;
        let extension_name = wasm_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        self.instantiate(&extension_name, &wasm_bytes, aot_cache_path)
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

    fn has_cached_module(&self, wasm_hash: &str) -> bool {
        match &self.aot_cache_dir {
            Some(dir) => dir.join(format!("{}.aot", wasm_hash)).exists(),
            None => false,
        }
    }
}
