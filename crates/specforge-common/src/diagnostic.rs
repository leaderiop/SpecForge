use crate::SourceSpan;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Severity levels for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
            Self::Info => f.write_str("info"),
        }
    }
}

/// All 38 validation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationCode {
    // Errors (15)
    E001, // Unresolved reference
    E002, // Duplicate ID
    E003, // Circular import
    E004, // Empty scenario block
    E005, // RPN mismatch (governance)
    E006, // Event trigger invalid
    E007, // Circular library dependency (product)
    E008, // Persona not defined (product)
    E009, // Surface not defined (product)
    E010, // Syntax error
    E011, // Invalid ref target format
    E012, // Unknown provider kind
    E013, // Reserved word used as identifier
    E014, // Invalid identifier characters
    E015, // Duplicate scenario title

    // Warnings (18)
    W001, // Orphan behavior
    W002, // Orphan feature (product)
    W003, // Unused invariant
    W004, // Unverified behavior
    W005, // Unmitigated high-risk invariant (governance)
    W006, // Unconstrained behavior (governance)
    W007, // Orphan event
    W008, // Uncovered capability (product)
    W009, // Orphan library (product)
    W010, // Deprecated feature (product)
    W011, // Orphan capability (product)
    W012, // Orphan ref
    W013, // Vague name
    W015, // Scenario missing when step
    W016, // Scenario missing then step
    W017, // Unused entity (generic)
    W018, // Deliverable with no capabilities (product)
    W019, // Constraint with no protected invariants (governance)

    // Info (5)
    I001, // Stale proposal (governance)
    I003, // Newer format available
    I004, // Unknown entity in reference field (cross-plugin)
    I005, // Unknown provider scheme
    I006, // Unused glossary term (product)
}

impl ValidationCode {
    pub fn severity(&self) -> Severity {
        match self {
            Self::E001
            | Self::E002
            | Self::E003
            | Self::E004
            | Self::E005
            | Self::E006
            | Self::E007
            | Self::E008
            | Self::E009
            | Self::E010
            | Self::E011
            | Self::E012
            | Self::E013
            | Self::E014
            | Self::E015 => Severity::Error,

            Self::W001
            | Self::W002
            | Self::W003
            | Self::W004
            | Self::W005
            | Self::W006
            | Self::W007
            | Self::W008
            | Self::W009
            | Self::W010
            | Self::W011
            | Self::W012
            | Self::W013
            | Self::W015
            | Self::W016
            | Self::W017
            | Self::W018
            | Self::W019 => Severity::Warning,

            Self::I001 | Self::I003 | Self::I004 | Self::I005 | Self::I006 => Severity::Info,
        }
    }

