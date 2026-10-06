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
pub mod builtin_passes;
pub mod check;
pub mod collect;
pub mod command;
pub mod config;
pub mod coverage;
pub mod doctor;
pub mod export;
pub mod extension;
pub mod format;
pub mod infer;
pub mod init;
pub mod inspect;
pub mod migrate;
pub mod model;
pub mod navigate;
pub mod options;
pub mod plan;
pub mod prove;
pub mod publish;
pub mod registry;
pub mod rename;
pub mod scan;
pub mod schema;
pub mod schema_cache;
pub mod stats;
pub mod trace;
pub mod view;
mod writes;

pub use writes::Writes;

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
    /// The files the operation changed on disk before it failed and did
    /// not restore (`extension::add` when the lock or config write fails
    /// after the module was placed): a surface reports them as written.
    /// An operation that rolls back what it wrote leaves this empty.
    pub writes: Writes,
}

impl OpError {
    pub fn new(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            suggestion: None,
            data: None,
            writes: Writes::none(),
        }
    }

    /// The same error, having left `writes` changed on disk.
    pub fn with_writes(mut self, writes: Writes) -> Self {
        self.writes = writes;
        self
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
            writes: Writes::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_carries_the_writes_it_left() {
        let error = OpError::new("E032", "failed to write specforge.lock");
        assert!(error.writes.is_empty());
        let module = "/p/.specforge/extensions/@sdk/greet/extension.wasm";
        let error = error.with_writes(Writes::from_iter([module]));
        let left: Vec<&std::path::Path> = error.writes.paths().collect();
        assert_eq!(left, [std::path::Path::new(module)]);
        // From a diagnostic: nothing written.
        let from: OpError = specforge_common::Diagnostic::error("E032", "x").into();
        assert!(from.writes.is_empty());
    }
}
