//! Severities raised per code, upward only (ADR 0026 D3, ADR 0286). K6 fills and applies them.

use specforge_common::{Diagnostic, Severity};
use std::collections::BTreeMap;

/// Severities raised per code (`Diagnostic::code` text as it prints: `gsd/W304`), upward only.
/// Pure: no file, no surface. Empty until a settings layer fills it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Raises {
    by_code: BTreeMap<String, Severity>,
}

/// Info < Warning < Error: `Severity` derives no `Ord`, and its declaration order puts `Error`
/// first, which would invert a derived one.
fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Info => 0,
        Severity::Warning => 1,
        Severity::Error => 2,
    }
}

impl Raises {
    /// Raise `code` to at least `severity`; a second raise of one code keeps the higher.
    pub fn raise(&mut self, code: impl Into<String>, severity: Severity) {
        let held = self.by_code.entry(code.into()).or_insert(severity);
        if rank(severity) > rank(*held) {
            *held = severity;
        }
    }

    /// `diagnostic` at the higher of its severity and the raise of its code: never lower.
    pub fn apply(&self, mut diagnostic: Diagnostic) -> Diagnostic {
        if let Some(&raised) = self.by_code.get(&diagnostic.code)
            && rank(raised) > rank(diagnostic.severity)
        {
            diagnostic.severity = raised;
        }
        diagnostic
    }

    pub fn is_empty(&self) -> bool {
        self.by_code.is_empty()
    }
}
