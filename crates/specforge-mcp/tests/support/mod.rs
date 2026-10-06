//! What every MCP test serves: a project on disk whose extensions are
//! declared with the SDK builders and run in process (`InProcessRuntime`,
//! the test adapter of the `WasmRuntime` port), loaded when the server
//! initializes over it — `ProjectSession::open_with_runtime`,
//! `Environment::load`, `build_registries`, a full compile — exactly as
//! `specforge mcp <root>` loads a project. No test writes a registry entry,
//! a graph or a diagnostic into the server's state: what a test serves is
//! what its sources and its extension declare.

pub mod extension;
mod legacy;
pub mod project;
pub mod rpc;

#[allow(
    unused_imports,
    reason = "the migrated test files import them from plan 10-T3 on"
)]
pub use extension::{EXT, TestExtension};
pub use legacy::{
    declare_headline_fields, declare_reference, obligate, obligations_rule, report, report_also,
    serve_in_memory_at, update_of,
};
#[allow(
    unused_imports,
    reason = "the migrated test files import them from plan 10-T3 on"
)]
pub use project::{Served, TestProject};
pub use rpc::*;
