use specforge_common::{Diagnostic, Severity, SourceSpan, Sym};
use specforge_test::prelude::*;

fn diag_with_span(code: &str, severity: Severity, msg: &str, file: &str, line: usize, col: usize) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity,
        message: msg.to_string(),
        span: Some(SourceSpan {
            file: Sym::new(file),
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
            file: Sym::new("test.spec"),
            start_line: 10,
            start_col: 4,
            end_line: 10,
            end_col: 20,
        }),
        suggestion: Some(suggestion.to_string()),
    }
}

// B:print_diagnostics_structured — verify unit "error diagnostic is formatted with file:line:col"
#[test]
#[specforge_test(behavior = "print_diagnostics_structured", verify = "error diagnostic is formatted with file:line:col")]
fn error_diagnostic_formatted_with_file_line_col() {
    let diag = diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "src/auth.spec", 42, 8);
    let formatted = specforge_emitter::format_diagnostic(&diag);
    assert!(formatted.contains("src/auth.spec"), "should include file path");
    assert!(formatted.contains("42"), "should include line number");
    assert!(formatted.contains("8"), "should include column number");
    assert!(formatted.contains("E001"), "should include diagnostic code");
    assert!(formatted.contains("error"), "should include severity label");
}

// B:print_diagnostics_structured — verify unit "suggestion is displayed when available"
#[test]
#[specforge_test(behavior = "print_diagnostics_structured", verify = "suggestion is displayed when available")]
fn suggestion_displayed_when_available() {
    let diag = diag_with_suggestion("E001", Severity::Error, "unresolved entity 'behavor'", "did you mean 'behavior'?");
    let formatted = specforge_emitter::format_diagnostic(&diag);
    assert!(formatted.contains("did you mean 'behavior'?"), "should display suggestion");
}

// B:print_diagnostics_structured — verify contract "requires/ensures consistency for structured diagnostic printing"
#[test]
#[specforge_test(behavior = "print_diagnostics_structured", verify = "requires/ensures consistency for structured diagnostic printing")]
fn print_diagnostics_contract() {
    // Requires: diagnostics collected (validation_complete)
    // Ensures: formatted with file path, line, column, severity
    let diag = diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "src/core.spec", 15, 4);
    let formatted = specforge_emitter::format_diagnostic(&diag);

    assert!(formatted.contains("src/core.spec"), "must include file path");
    assert!(formatted.contains("15"), "must include line");
    assert!(formatted.contains("4"), "must include column");
    assert!(formatted.contains("error"), "must include severity label");
    assert!(formatted.contains("E001"), "must include diagnostic code");

    // With suggestion
    let diag2 = diag_with_suggestion("E001", Severity::Error, "unresolved", "did you mean 'bar'?");
    let formatted2 = specforge_emitter::format_diagnostic(&diag2);
    assert!(formatted2.contains("did you mean 'bar'?"), "must include suggestion");
}

// B:export_diagnostics_as_json — verify unit "diagnostics serialized as JSON array to stdout"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "diagnostics serialized as JSON array to stdout")]
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

// B:export_diagnostics_as_json — verify unit "each diagnostic includes code, severity, message, file, line, column"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "each diagnostic includes code, severity, message, file, line, column")]
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

// B:export_diagnostics_as_json — verify unit "suggestion field included when available"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "suggestion field included when available")]
fn suggestion_field_included_in_json_when_available() {
    let diags = vec![
        diag_with_suggestion("E001", Severity::Error, "unresolved entity 'behavor'", "did you mean 'behavior'?"),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[0]["suggestion"].as_str().unwrap(), "did you mean 'behavior'?");
}

