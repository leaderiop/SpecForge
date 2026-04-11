use serde::Serialize;
use specforge_common::{Diagnostic, Severity};

pub fn format_diagnostic(diag: &Diagnostic) -> String {
    let severity_label = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    };

    let location = if let Some(span) = &diag.span {
        format!("{}:{}:{}", span.file, span.start_line, span.start_col)
    } else {
        "<unknown>".to_string()
    };

    let mut output = format!("{}: {}[{}]: {}", location, severity_label, diag.code, diag.message);

    if let Some(suggestion) = &diag.suggestion {
        output.push_str(&format!("\n  help: {}", suggestion));
    }

    output
}

pub fn serialize_diagnostics(diagnostics: &[Diagnostic]) -> String {
    let entries: Vec<DiagnosticEntry> = diagnostics.iter().map(|d| {
        let (file, line, column) = if let Some(span) = &d.span {
            (Some(span.file.to_string()), Some(span.start_line), Some(span.start_col))
        } else {
            (None, None, None)
        };

        DiagnosticEntry {
            code: &d.code,
            severity: &d.severity,
            message: &d.message,
            file,
            line,
            column,
            suggestion: d.suggestion.as_deref(),
        }
    }).collect();

    serde_json::to_string_pretty(&entries).expect("diagnostic serialization cannot fail")
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
                MAX_DIAGNOSTICS,
                total
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
    sorted.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let infos = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();

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
        summary.push_str(&format!(
            "\n  run `specforge explain <code>` for details on any diagnostic code"
        ));
    }

    summary
}

#[derive(Serialize)]
struct DiagnosticEntry<'a> {
    code: &'a str,
    severity: &'a Severity,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<&'a str>,
}
