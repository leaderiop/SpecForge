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

/// All 57 validation codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationCode {
    // Errors (18)
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
    E016, // Test file not found
    E017, // Enhancement field conflict (unresolved)
    E018, // Enhancement shadows built-in field
    E019, // Wasm plugin load failed
    E020, // Peer dependency unsatisfied
    E021, // Peer dependency cycle
    E022, // Entity kind conflict (two plugins register same name)
    E023, // Entity kind shadows reserved/built-in word

    // Warnings (27)
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
    W018, // Testable entity missing test links
    W019, // Constraint with no protected invariants (governance)
    W020, // Unknown generator (not a built-in and binary not found)
    W021, // Deliverable with no capabilities (product)
    W022, // Enhanced field type mismatch
    W023, // Load-order conflict resolution (priority policy)
    W024, // Missing required enhanced field
    W025, // Fuel exhausted (Wasm execution timeout)
    W026, // Memory limit reached (Wasm)
    W027, // Entity kind load-order resolution (priority policy)

    // Info (8)
    I001, // Stale proposal (governance)
    I003, // Newer format available
    I004, // Unknown entity in reference field (cross-plugin)
    I005, // Unknown provider scheme
    I006, // Unused glossary term (product)
    I007, // AOT cache hit
    I008, // Wasm plugin loaded
    I009, // Wasm plugin initialized
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
            | Self::E015
            | Self::E016
            | Self::E017
            | Self::E018
            | Self::E019
            | Self::E020
            | Self::E021
            | Self::E022
            | Self::E023 => Severity::Error,

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
            | Self::W019
            | Self::W020
            | Self::W021
            | Self::W022
            | Self::W023
            | Self::W024
            | Self::W025
            | Self::W026
            | Self::W027 => Severity::Warning,

            Self::I001 | Self::I003 | Self::I004 | Self::I005 | Self::I006 | Self::I007 | Self::I008 | Self::I009 => Severity::Info,
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
            Self::E016 => "test file not found",
            Self::E017 => "enhancement field conflict",
            Self::E018 => "enhancement shadows built-in field",
            Self::E019 => "wasm plugin load failed",
            Self::E020 => "peer dependency unsatisfied",
            Self::E021 => "peer dependency cycle",
            Self::E022 => "entity kind conflict",
            Self::E023 => "entity kind shadows reserved word",
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
            Self::W018 => "testable entity missing test links",
            Self::W019 => "constraint with no protected invariants",
            Self::W020 => "unknown generator",
            Self::W021 => "deliverable with no capabilities",
            Self::W022 => "enhanced field type mismatch",
            Self::W023 => "load-order conflict resolution",
            Self::W024 => "missing required enhanced field",
            Self::W025 => "wasm fuel exhausted",
            Self::W026 => "wasm memory limit reached",
            Self::W027 => "entity kind load-order resolution",
            Self::I001 => "stale proposal",
            Self::I003 => "newer format available",
            Self::I004 => "unknown entity in reference field",
            Self::I005 => "unknown provider scheme",
            Self::I006 => "unused glossary term",
            Self::I007 => "aot cache hit",
            Self::I008 => "wasm plugin loaded",
            Self::I009 => "wasm plugin initialized",
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
            | Self::E016
            | Self::W001
            | Self::W003
            | Self::W004
            | Self::W007
            | Self::W012
            | Self::W013
            | Self::W015
            | Self::W016
            | Self::W017
            | Self::W018
            | Self::W020
            | Self::W022
            | Self::W023
            | Self::W024
            | Self::E017
            | Self::E018
            | Self::E019
            | Self::E020
            | Self::E021
            | Self::E022
            | Self::E023
            | Self::W025
            | Self::W026
            | Self::W027
            | Self::I003
            | Self::I004
            | Self::I005
            | Self::I007
            | Self::I008
            | Self::I009 => super::Module::Core,

            Self::E007
            | Self::E008
            | Self::E009
            | Self::W002
            | Self::W008
            | Self::W009
            | Self::W010
            | Self::W011
            | Self::W021
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
            Self::E016 => "E016",
            Self::E017 => "E017",
            Self::E018 => "E018",
            Self::E019 => "E019",
            Self::E020 => "E020",
            Self::E021 => "E021",
            Self::E022 => "E022",
            Self::E023 => "E023",
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
            Self::W020 => "W020",
            Self::W021 => "W021",
            Self::W022 => "W022",
            Self::W023 => "W023",
            Self::W024 => "W024",
            Self::W025 => "W025",
            Self::W026 => "W026",
            Self::W027 => "W027",
            Self::I001 => "I001",
            Self::I003 => "I003",
            Self::I004 => "I004",
            Self::I005 => "I005",
            Self::I006 => "I006",
            Self::I007 => "I007",
            Self::I008 => "I008",
            Self::I009 => "I009",
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
        // 23 errors + 26 warnings + 8 info = 57
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
            ValidationCode::E016,
            ValidationCode::E017,
            ValidationCode::E018,
            ValidationCode::E019,
            ValidationCode::E020,
            ValidationCode::E021,
            ValidationCode::E022,
            ValidationCode::E023,
        ]
        .to_vec();
        assert_eq!(errors.len(), 23);
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
            ValidationCode::W020,
            ValidationCode::W021,
            ValidationCode::W022,
            ValidationCode::W023,
            ValidationCode::W024,
            ValidationCode::W025,
            ValidationCode::W026,
            ValidationCode::W027,
        ]
        .to_vec();
        assert_eq!(warnings.len(), 26);
        for code in &warnings {
            assert_eq!(code.severity(), Severity::Warning);
        }

        let infos: Vec<_> = [
            ValidationCode::I001,
            ValidationCode::I003,
            ValidationCode::I004,
            ValidationCode::I005,
            ValidationCode::I006,
            ValidationCode::I007,
            ValidationCode::I008,
            ValidationCode::I009,
        ]
        .to_vec();
        assert_eq!(infos.len(), 8);
        for code in &infos {
            assert_eq!(code.severity(), Severity::Info);
        }

        assert_eq!(errors.len() + warnings.len() + infos.len(), 57);
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
