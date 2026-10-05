mod bridge;
mod detect;
mod error;
mod host;
mod load;
mod types;

pub use bridge::declaration_to_manifest;
pub use detect::{ExtensionMode, detect_extension_mode, find_wasm_binary};
pub use error::ProtocolError;
pub use load::{Loaded, load_declaration};
pub use types::*;

pub use specforge_protocol_types::{PROTOCOL_VERSION, SUPPORTED_CATEGORIES};
