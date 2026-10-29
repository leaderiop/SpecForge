//! Every integration test of the crate in one binary: each `tests/*.rs` linked wasmtime and the crate on its
//! own, and the links were most of a CI build. The files stay where they are, so their paths do not change.

#[path = "diagnostic.rs"]
mod diagnostic;
#[path = "json_span.rs"]
mod json_span;
#[path = "present.rs"]
mod present;
#[path = "project_files.rs"]
mod project_files;
#[path = "project_root.rs"]
mod project_root;
#[path = "project_settings.rs"]
mod project_settings;
#[path = "shape.rs"]
mod shape;
#[path = "titled_list.rs"]
mod titled_list;
