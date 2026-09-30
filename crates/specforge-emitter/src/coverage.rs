//! The host's coverage vocabulary: which kinds are testable, what an
//! entity's obligations are, and how the recorded test report is read.
//!
//! Every surface that asks "does this entity count toward coverage" or
//! "what does it promise to prove" (stats, plan validation, the context
//! exports, the MCP coverage, inspect, review and trace views) reads it
//! here, so they cannot disagree.

use crate::analyze::TestReport;
use crate::collect::REPORT_FILE;
use serde_json::Value;
use specforge_common::Diagnostic;
use specforge_graph::{FieldMap, FieldValue, Node};
use specforge_parser::VerifyStatement;
use specforge_registry::KindRegistry;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The kinds that count toward coverage: those an extension's manifest
/// declares `testable`. Nothing is testable by default, and accepting
/// `verify` statements (`supports_verify`) does not make a kind testable.
pub fn testable_kinds(reg: &KindRegistry) -> BTreeSet<&str> {
    reg.iter()
        .filter(|(_, kind)| kind.testable)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// An entity's obligations: its `verify` statements, in declaration order.
///
/// They are found wherever they sit among the entity's fields. A type may
/// declare a struct member named `verify` (`verify string @optional`); that
/// member is a field, not an obligation, and must not hide the statements,
/// which a first-match lookup of the `verify` key would do.
pub fn obligations(node: &Node) -> &[VerifyStatement] {
    obligations_in(&node.fields)
}

/// [`obligations`] over a bare field map.
pub fn obligations_in(fields: &FieldMap) -> &[VerifyStatement] {
    fields
        .entries()
        .iter()
        .find_map(|entry| match &entry.value {
            FieldValue::VerifyList(stmts) => Some(stmts.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

/// Why a test report could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// The report is there (or was named) but could not be read.
    Unreadable {
        path: PathBuf,
        detail: String,
        /// The named file does not exist.
        missing: bool,
    },
    /// The report does not parse as a `specforge-report.json`.
    Malformed { path: PathBuf, detail: String },
}

impl ReportError {
    pub fn path(&self) -> &Path {
        match self {
            ReportError::Unreadable { path, .. } | ReportError::Malformed { path, .. } => path,
        }
    }

    /// The error as a diagnostic (E045, an invalid test report).
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::error("E045", self.to_string()).with_suggestion(
            "run `specforge collect` again to rewrite the report, or fix or remove the file",
        )
    }
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::Unreadable { path, detail, .. } => {
                write!(f, "cannot read test results {}: {detail}", path.display())
            }
            ReportError::Malformed { path, detail } => write!(
                f,
                "invalid test results {}: {detail} (expected the RES-15 specforge-report.json shape)",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ReportError {}

/// The project's recorded test results (`specforge-report.json` at `root`,
/// written by `specforge collect`). No report means no recorded tests
/// (`Ok(None)`); a report that is there but unreadable or malformed is an
/// error, never read as empty, so coverage cannot silently drop.
pub fn read_report(root: &Path) -> Result<Option<TestReport>, ReportError> {
    let path = root.join(REPORT_FILE);
    if !path.exists() {
        return Ok(None);
    }
    read_report_file(&path).map(Some)
}

/// A test report at an explicit path (`--test-results`), which must exist.
pub fn read_report_file(path: &Path) -> Result<TestReport, ReportError> {
    let raw = std::fs::read_to_string(path).map_err(|e| ReportError::Unreadable {
        path: path.to_path_buf(),
        detail: e.to_string(),
        missing: e.kind() == std::io::ErrorKind::NotFound,
    })?;
    serde_json::from_str(&raw).map_err(|e| ReportError::Malformed {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })
}

/// The obligations as the exports write them (`[{kind, description}]`), or
/// `None` when the entity declares none.
pub(crate) fn obligations_json(node: &Node) -> Option<Value> {
    let stmts = obligations(node);
    (!stmts.is_empty()).then(|| {
        Value::Array(
            stmts
                .iter()
                .map(|s| serde_json::json!({"kind": s.kind, "description": s.description}))
                .collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::KindRegistryEntry;
    use specforge_test_macros::test as specforge_test;

    fn kind(name: &str, testable: bool, supports_verify: bool) -> KindRegistryEntry {
        KindRegistryEntry {
            kind_name: name.into(),
            description: None,
            source_extension: "@test/ext".into(),
            testable,
            singleton: false,
            supports_verify,
            allowed_verify_kinds: Vec::new(),
            has_body_parser: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
        }
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "no default testability assumed by core"
    )]
    fn no_kind_is_testable_unless_an_extension_says_so() {
        assert!(testable_kinds(&KindRegistry::new()).is_empty());

        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", false, false));
        assert!(testable_kinds(&reg).is_empty());
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "testable=false entity excluded from coverage"
    )]
    fn only_kinds_declared_testable_count() {
        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", true, true));
        reg.register(kind("type", true, true));
        // Accepts verify statements but does not count toward coverage.
        reg.register(kind("property", false, true));
        reg.register(kind("feature", false, false));
        assert_eq!(
            testable_kinds(&reg).into_iter().collect::<Vec<_>>(),
            ["behavior", "type"]
        );
    }
}
