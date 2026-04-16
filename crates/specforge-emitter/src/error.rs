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
