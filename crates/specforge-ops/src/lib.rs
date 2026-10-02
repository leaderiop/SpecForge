//! User-level operations shared by the CLI and the MCP server.
//!
//! Each surface used to orchestrate the backend crates on its own, so fixes
//! landed on one side only. An operation here takes a typed request and
//! returns a typed outcome or an [`OpError`]; the CLI and MCP only translate
//! arguments in and present the result.
//!
//! Operations never write to stdout: the MCP server owns it (it is the
//! JSON-RPC stream). The crate denies `clippy::print_stdout`.

pub mod analyze;
pub mod collect;
pub mod config;
pub mod doctor;
pub mod export;
pub mod extension;
pub mod format;
pub mod infer;
pub mod init;
pub mod migrate;
pub mod prove;
pub mod registry;
pub mod rename;

use std::borrow::Cow;

/// Why an operation failed. `code` is data for the surface to present (a
/// CLI `error[CODE]`, an MCP error's `data.code`), never folded into
/// `message`.
#[derive(Debug, Clone, PartialEq)]
pub struct OpError {
    pub code: Cow<'static, str>,
    pub message: String,
    pub suggestion: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl OpError {
    pub fn new(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            suggestion: None,
            data: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl std::fmt::Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OpError {}

/// A diagnostic's code stays data; its message and suggestion carry over.
impl From<specforge_common::Diagnostic> for OpError {
    fn from(diagnostic: specforge_common::Diagnostic) -> Self {
        Self {
            code: Cow::Owned(diagnostic.code),
            message: diagnostic.message,
            suggestion: diagnostic.suggestion,
            data: None,
        }
    }
}
