//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "queries.rs"]
mod queries;
#[path = "recovery.rs"]
mod recovery;
#[path = "vocabulary.rs"]
mod vocabulary;
