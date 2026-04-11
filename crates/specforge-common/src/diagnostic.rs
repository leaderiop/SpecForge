use crate::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub suggestion: Option<String>,
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
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            suggestion: None,
        }
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Info,
            message: message.into(),
            span: None,
            suggestion: None,
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