    /// Short human-readable description for this code.
    pub fn message(&self) -> &'static str {
        match self {
            Self::E001 => "unresolved reference",
            Self::E002 => "duplicate entity name",
            Self::E003 => "circular import",
            Self::E004 => "empty scenario block",
            Self::E005 => "RPN mismatch",
            Self::E006 => "invalid event trigger",
            Self::E007 => "circular library dependency",
            Self::E008 => "persona not defined",
            Self::E009 => "surface not defined",
            Self::E010 => "syntax error",
            Self::E011 => "invalid ref target format",
            Self::E012 => "unknown provider kind",
            Self::E013 => "reserved word used as identifier",
            Self::E014 => "invalid identifier characters",
            Self::E015 => "duplicate scenario title",
            Self::W001 => "orphan behavior",
            Self::W002 => "orphan feature",
            Self::W003 => "unused invariant",
            Self::W004 => "unverified behavior",
            Self::W005 => "unmitigated high-risk invariant",
            Self::W006 => "unconstrained behavior",
            Self::W007 => "orphan event",
            Self::W008 => "uncovered capability",
            Self::W009 => "orphan library",
            Self::W010 => "deprecated feature",
            Self::W011 => "orphan capability",
            Self::W012 => "orphan ref",
            Self::W013 => "vague entity name",
            Self::W015 => "scenario missing when step",
            Self::W016 => "scenario missing then step",
            Self::W017 => "unused entity",
            Self::W018 => "deliverable with no capabilities",
            Self::W019 => "constraint with no protected invariants",
            Self::I001 => "stale proposal",
            Self::I003 => "newer format available",
            Self::I004 => "unknown entity in reference field",
            Self::I005 => "unknown provider scheme",
            Self::I006 => "unused glossary term",
        }
    }

    /// Which module owns this validation code.
    pub fn module(&self) -> super::Module {
        match self {
            Self::E001
            | Self::E002
            | Self::E003
            | Self::E004
            | Self::E006
            | Self::E010
            | Self::E011
            | Self::E012
            | Self::E013
            | Self::E014
            | Self::E015
            | Self::W001
            | Self::W003
            | Self::W004
            | Self::W007
            | Self::W012
            | Self::W013
            | Self::W015
            | Self::W016
            | Self::W017
            | Self::I003
            | Self::I004
            | Self::I005 => super::Module::Core,

            Self::E007
            | Self::E008
            | Self::E009
            | Self::W002
            | Self::W008
            | Self::W009
            | Self::W010
            | Self::W011
            | Self::W018
            | Self::I006 => super::Module::Product,

            Self::E005 | Self::W005 | Self::W006 | Self::W019 | Self::I001 => {
                super::Module::Governance
            }
        }
    }
}

impl fmt::Display for ValidationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::E001 => "E001",
            Self::E002 => "E002",
            Self::E003 => "E003",
            Self::E004 => "E004",
            Self::E005 => "E005",
            Self::E006 => "E006",
            Self::E007 => "E007",
            Self::E008 => "E008",
            Self::E009 => "E009",
            Self::E010 => "E010",
            Self::E011 => "E011",
            Self::E012 => "E012",
            Self::E013 => "E013",
            Self::E014 => "E014",
            Self::E015 => "E015",
            Self::W001 => "W001",
            Self::W002 => "W002",
            Self::W003 => "W003",
            Self::W004 => "W004",
            Self::W005 => "W005",
            Self::W006 => "W006",
            Self::W007 => "W007",
            Self::W008 => "W008",
            Self::W009 => "W009",
            Self::W010 => "W010",
            Self::W011 => "W011",
            Self::W012 => "W012",
            Self::W013 => "W013",
            Self::W015 => "W015",
            Self::W016 => "W016",
            Self::W017 => "W017",
            Self::W018 => "W018",
            Self::W019 => "W019",
            Self::I001 => "I001",
            Self::I003 => "I003",
            Self::I004 => "I004",
            Self::I005 => "I005",
            Self::I006 => "I006",
        };
        f.write_str(code)
    }
}

/// A single diagnostic emitted by the compiler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: ValidationCode,
    pub span: SourceSpan,
    pub message: String,
    pub help: Option<String>,
    pub labels: Vec<DiagnosticLabel>,
}

/// An additional label pointing to a related source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    pub span: SourceSpan,
    pub message: String,
}

