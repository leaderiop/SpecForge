use crate::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub suggestion: Option<String>,
}
