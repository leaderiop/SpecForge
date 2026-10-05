//! The extension protocol on the host side: the wire types (shared with
//! the SDK through `specforge-protocol-types`) and the one loader that
//! reads an extension's declaration ([`load_declaration`], ADR 0012).

mod host;
mod load;

pub use load::{Loaded, load_declaration};

pub use specforge_protocol_types::*;
