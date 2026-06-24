mod bridge;
mod detect;
mod error;
mod host;
mod types;

pub use bridge::{populate_from_protocol, protocol_extension_to_manifest, protocol_surfaces_to_manifest};
pub use detect::{ExtensionMode, detect_extension_mode, find_wasm_binary};
pub use error::ProtocolError;
pub use host::{load_protocol_extension, ExtensionDescriptions, ProtocolExtension, ProtocolHost};
pub use types::*;

/// Protocol version for the extension wire format (semver).
/// Extensions with the same major version are considered compatible.
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// All supported describe categories that the host can request.
pub const SUPPORTED_CATEGORIES: &[&str] = &[
    "entities",
    "edges",
    "fields",
    "shared_fields",
    "enhancements",
    "validation_rules",
    "surfaces",
    "grammars",
    "body_parsers",
    "collectors",
    "passes",
    "feature_flags",
    "analyzers",
];
