use std::path::Path;

/// Result of calling a Wasm export function.
#[derive(Debug, Clone)]
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
