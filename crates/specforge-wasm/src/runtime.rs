use std::path::Path;

use crate::sandbox::Limits;

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
    /// Compile the component `bytes` and register it under `name`,
    /// replacing what was loaded under it. The bytes are the ones the
    /// caller checked: the runtime reads nothing else.
    fn load(&self, name: &str, bytes: &[u8]) -> Result<(), String>;

    /// Register what is loaded as `from` under `to` instead, without
    /// compiling it again. False when `from` is not loaded.
    fn rename(&self, from: &str, to: &str) -> bool;

    /// Drop what is loaded as `name`. False when nothing was.
    fn unload(&self, name: &str) -> bool;

    /// Load a .wasm component binary into the runtime.
    fn load_module(&self, wasm_path: &Path) -> Result<(), String>;

    /// Call an export function on a loaded module.
    fn call_export(&self, extension_name: &str, export_name: &str, input: &[u8]) -> WasmCallResult;

    /// Hold `extension_name`'s later calls to `limits` (its handshake's
    /// sandbox, ADR 0037): the component runtime enforces them, the
    /// in-process runtime records them (its guest runs in the host process
    /// and is held to nothing). An extension that is not loaded is ignored.
    fn apply_limits(&self, extension_name: &str, limits: Limits);

    /// Load a .wasm component binary under `extension_name`, the name its
    /// exports are then called by. Runtimes that key modules by path
    /// (mocks) load it as [`WasmRuntime::load_module`] does.
    fn load_module_named(&self, _extension_name: &str, wasm_path: &Path) -> Result<(), String> {
        self.load_module(wasm_path)
    }

    /// Why `extension_name` failed to load when the runtime was built (a
    /// missing or tampered installed binary), so compile can report that
    /// diagnostic instead of a bare "not loaded". For a `.wasm` file entry
    /// of `specforge.json` the key is the entry itself (trimmed), since
    /// what it would have declared is unknown.
    fn load_failure(&self, _extension_name: &str) -> Option<specforge_common::Diagnostic> {
        None
    }

    /// The extension the `.wasm` file entry `entry` of `specforge.json`
    /// (trimmed; see [`specforge_common::ExtensionEntry::File`]) was
    /// loaded as when the runtime was built: the name its component
    /// declares, which its exports are called by. `None` when the runtime
    /// did not load that entry.
    fn file_entry_extension(&self, _entry: &str) -> Option<String> {
        None
    }
}
