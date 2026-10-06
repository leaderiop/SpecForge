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

/// What kind of failure an operation reports: the closed set every surface
/// maps its own codes from (MCP's `ErrorCode`), decided where the failure is
/// raised (ADR 0024 D15). [`OpError::code`] stays what the CLI prints
/// (`error[E027]`, `error[unknown_format]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpErrorKind {
    /// The request is wrong: an unknown name or format, a malformed
    /// specifier, a budget too small for the export.
    InvalidInput,
    /// An entity the request names is not in the graph.
    EntityNotFound,
    /// A file or directory the request names, or the project's config, is
    /// missing.
    FileNotFound,
    /// The extension the request names is not installed, built in, or
    /// published.
    ExtensionNotFound,
    /// The request contradicts what is there: a name taken, a project in
    /// place, an unsatisfiable peer, a version diamond.
    Conflict,
    /// A file the operation reads is not what it should hold: the config,
    /// the lock, a manifest, a test report, a signature's metadata.
    SchemaMismatch,
    /// Something the operation needs is not set up: no project, no
    /// registry, no collector, no lock file.
    PreconditionFailed,
    /// The operation was not allowed: an unapproved command, a write the OS
    /// refused.
    PermissionDenied,
    /// A registry or a command did not answer in time.
    Timeout,
    /// The operation failed on its own side: a write that failed for
    /// another reason, an extension that trapped, a serialization.
    Internal,
}

impl OpErrorKind {
    /// The kind of a failure reported as diagnostic `code`: the one table
    /// (E003 entity, E019/E054/E062/E064 and a registry's R-RES-004 input,
    /// R-RES-001 extension, E027 and R-RES-006 conflict, E045/E067 and a
    /// registry's R-TRUST-004/R-OPS-004 schema, E058/E063 precondition,
    /// E059 permission, R004 timeout, else internal). MCP's
    /// `ErrorCode::for_diagnostic` reads it.
    pub fn of_diagnostic(code: &str) -> Self {
        match code {
            "E003" => Self::EntityNotFound,
            "E019" | "E054" | "E062" | "E064" | "R-RES-004" => Self::InvalidInput,
            "R-RES-001" => Self::ExtensionNotFound,
            "E027" | "R-RES-006" => Self::Conflict,
            "E045" | "E067" | "R-TRUST-004" | "R-OPS-004" => Self::SchemaMismatch,
            "E058" | "E063" => Self::PreconditionFailed,
            "E059" => Self::PermissionDenied,
            "R004" => Self::Timeout,
            _ => Self::Internal,
        }
    }

    /// The kind of a failed file operation: `PermissionDenied` when the OS
    /// refused, `FileNotFound` for `NotFound`, else `Internal`.
    pub fn of_io(error: &std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::NotFound => Self::FileNotFound,
            _ => Self::Internal,
        }
    }
}

/// Why an operation failed. `code` is data for the surface to present (a
/// CLI `error[CODE]`, an MCP error's `data.code`), never folded into
/// `message`; `kind` is what the failure is, which each surface maps to its
/// own codes.
#[derive(Debug, Clone, PartialEq)]
pub struct OpError {
    pub kind: OpErrorKind,
    pub code: Cow<'static, str>,
    pub message: String,
    pub suggestion: Option<String>,
    /// What else the failure says, as the operation's own JSON. Boxed, like
    /// `entity`, so the error stays small enough to be returned whole
    /// (`clippy.toml`'s `large-error-threshold`).
    pub data: Option<Box<serde_json::Value>>,
    /// The entity the failure is about (`EntityNotFound`, a rename's
    /// `Conflict`).
    pub entity: Option<Box<str>>,
    /// The files the operation changed on disk before it failed and did
    /// not restore (`extension::add` when the lock or config write fails
    /// after the module was placed): a surface reports them as written.
    /// An operation that rolls back what it wrote leaves this empty.
    pub writes: Writes,
}

impl OpError {
    pub fn new(
        kind: OpErrorKind,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            suggestion: None,
            data: None,
            entity: None,
            writes: Writes::none(),
        }
    }

    /// A failure reported as diagnostic `code`, its kind
    /// [`OpErrorKind::of_diagnostic`]'s.
    pub fn diagnostic(code: impl Into<Cow<'static, str>>, message: impl Into<String>) -> Self {
        let code = code.into();
        Self::new(OpErrorKind::of_diagnostic(&code), code, message)
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

    /// The same error, about `entity`.
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entity = Some(entity.into().into_boxed_str());
        self
    }

    /// The same error, carrying `data`.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(Box::new(data));
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
        let mut error = Self::diagnostic(Cow::Owned(diagnostic.code), diagnostic.message);
        error.suggestion = diagnostic.suggestion;
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_kinds_of_diagnostic_codes() {
        for (code, kind) in [
            ("E003", OpErrorKind::EntityNotFound),
            ("E019", OpErrorKind::InvalidInput),
            ("E027", OpErrorKind::Conflict),
            ("E045", OpErrorKind::SchemaMismatch),
            ("E058", OpErrorKind::PreconditionFailed),
            ("E059", OpErrorKind::PermissionDenied),
            ("E062", OpErrorKind::InvalidInput),
            ("E063", OpErrorKind::PreconditionFailed),
            ("E067", OpErrorKind::SchemaMismatch),
            ("R-RES-001", OpErrorKind::ExtensionNotFound),
            ("R-RES-004", OpErrorKind::InvalidInput),
            ("R-RES-006", OpErrorKind::Conflict),
            ("R-TRUST-004", OpErrorKind::SchemaMismatch),
            ("R-OPS-004", OpErrorKind::SchemaMismatch),
            ("R004", OpErrorKind::Timeout),
            ("E999", OpErrorKind::Internal),
            ("not_a_code", OpErrorKind::Internal),
        ] {
            assert_eq!(OpErrorKind::of_diagnostic(code), kind, "{code}");
            assert_eq!(OpError::diagnostic(code, "x").kind, kind, "{code}");
        }
    }

    #[test]
    fn an_error_stays_small_enough_to_return_whole() {
        // clippy.toml's `large-error-threshold` is 129 bytes.
        assert!(std::mem::size_of::<OpError>() <= 129);
    }

    #[test]
    fn failure_kinds_of_io_errors() {
        use std::io::{Error, ErrorKind};
        assert_eq!(
            OpErrorKind::of_io(&Error::from(ErrorKind::PermissionDenied)),
            OpErrorKind::PermissionDenied
        );
        assert_eq!(
            OpErrorKind::of_io(&Error::from(ErrorKind::NotFound)),
            OpErrorKind::FileNotFound
        );
        assert_eq!(
            OpErrorKind::of_io(&Error::from(ErrorKind::NotADirectory)),
            OpErrorKind::Internal
        );
    }

    #[test]
    fn an_error_carries_the_writes_it_left() {
        let error = OpError::diagnostic("E032", "failed to write specforge.lock");
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
