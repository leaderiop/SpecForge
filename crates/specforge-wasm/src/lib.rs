#![allow(clippy::result_large_err)]

pub mod calls;
mod install;
mod integrity;
mod lifecycle;
mod lock_file;
pub mod protocol;
pub mod runtime;
pub mod sandbox;
mod specifier;
mod surface;
#[cfg(feature = "testing")]
pub mod testing;
mod toposort;
mod uninstall;

pub use calls::{CallError, CallFailure, Encoded, ExtensionCalls, Operation, pass_diagnostics};
pub use install::{InstallResult, install_extension, installed_wasm_path};
pub use integrity::hex_sha256;
pub use lifecycle::load_wasm_module;
pub use lock_file::{
    DoctorStatus, LockFile, LockFileEntry, collect_peer_requirers, read_lock_file,
    run_doctor_check, write_lock_file,
};
pub use runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};
pub use sandbox::default_sandbox_policy;
pub use specifier::{ExtensionSpecifier, parse_extension_specifier};
pub use surface::{AutoPromotedMcpTool, auto_promote_commands_to_mcp_tools};
pub use toposort::topological_sort_extensions;
pub use uninstall::uninstall_extension;
