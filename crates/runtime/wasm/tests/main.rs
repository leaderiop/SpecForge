#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

mod calls;
mod extension_text;
mod handshake_conformance;
mod judged_wire;
mod load_declaration;
mod plan_wire;
mod prompt_wire;
mod protocol_host;
mod protocol_types;
mod read_only;
mod runtime_contract;
mod sdk_vocabulary;
