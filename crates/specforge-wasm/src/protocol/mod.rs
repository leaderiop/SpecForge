mod bridge;
mod detect;
mod error;
mod host;
mod types;

pub use bridge::{
    populate_from_protocol, protocol_extension_to_manifest, protocol_surfaces_to_manifest,
};
pub use detect::{ExtensionMode, detect_extension_mode, find_wasm_binary};
pub use error::ProtocolError;
pub use host::{ExtensionDescriptions, ProtocolExtension, ProtocolHost, load_protocol_extension};
pub use types::*;

pub use specforge_protocol_types::{PROTOCOL_VERSION, SUPPORTED_CATEGORIES};