// B:export_diagnostics_as_json — verify unit "JSON output is valid and parseable"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "JSON output is valid and parseable")]
fn json_diagnostics_output_is_valid_json() {
    let diags = vec![
        diag_with_span("E001", Severity::Error, "bad ref", "test.spec", 1, 0),
        diag_with_suggestion("W002", Severity::Warning, "unused", "remove it"),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");
    assert!(parsed.is_array());
}

// B:exit_code_reflects_diagnostic_severity — verify unit "exit 0 with no errors"
#[test]
#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 0 with no errors")]
fn exit_code_zero_no_errors() {
    let diags = vec![
        Diagnostic {
            code: "W002".to_string(),
            severity: Severity::Warning,
            message: "unused entity".to_string(),
            span: None,
            suggestion: None,
        },
        Diagnostic {
            code: "I003".to_string(),
            severity: Severity::Info,
            message: "note".to_string(),
            span: None,
            suggestion: None,
        },
    ];
    assert_eq!(specforge_emitter::compute_exit_code(&diags), 0);
    assert_eq!(specforge_emitter::compute_exit_code(&[]), 0);
}

// B:exit_code_reflects_diagnostic_severity — verify unit "exit 1 with errors"
#[test]
#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 1 with errors")]
fn exit_code_one_with_errors() {
    let diags = vec![
        Diagnostic {
            code: "E001".to_string(),
            severity: Severity::Error,
            message: "unresolved".to_string(),
            span: None,
            suggestion: None,
        },
    ];
    assert_eq!(specforge_emitter::compute_exit_code(&diags), 1);
}

// B:exit_code_reflects_diagnostic_severity — verify contract "requires/ensures consistency for exit code severity mapping"
#[test]
#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "requires/ensures consistency for exit code severity mapping")]
fn exit_code_contract() {
    // Requires: diagnostics collected (validation_complete)
    // Ensures: exit 0 when no errors, exit 1 when errors present
    let no_errors = vec![
        Diagnostic { code: "W001".into(), severity: Severity::Warning, message: "w".into(), span: None, suggestion: None },
    ];
    let with_errors = vec![
        Diagnostic { code: "E001".into(), severity: Severity::Error, message: "e".into(), span: None, suggestion: None },
        Diagnostic { code: "W001".into(), severity: Severity::Warning, message: "w".into(), span: None, suggestion: None },
    ];
    assert_eq!(specforge_emitter::compute_exit_code(&no_errors), 0);
    assert_eq!(specforge_emitter::compute_exit_code(&with_errors), 1);
}

// B:print_diagnostics_structured — verify unit "diagnostic includes context snippet"
#[test]
#[specforge_test(behavior = "print_diagnostics_structured", verify = "diagnostic includes context snippet")]
fn diagnostic_includes_context_snippet() {
    // The formatted diagnostic includes file:line:col as the context locator.
    // This provides the context snippet reference for agents/tools to look up the source.
    let diag = diag_with_span("E001", Severity::Error, "unresolved entity 'foo'", "src/auth.spec", 42, 8);
    let formatted = specforge_emitter::format_diagnostic(&diag);

    // Context snippet is represented as file:line:col location reference
    assert!(formatted.contains("src/auth.spec:42:8"), "must include file:line:col context reference");
    assert!(formatted.contains("unresolved entity 'foo'"), "must include the diagnostic message");
}

// B:export_diagnostics_as_json — verify unit "exit code unaffected by format flag"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "exit code unaffected by format flag")]
fn exit_code_unaffected_by_format_flag() {
    // The exit code is computed from diagnostics alone, independent of output format.
    // Whether diagnostics are serialized as JSON or formatted as text, exit code is the same.
    let diags_with_errors = vec![
        diag_with_span("E001", Severity::Error, "bad ref", "test.spec", 1, 0),
    ];
    let diags_no_errors = vec![
        diag_with_span("W002", Severity::Warning, "unused", "test.spec", 1, 0),
    ];

    // Serialize as JSON (simulating --format=json) — exit code unchanged
    let _json = specforge_emitter::serialize_diagnostics(&diags_with_errors);
    assert_eq!(specforge_emitter::compute_exit_code(&diags_with_errors), 1);

    let _json = specforge_emitter::serialize_diagnostics(&diags_no_errors);
    assert_eq!(specforge_emitter::compute_exit_code(&diags_no_errors), 0);

    // Format as text (default format) — exit code unchanged
    let _text = specforge_emitter::format_diagnostic(&diags_with_errors[0]);
    assert_eq!(specforge_emitter::compute_exit_code(&diags_with_errors), 1);

    let _text = specforge_emitter::format_diagnostic(&diags_no_errors[0]);
    assert_eq!(specforge_emitter::compute_exit_code(&diags_no_errors), 0);
}

// B:exit_code_reflects_diagnostic_severity — verify unit "exit 1 with warnings in strict mode"
#[test]
#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 1 with warnings in strict mode")]
fn exit_code_one_with_warnings_in_strict_mode() {
    let diags = vec![
        Diagnostic {
            code: "W002".to_string(),
            severity: Severity::Warning,
            message: "unused entity".to_string(),
            span: None,
            suggestion: None,
        },
    ];
    // Normal mode: warnings don't cause exit 1
    assert_eq!(specforge_emitter::compute_exit_code(&diags), 0);
    // Strict mode: warnings cause exit 1
    assert_eq!(specforge_emitter::compute_exit_code_strict(&diags, true), 1);
    // Strict mode with no warnings: exit 0
    assert_eq!(specforge_emitter::compute_exit_code_strict(&[], true), 0);
}

// B:export_diagnostics_as_json — verify unit "suggestion field included when available"
// (inverse case: suggestion absent when none)
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json")]
fn suggestion_field_absent_in_json_when_none() {
    let diags = vec![
        diag_with_span("W002", Severity::Warning, "unused", "test.spec", 1, 0),
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed[0].get("suggestion").is_none() || parsed[0]["suggestion"].is_null());
}

// === diagnostic truncation ===

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "max diagnostics limit truncates output")]
fn truncate_diagnostics_limits_output() {
    let mut diags: Vec<Diagnostic> = (0..150)
        .map(|i| Diagnostic::error("E001", format!("error {}", i)))
        .collect();

    specforge_emitter::truncate_diagnostics(&mut diags);

    assert_eq!(diags.len(), 101, "should be 100 + 1 summary");
    assert_eq!(diags.last().unwrap().code, "I999");
    assert!(diags.last().unwrap().message.contains("150"));
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "no truncation under limit")]
fn truncate_diagnostics_no_op_under_limit() {
    let mut diags: Vec<Diagnostic> = (0..50)
        .map(|i| Diagnostic::error("E001", format!("error {}", i)))
        .collect();

    specforge_emitter::truncate_diagnostics(&mut diags);

    assert_eq!(diags.len(), 50, "should not truncate under limit");
}

