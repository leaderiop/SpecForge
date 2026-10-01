use serde::Serialize;
use specforge_common::{Diagnostic, Severity, SourceSpan};

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
/// one entry per diagnostic with code, severity, message, suggestion, the
/// span nested under `span`, and the span's start flat as `file`, `line`
/// and `column`. Absent values are `null`, never missing keys. The shape
/// is the superset of the nested (CLI) and flat (MCP) shapes the surfaces
/// printed before, so readers of either keep working.
pub fn diagnostics_json(diagnostics: &[Diagnostic]) -> Vec<DiagnosticJson<'_>> {
    diagnostics
        .iter()
        .map(|d| DiagnosticJson {
            code: &d.code,
            severity: &d.severity,
            message: &d.message,
            span: d.span.as_ref(),
            suggestion: d.suggestion.as_deref(),
            file: d.span.as_ref().map(|s| s.file.as_str()),
            line: d.span.as_ref().map(|s| s.start_line),
            column: d.span.as_ref().map(|s| s.start_col),
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

/// Group diagnostics by code and return a summary string.
/// Shows the top 5 most frequent diagnostic codes with counts.
pub fn diagnostic_summary(diagnostics: &[Diagnostic]) -> String {
    use std::collections::HashMap;

    let mut counts: HashMap<&str, (usize, &Severity)> = HashMap::new();
    for d in diagnostics {
        counts
            .entry(&d.code)
            .and_modify(|(count, _)| *count += 1)
            .or_insert((1, &d.severity));
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|(_, (count, _))| std::cmp::Reverse(*count));

    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let infos = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Info)
        .count();

    let mut summary = format!(
        "{} diagnostics: {} errors, {} warnings, {} info",
        diagnostics.len(),
        errors,
        warnings,
        infos,
    );

    if !sorted.is_empty() {
        summary.push_str("\n  top codes:");
        for (code, (count, severity)) in sorted.iter().take(5) {
            let label = match severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            summary.push_str(&format!("\n    {} ({}) x{}", code, label, count));
        }
    }

    if diagnostics.len() > 5 {
        summary.push_str("\n  run `specforge explain <code>` for details on any diagnostic code");
    }

    summary
}

/// One diagnostic as JSON: see [`diagnostics_json`].
#[derive(Debug, Serialize)]
pub struct DiagnosticJson<'a> {
    pub code: &'a str,
    pub severity: &'a Severity,
    pub message: &'a str,
    /// The location nested: file, start and end line and column.
    pub span: Option<&'a SourceSpan>,
    pub suggestion: Option<&'a str>,
    /// The span's start, flat.
    pub file: Option<&'a str>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}
