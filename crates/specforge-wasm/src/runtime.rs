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

/// Testable abstraction over a Wasm runtime (wasmtime component engine).
///
/// Compilation caching is a host-engine concern (wasmtime's native on-disk
/// cache, configured at `ComponentRuntime` construction) — it is not part of
/// the load contract.
pub trait WasmRuntime: Send + Sync {
    /// Load a .wasm component binary into the runtime.
    fn load_module(&self, wasm_path: &Path) -> Result<(), String>;

    /// Call an export function on a loaded module.
    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult;

    /// Applies an extension's declared wall-clock budget (its handshake
    /// `sandbox_policy.max_execution_ms`) to its subsequent calls.
    /// Runtimes that cannot enforce wall-clock limits (mocks, test doubles)
    /// ignore this (audit C7-10).
    fn set_execution_deadline_ms(&self, _extension_name: &str, _max_execution_ms: u64) {}

    /// Load a .wasm component binary under `extension_name`, the name its
    /// exports are then called by. Runtimes that key modules by path
    /// (mocks) load it as [`WasmRuntime::load_module`] does.
    fn load_module_named(&self, _extension_name: &str, wasm_path: &Path) -> Result<(), String> {
        self.load_module(wasm_path)
    }

    /// Why `extension_name` failed to load when the runtime was built (a
    /// missing or tampered installed binary), so compile can report that
    /// diagnostic instead of a bare "not loaded".
    fn load_failure(&self, _extension_name: &str) -> Option<specforge_common::Diagnostic> {
        None
    }
}

/// A mock runtime for testing — records calls and returns configured results.
#[cfg(test)]
pub struct MockRuntime {
    pub load_results: std::collections::HashMap<String, Result<(), String>>,
    pub call_results: std::collections::HashMap<String, WasmCallResult>,
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
        }
    }

    pub fn with_load_ok(mut self, path: &str) -> Self {
        self.load_results.insert(path.to_string(), Ok(()));
        self
    }

    pub fn with_load_err(mut self, path: &str, err: &str) -> Self {
        self.load_results
            .insert(path.to_string(), Err(err.to_string()));
        self
    }

    pub fn with_call_ok(mut self, export: &str, output: Vec<u8>) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Ok(output));
        self
    }

    pub fn with_call_trap(mut self, export: &str, trap: WasmTrapInfo) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Trap(trap));
        self
    }
}

#[cfg(test)]
impl WasmRuntime for MockRuntime {
    fn load_module(&self, wasm_path: &Path) -> Result<(), String> {
        let key = wasm_path.to_string_lossy().to_string();
        self.load_results.get(&key).cloned().unwrap_or(Ok(()))
    }

    fn call_export(
        &self,
        _extension_name: &str,
        export_name: &str,
        _input: &[u8],
    ) -> WasmCallResult {
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or(WasmCallResult::Ok(vec![]))
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
