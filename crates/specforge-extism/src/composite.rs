use std::path::Path;

use specforge_wasm::runtime::{WasmCallResult, WasmRuntime};
use specforge_wasm::BuiltinRuntime;

use crate::ExtismRuntime;

/// A runtime that composes built-in native extensions with Extism-loaded Wasm extensions.
///
/// Dispatch logic:
/// - If the extension name is registered in the `BuiltinRuntime`, delegate there.
/// - Otherwise, delegate to the `ExtismRuntime` (for `.wasm`-loaded extensions).
pub struct CompositeRuntime {
    builtin: BuiltinRuntime,
    extism: ExtismRuntime,
}

impl CompositeRuntime {
    pub fn new(builtin: BuiltinRuntime, extism: ExtismRuntime) -> Self {
        Self { builtin, extism }
    }

    /// Load a Wasm extension file under a given name via Extism.
    pub fn load_wasm_extension(&self, name: &str, wasm_path: &Path) -> Result<(), String> {
        self.extism.load_module_as(name, wasm_path, None)
    }
}

impl WasmRuntime for CompositeRuntime {
    fn load_module(&self, wasm_path: &Path, aot_cache_path: Option<&Path>) -> Result<(), String> {
        self.extism.load_module(wasm_path, aot_cache_path)
    }

    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult {
        // Try builtin first
        let result = self.builtin.call_export(extension_name, export_name, input);
        if matches!(&result, WasmCallResult::Trap(t) if t.kind == "not_found") {
            // Not a builtin — try Extism
            return self.extism.call_export(extension_name, export_name, input);
        }
        result
    }

    fn has_cached_module(&self, wasm_hash: &str) -> bool {
        self.builtin.has_cached_module(wasm_hash) || self.extism.has_cached_module(wasm_hash)
    }
}
