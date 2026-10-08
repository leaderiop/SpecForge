#![allow(clippy::result_large_err)]

pub mod calls;
pub mod protocol;
pub mod runtime;
pub mod sandbox;
#[cfg(feature = "testing")]
pub mod testing;
mod toposort;

pub use calls::{
    CallError, CallFailure, Encoded, ExtensionCalls, Handshake, Operation, pass_diagnostics,
};
pub use runtime::{WasmCallResult, WasmRuntime, WasmTrapInfo};
pub use sandbox::{Limits, Sandbox};
pub use toposort::topological_sort_extensions;
