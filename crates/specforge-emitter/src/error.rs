/// Structured error type for emitter operations.
///
/// Replaces raw `String` errors with categorized variants so callers
/// can match on error kind without parsing human-readable messages.
#[derive(Debug, Clone)]
pub enum EmitterError {
    /// The requested entity was not found in the graph.
    EntityNotFound(String),
    /// Serialization of graph data failed.
    SerializationError(String),
    /// An invalid scope or filter was provided.
    InvalidScope(String),
    /// Catch-all for errors that don't fit other categories.
    Other(String),
}

impl EmitterError {
    /// The process exit code a command reports when it fails with this
    /// error. Every emitter error is a failed request: 1.
    pub fn exit_code(&self) -> i32 {
        1
    }
}

impl std::fmt::Display for EmitterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmitterError::EntityNotFound(msg) => write!(f, "{}", msg),
            EmitterError::SerializationError(msg) => write!(f, "{}", msg),
            EmitterError::InvalidScope(msg) => write!(f, "{}", msg),
            EmitterError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for EmitterError {}
