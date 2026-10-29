//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "blob_size.rs"]
mod blob_size;
#[path = "bridge_wit.rs"]
mod bridge_wit;
#[path = "builtin_codes.rs"]
mod builtin_codes;
#[path = "builtins.rs"]
mod builtins;
#[path = "builtins_table.rs"]
mod builtins_table;
#[path = "compile_cache.rs"]
mod compile_cache;
#[path = "declarations.rs"]
mod declarations;
#[path = "epoch_deadline.rs"]
mod epoch_deadline;
#[path = "greet_sdk.rs"]
mod greet_sdk;
#[path = "native_tier_gate.rs"]
mod native_tier_gate;
#[path = "product_blob.rs"]
mod product_blob;
#[path = "runtime.rs"]
mod runtime;
#[path = "sandbox_limits.rs"]
mod sandbox_limits;
#[path = "sandbox_probe.rs"]
mod sandbox_probe;
// Shared by sandbox_limits and sandbox_probe.
#[allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]
mod probe_support;
