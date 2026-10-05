//! Diagnostics as every surface prints them: the one-line text form, the
//! JSON form (with the catalog's title per code), the output cap and the
//! exit code. The human, source-annotated rendering is
//! `specforge_validator::render_diagnostics`.

use crate::{Diagnostic, DiagnosticData, Severity, SourceSpan};
use serde::Serialize;

pub fn format_diagnostic(diag: &Diagnostic) -> String {
    let severity_label = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };

    let location = if let Some(span) = &diag.span {
        format!("{}:{}:{}", span.file, span.start_line, span.start_col)
    } else {
        // Use the diagnostic code as a deterministic fallback so that
        // spanless diagnostics are still identifiable and grep-able.
        format!("<{}>", diag.code)
    };

    let mut output = format!(
        "{}: {}[{}]: {}",
        location, severity_label, diag.code, diag.message
    );

    if let Some(suggestion) = &diag.suggestion {
        output.push_str(&format!("\n  help: {}", suggestion));
    }

    output
}

/// Diagnostics as the JSON every surface prints (`check --format json`,
/// MCP validate, the `specforge://diagnostics` resource, tool `_meta`):
/// one entry per diagnostic with code, the catalogue's title for it,
/// severity, message, suggestion, the
/// span nested under `span`, and the span's start flat as `file`, `line`
/// and `column`. Absent values are `null`, never missing keys — except
/// `data`, the typed payload, which is there only when the diagnostic
/// carries one (so a diagnostic without it prints as it always has). The
/// shape is the superset of the nested (CLI) and flat (MCP) shapes the
/// surfaces printed before, so readers of either keep working.
pub fn diagnostics_json(diagnostics: &[Diagnostic]) -> Vec<DiagnosticJson<'_>> {
    diagnostics
        .iter()
        .map(|d| DiagnosticJson {
            code: &d.code,
            title: specforge_diagnostics::lookup(&d.code).map(|entry| entry.title),
            severity: &d.severity,
            message: &d.message,
            span: d.span.as_ref(),
            suggestion: d.suggestion.as_deref(),
            file: d.span.as_ref().map(|s| s.file.as_str()),
            line: d.span.as_ref().map(|s| s.start_line),
            column: d.span.as_ref().map(|s| s.start_col),
            data: d.data.as_deref(),
        })
        .collect()
}

/// [`diagnostics_json`] as compact JSON text.
pub fn serialize_diagnostics(diagnostics: &[Diagnostic]) -> String {
    serde_json::to_string(&diagnostics_json(diagnostics))
        .expect("diagnostic serialization cannot fail")
}

/// Maximum number of diagnostics to emit before truncating.
/// This prevents runaway output on extremely malformed specs.
pub const MAX_DIAGNOSTICS: usize = 100;

/// Truncate diagnostics to MAX_DIAGNOSTICS, adding a summary if truncated.
pub fn truncate_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    if diagnostics.len() > MAX_DIAGNOSTICS {
        let total = diagnostics.len();
        diagnostics.truncate(MAX_DIAGNOSTICS);
        diagnostics.push(Diagnostic::info(
            "I999",
            format!(
                "showing first {} of {} diagnostics — fix these and rerun",
                MAX_DIAGNOSTICS, total
            ),
        ));
    }
}

/// One diagnostic as JSON: see [`diagnostics_json`].
#[derive(Debug, Serialize)]
pub struct DiagnosticJson<'a> {
    pub code: &'a str,
    /// The catalogue's title for the code; null for a code it doesn't have
    /// (a third-party extension's).
    pub title: Option<&'static str>,
    pub severity: &'a Severity,
    pub message: &'a str,
    /// The location nested: file, start and end line and column.
    pub span: Option<&'a SourceSpan>,
    pub suggestion: Option<&'a str>,
    /// The span's start, flat.
    pub file: Option<&'a str>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// The diagnostic's typed payload ([`DiagnosticData`]); the key is
    /// absent when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<&'a DiagnosticData>,
}

/// Compute the process exit code from collected diagnostics.
///
/// Returns 0 if no error-level diagnostics exist, 1 otherwise. Strict
/// mode is not a separate rule: `specforge_project::DiagnosticPolicy`
/// promotes warnings to errors before the exit code is computed.
pub fn compute_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    if diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error))
    {
        1
    } else {
        0
    }
}
