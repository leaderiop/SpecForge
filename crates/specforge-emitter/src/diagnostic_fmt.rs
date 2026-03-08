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
            (Some(span.file.clone()), Some(span.start_line), Some(span.start_col))
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
