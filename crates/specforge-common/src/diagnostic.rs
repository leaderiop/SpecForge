use crate::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub suggestion: Option<String>,
    /// What the diagnostic is about, typed, for the consumers that act on
    /// it (the LSP's quick fixes) — so none of them parses `message`, which
    /// is presentation. Absent for most diagnostics, and then serialized
    /// not at all. Boxed so a diagnostic without one stays small (it is
    /// the error type of many `Result`s).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Box<DiagnosticData>>,
}

/// A diagnostic's structured payload: the values its message names, as
/// data. Serialized with a `kind` tag (`{"kind": "unresolved_reference",
/// …}`); a variant exists only for a diagnostic some consumer acts on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiagnosticData {
    /// E003: `entity`'s reference field `field` names `target`, which no
    /// entity declares. `did_you_mean` is the closest declared id, when one
    /// is close enough to suggest.
    UnresolvedReference {
        target: String,
        entity: String,
        field: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        did_you_mean: Option<String>,
    },
    /// E025: a `use` import names `path`, which resolves to no `.spec`
    /// file. `did_you_mean` is the closest known path, when one is close.
    UnresolvedImport {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        did_you_mean: Option<String>,
    },
    /// E013, E026: `keyword` — an entity id, or an extension's kind
    /// keyword — collides with a keyword the grammar or an earlier
    /// extension already owns.
    ShadowedKeyword { keyword: String },
    /// W061: the entities of a reference cycle, in path order (the first
    /// one repeated last, closing the cycle).
    ReferenceCycle { path: Vec<String> },
    /// A diagnostic an extension pass raised about `entity` (the pass named
    /// it), whether or not the graph holds that entity.
    Subject { entity: String },
}

impl DiagnosticData {
    /// The entities the payload says the diagnostic is about, each once,
    /// in the payload's order: an unresolved reference's holder, a cycle's
    /// entities, a pass diagnostic's subject. None for the others.
    /// Navigation attributes a diagnostic from these, never its message.
    pub fn entities(&self) -> Vec<&str> {
        let mut entities: Vec<&str> = Vec::new();
        let named: Vec<&str> = match self {
            DiagnosticData::UnresolvedReference { entity, .. }
            | DiagnosticData::Subject { entity } => vec![entity.as_str()],
            DiagnosticData::ReferenceCycle { path } => path.iter().map(String::as_str).collect(),
            DiagnosticData::UnresolvedImport { .. } | DiagnosticData::ShadowedKeyword { .. } => {
                Vec::new()
            }
        };
        for entity in named {
            if !entities.contains(&entity) {
                entities.push(entity);
            }
        }
        entities
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.code, self.message)
    }
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            span: None,
            suggestion: None,
            data: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            suggestion: None,
            data: None,
        }
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Info,
            message: message.into(),
            span: None,
            suggestion: None,
            data: None,
        }
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_data(mut self, data: DiagnosticData) -> Self {
        self.data = Some(Box::new(data));
        self
    }

    /// Returns true if this diagnostic is an error.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Extension trait for collections of diagnostics.
pub trait DiagnosticsExt {
    /// Returns true if the collection contains at least one error-severity diagnostic.
    fn has_errors(&self) -> bool;

    /// Returns the number of error-severity diagnostics.
    fn error_count(&self) -> usize;
}

impl DiagnosticsExt for Vec<Diagnostic> {
    fn has_errors(&self) -> bool {
        self.iter().any(|d| d.is_error())
    }

    fn error_count(&self) -> usize {
        self.iter().filter(|d| d.is_error()).count()
    }
}

impl DiagnosticsExt for [Diagnostic] {
    fn has_errors(&self) -> bool {
        self.iter().any(|d| d.is_error())
    }

    fn error_count(&self) -> usize {
        self.iter().filter(|d| d.is_error()).count()
    }
}
