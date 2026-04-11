use std::path::Path;

/// Result of calling a Wasm export function.
#[derive(Debug)]
pub enum WasmCallResult {
    Ok(Vec<u8>),
    Trap(WasmTrapInfo),
}

/// Details of a Wasm trap.
#[derive(Debug, Clone)]
pub struct WasmTrapInfo {
    pub kind: String,
    pub message: String,
    pub export_name: String,
}

/// Extension lifecycle state tracked by the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionLifecycleState {
    Discovered,
    Loading,
    Initialized,
    Validating,
    Exporting,
    Unloaded,
    Failed,
}

/// A loaded Wasm module — opaque handle returned by the runtime.
#[derive(Debug)]
pub struct LoadedModule {
    pub extension_name: String,
    pub wasm_hash: String,
    pub state: ExtensionLifecycleState,
}

/// Testable abstraction over a Wasm runtime (Extism in production).
pub trait WasmRuntime: Send + Sync {
    /// Load a .wasm binary (or AOT-cached artifact) into the runtime.
    fn load_module(&self, wasm_path: &Path, aot_cache_path: Option<&Path>) -> Result<(), String>;

    /// Call an export function on a loaded module.
    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult;

    /// Check if an AOT-cached artifact exists for the given hash.
    fn has_cached_module(&self, wasm_hash: &str) -> bool;
}

/// A mock runtime for testing — records calls and returns configured results.
#[cfg(test)]
pub struct MockRuntime {
    pub load_results: std::collections::HashMap<String, Result<(), String>>,
    pub call_results: std::collections::HashMap<String, WasmCallResult>,
    pub cached_modules: std::collections::HashSet<String>,
}

#[cfg(test)]
impl Default for MockRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl MockRuntime {
    pub fn new() -> Self {
        Self {
            load_results: std::collections::HashMap::new(),
            call_results: std::collections::HashMap::new(),
            cached_modules: std::collections::HashSet::new(),
        }
    }

    pub fn with_cached(mut self, hash: &str) -> Self {
        self.cached_modules.insert(hash.to_string());
        self
    }

    pub fn with_load_ok(mut self, path: &str) -> Self {
        self.load_results.insert(path.to_string(), Ok(()));
        self
    }

    pub fn with_load_err(mut self, path: &str, err: &str) -> Self {
        self.load_results.insert(path.to_string(), Err(err.to_string()));
        self
    }

    pub fn with_call_ok(mut self, export: &str, output: Vec<u8>) -> Self {
        self.call_results.insert(export.to_string(), WasmCallResult::Ok(output));
        self
    }

    pub fn with_call_trap(mut self, export: &str, trap: WasmTrapInfo) -> Self {
        self.call_results.insert(export.to_string(), WasmCallResult::Trap(trap));
        self
    }
}

#[cfg(test)]
impl WasmRuntime for MockRuntime {
    fn load_module(&self, wasm_path: &Path, _aot_cache_path: Option<&Path>) -> Result<(), String> {
        let key = wasm_path.to_string_lossy().to_string();
        self.load_results
            .get(&key)
            .cloned()
            .unwrap_or(Ok(()))
    }

    fn call_export(&self, _extension_name: &str, export_name: &str, _input: &[u8]) -> WasmCallResult {
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or(WasmCallResult::Ok(vec![]))
    }

    fn has_cached_module(&self, wasm_hash: &str) -> bool {
        self.cached_modules.contains(wasm_hash)
    }
}

// Allow cloning WasmCallResult for mock
impl Clone for WasmCallResult {
    fn clone(&self) -> Self {
        match self {
            WasmCallResult::Ok(data) => WasmCallResult::Ok(data.clone()),
            WasmCallResult::Trap(info) => WasmCallResult::Trap(info.clone()),
        }
    }
}