impl Diagnostic {
    pub fn new(code: ValidationCode, span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
            help: None,
            labels: Vec::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_label(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }

    pub fn severity(&self) -> Severity {
        self.code.severity()
    }
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.span
            .cmp(&other.span)
            .then(self.code.cmp(&other.code))
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Accumulator for diagnostics during compilation (INV-SF-2: multi-error collection).
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Sort diagnostics by (file, line, col, code) for deterministic output (INV-SF-10).
    pub fn sorted(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort();
        self.diagnostics
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity() == Severity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity() == Severity::Info)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity() == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_code_count() {
        // 15 errors + 18 warnings + 5 info = 38
        let errors: Vec<_> = [
            ValidationCode::E001,
            ValidationCode::E002,
            ValidationCode::E003,
            ValidationCode::E004,
            ValidationCode::E005,
            ValidationCode::E006,
            ValidationCode::E007,
            ValidationCode::E008,
            ValidationCode::E009,
            ValidationCode::E010,
            ValidationCode::E011,
            ValidationCode::E012,
            ValidationCode::E013,
            ValidationCode::E014,
            ValidationCode::E015,
        ]
        .to_vec();
        assert_eq!(errors.len(), 15);
        for code in &errors {
            assert_eq!(code.severity(), Severity::Error);
        }

        let warnings: Vec<_> = [
            ValidationCode::W001,
            ValidationCode::W002,
            ValidationCode::W003,
            ValidationCode::W004,
            ValidationCode::W005,
            ValidationCode::W006,
            ValidationCode::W007,
            ValidationCode::W008,
            ValidationCode::W009,
            ValidationCode::W010,
            ValidationCode::W011,
            ValidationCode::W012,
            ValidationCode::W013,
            ValidationCode::W015,
            ValidationCode::W016,
            ValidationCode::W017,
            ValidationCode::W018,
            ValidationCode::W019,
        ]
        .to_vec();
        assert_eq!(warnings.len(), 18);
        for code in &warnings {
            assert_eq!(code.severity(), Severity::Warning);
        }

        let infos: Vec<_> = [
            ValidationCode::I001,
            ValidationCode::I003,
            ValidationCode::I004,
            ValidationCode::I005,
            ValidationCode::I006,
        ]
        .to_vec();
        assert_eq!(infos.len(), 5);
        for code in &infos {
            assert_eq!(code.severity(), Severity::Info);
        }

        assert_eq!(errors.len() + warnings.len() + infos.len(), 38);
    }

    #[test]
    fn diagnostic_bag_sorting() {
        let mut bag = DiagnosticBag::new();
        bag.push(Diagnostic::new(
            ValidationCode::W001,
            SourceSpan::new("b.spec", 5, 1, 5, 10),
            "orphan",
        ));
        bag.push(Diagnostic::new(
            ValidationCode::E001,
            SourceSpan::new("a.spec", 3, 1, 3, 10),
            "unresolved",
        ));
        bag.push(Diagnostic::new(
            ValidationCode::E002,
            SourceSpan::new("a.spec", 1, 1, 1, 10),
            "duplicate",
        ));

        let sorted = bag.sorted();
        assert_eq!(sorted[0].code, ValidationCode::E002); // a.spec:1
        assert_eq!(sorted[1].code, ValidationCode::E001); // a.spec:3
        assert_eq!(sorted[2].code, ValidationCode::W001); // b.spec:5
    }

    #[test]
    fn diagnostic_bag_counts() {
        let mut bag = DiagnosticBag::new();
        let span = SourceSpan::file_start("test.spec");
        bag.push(Diagnostic::new(ValidationCode::E001, span.clone(), "err1"));
        bag.push(Diagnostic::new(ValidationCode::E002, span.clone(), "err2"));
        bag.push(Diagnostic::new(ValidationCode::W001, span.clone(), "warn1"));
        bag.push(Diagnostic::new(ValidationCode::I003, span, "info1"));

        assert_eq!(bag.len(), 4);
        assert_eq!(bag.error_count(), 2);
        assert_eq!(bag.warning_count(), 1);
        assert_eq!(bag.info_count(), 1);
        assert!(bag.has_errors());
    }

    #[test]
    fn diagnostic_builder() {
        let d = Diagnostic::new(
            ValidationCode::E001,
            SourceSpan::new("test.spec", 10, 5, 10, 15),
            "unresolved reference `data_persistance`",
        )
        .with_help("did you mean `data_persistence`?")
        .with_label(
            SourceSpan::new("test.spec", 10, 5, 10, 15),
            "not found",
        );

        assert_eq!(d.code, ValidationCode::E001);
        assert_eq!(d.help.as_deref(), Some("did you mean `data_persistence`?"));
        assert_eq!(d.labels.len(), 1);
    }
}
