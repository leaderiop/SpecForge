//! Judged rules over a project (ADR 0290): the digests a verdict is fresh against, the state of each judged rule on
//! each entity, and the inputs query. The declaration is the registry's (`specforge_registry::judged`).

mod digest;

pub use digest::{entity_digest, file_digest};
