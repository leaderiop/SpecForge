//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "cli.rs"]
mod cli;
#[path = "local_registry.rs"]
mod local_registry;
#[path = "publish_signatures.rs"]
mod publish_signatures;
#[path = "scope_ownership.rs"]
mod scope_ownership;
#[path = "server_contracts.rs"]
mod server_contracts;
#[path = "stored_versions.rs"]
mod stored_versions;
