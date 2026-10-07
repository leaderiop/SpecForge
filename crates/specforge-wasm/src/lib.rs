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
#[cfg(feature = "testing")]
pub mod testing;
mod toposort;
mod uninstall;

pub use calls::{
    CallError, CallFailure, Encoded, ExtensionCalls, Handshake, Operation, pass_diagnostics,
};
pub use install::{InstallResult, install_extension, installed_wasm_path};
pub use integrity::hex_sha256;
pub use lifecycle::load_wasm_module;
pub use lock_file::{
    DoctorStatus, LOCK_FILE, LockFile, LockFileEntry, LockState, collect_peer_requirers, lock_path,
    read_lock_file, run_doctor_check, write_lock_file,
};
pub use runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};
pub use sandbox::{Limits, Sandbox};
pub use specifier::{ExtensionSpecifier, parse_extension_specifier};
pub use toposort::topological_sort_extensions;
pub use uninstall::uninstall_extension;
