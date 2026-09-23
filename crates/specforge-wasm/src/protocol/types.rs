//! Wire types moved to the `specforge-protocol-types` crate so the
//! plugin-side SDK can share the exact same definitions (single source of
//! truth). Re-exported here to keep all host call sites unchanged.

pub use specforge_protocol_types::*;
