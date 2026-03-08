use specforge_common::{Diagnostic, Severity, SourceSpan};

fn diag_with_span(code: &str, severity: Severity, msg: &str, file: &str, line: usize, col: usize) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: msg.to_string(),
        span: Some(SourceSpan {
            file: file.to_string(),
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col + 5,
        }),
        suggestion: None,
    }
}

fn diag_with_suggestion(code: &str, severity: Severity, msg: &str, suggestion: &str) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: msg.to_string(),
        span: Some(SourceSpan {
            file: "test.spec".to_string(),
            start_line: 10,
            start_col: 4,
            end_line: 10,
            end_col: 20,
        }),
        suggestion: Some(suggestion.to_string()),
    }
}

#[test]
fn error_diagnostic_formatted_with_file_line_col() {
    let diag = diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "src/auth.spec", 42, 8);
    let formatted = specforge_emitter::format_diagnostic(&diag);
    assert!(formatted.contains("src/auth.spec"), "should include file path");
    assert!(formatted.contains("42"), "should include line number");
    assert!(formatted.contains("8"), "should include column number");
    assert!(formatted.contains("E001"), "should include diagnostic code");
    assert!(formatted.contains("error"), "should include severity label");
}

#[test]
fn suggestion_displayed_when_available() {
    let diag = diag_with_suggestion("E001", Severity::Error, "unresolved entity 'behavor'", "did you mean 'behavior'?");
    let formatted = specforge_emitter::format_diagnostic(&diag);
    assert!(formatted.contains("did you mean 'behavior'?"), "should display suggestion");
}

#[test]
fn diagnostics_serialized_as_json_array() {
    let diags = vec![
        diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "test.spec", 10, 4),
        diag_with_span("W002", Severity::Warning, "unused entity 'bar'", "test.spec", 20, 0),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    let arr = parsed.as_array().expect("should be JSON array");
    assert_eq!(arr.len(), 2);
}

#[test]
fn each_diagnostic_includes_code_severity_message_file_line_column() {
    let diags = vec![
        diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "src/auth.spec", 42, 8),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let entry = &parsed[0];

    assert_eq!(entry["code"].as_str().unwrap(), "E001");
    assert_eq!(entry["severity"].as_str().unwrap(), "Error");
    assert_eq!(entry["message"].as_str().unwrap(), "unresolved entity 'foo'");
    assert_eq!(entry["file"].as_str().unwrap(), "src/auth.spec");
    assert_eq!(entry["line"].as_u64().unwrap(), 42);
    assert_eq!(entry["column"].as_u64().unwrap(), 8);
}

#[test]
fn suggestion_field_included_in_json_when_available() {
    let diags = vec![
        diag_with_suggestion("E001", Severity::Error, "unresolved entity 'behavor'", "did you mean 'behavior'?"),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[0]["suggestion"].as_str().unwrap(), "did you mean 'behavior'?");
}

#[test]
fn suggestion_field_absent_in_json_when_none() {
    let diags = vec![
        diag_with_span("W002", Severity::Warning, "unused", "test.spec", 1, 0),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed[0].get("suggestion").is_none() || parsed[0]["suggestion"].is_null());
}
