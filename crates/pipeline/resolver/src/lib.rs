//! Import resolution over a compile's sources (ADR 0055): an import path
//! is read from the spec root and names a source by its key (its path from the project root, ADR 0242). Nothing here reads, lists or stats a file.

mod resolve;

pub use resolve::{import_target, resolve_imports};
