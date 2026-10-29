//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "compare.rs"]
mod compare;
#[path = "defaults.rs"]
mod defaults;
#[path = "header.rs"]
mod header;
#[path = "layout.rs"]
mod layout;
#[path = "line_breaks.rs"]
mod line_breaks;
#[path = "names.rs"]
mod names;
#[path = "reads.rs"]
mod reads;
#[path = "rollback.rs"]
mod rollback;
#[path = "titles.rs"]
mod titles;