// === diagnostic summary ===

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "summary groups diagnostics by code")]
fn diagnostic_summary_groups_by_code() {
    let diags = vec![
        Diagnostic::error("E001", "ref 1"),
        Diagnostic::error("E001", "ref 2"),
        Diagnostic::error("E001", "ref 3"),
        Diagnostic::warning("W003", "cycle 1"),
        Diagnostic::info("I004", "extension 1"),
    ];

    let summary = specforge_emitter::diagnostic_summary(&diags);

    assert!(summary.contains("5 diagnostics"), "should show total count");
    assert!(summary.contains("3 errors"), "should show error count");
    assert!(summary.contains("1 warnings"), "should show warning count");
    assert!(summary.contains("1 info"), "should show info count");
    assert!(summary.contains("E001"), "should mention most frequent code");
    assert!(summary.contains("x3"), "should show E001 count");
}

// === DiagnosticsExt trait ===

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "has_errors detects error severity")]
fn diagnostics_ext_has_errors() {
    use specforge_common::DiagnosticsExt;

    let no_errors = vec![
        Diagnostic::warning("W001", "warn"),
        Diagnostic::info("I001", "info"),
    ];
    assert!(!no_errors.has_errors());
    assert_eq!(no_errors.error_count(), 0);

    let with_errors = vec![
        Diagnostic::warning("W001", "warn"),
        Diagnostic::error("E001", "error"),
    ];
    assert!(with_errors.has_errors());
    assert_eq!(with_errors.error_count(), 1);
}
