//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "codex.rs"]
mod codex;
#[path = "golden.rs"]
mod golden;
#[path = "hook_protocol.rs"]
mod hook_protocol;
// Shared by codex and golden.
#[allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]
mod support;
