use std::fmt;

/// Errors that can occur during the extension protocol lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Host and extension protocol versions are incompatible.
    IncompatibleVersion {
        host_version: String,
        extension_version: String,
    },
    /// The __handshake export failed or returned invalid data.
    HandshakeFailed(String),
    /// The __describe export failed or returned invalid data.
    DescribeFailed { category: String, reason: String },
    /// JSON deserialization of a protocol message failed.
    DeserializationError(String),
    /// The host requested an unsupported describe category.
    UnsupportedCategory(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleVersion {
                host_version,
                extension_version,
            } => write!(
                f,
                "protocol version mismatch: host={}, extension={}",
                host_version, extension_version
            ),
            Self::HandshakeFailed(reason) => write!(f, "handshake failed: {}", reason),
            Self::DescribeFailed { category, reason } => {
                write!(f, "describe '{}' failed: {}", category, reason)
            }
            Self::DeserializationError(msg) => write!(f, "deserialization error: {}", msg),
            Self::UnsupportedCategory(cat) => write!(f, "unsupported category: {}", cat),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<serde_json::Error> for ProtocolError {
    fn from(e: serde_json::Error) -> Self {
        Self::DeserializationError(e.to_string())
    }
}
