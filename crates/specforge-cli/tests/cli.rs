use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn setup_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }
    dir
}

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

// === check_mode_for_ci (self-check) ===

#[specforge_test(
    behavior = "check_mode_for_ci",
    verify = "check mode works in CI environment"
)]
fn self_check_runs_without_crashing() {
    // Find the project's spec/ directory relative to the crate manifest
    let spec_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // project root
        .join("spec");

    if !spec_dir.exists() {
        panic!("spec/ directory not found at {:?}", spec_dir);
    }

    // As a CI runner invokes it: CI set, no terminal, empty stdin.
    let ci_check = |args: &[&str], path: &std::path::Path| {
        specforge_cmd()
            .env("CI", "true")
            .env("TERM", "dumb")
            .env_remove("CLICOLOR_FORCE")
            .write_stdin("")
            .arg("check")
            .args(args)
            .arg(path)
            .output()
            .unwrap()
    };

    // The repository's own specs, which CI checks: clean, exit 0, the
    // summary on stderr and nothing on stdout.
    let output = ci_check(&[], &spec_dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("0 errors, 0 warnings, 0 infos"), "{stderr}");
    assert!(!stderr.contains('\x1b'), "no terminal escapes: {stderr:?}");

    let output = ci_check(&["--format=json"], &spec_dir);
    assert_eq!(output.status.code(), Some(0));
    let diagnostics: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(diagnostics, serde_json::json!([]));

    // A broken project fails the CI step with exit 1 and says why on stderr.
    let dir = setup_project(&[(
        "main.spec",
        "\nbehavior alpha \"A\" { contract \"first\" }\nfeature gamma \"G\" { behaviors [alpha, nonexistent] }\n",
    )]);
    let output = ci_check(&[], dir.path());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("[E003] Error: unresolved reference 'nonexistent' in entity 'gamma'"),
        "{stderr}"
    );
}

// === check_mode_for_ci ===

#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "exit 0 with no errors"
)]
fn check_clean_project_exits_zero() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success();
}

// === exit_code_reflects_diagnostic_severity ===

#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "exit 1 with errors"
)]
fn check_project_with_errors_exits_one() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .code(1);
}

#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "exit 1 with warnings in strict mode"
)]
fn strict_mode_promotes_warnings_to_errors() {
    // Orphan ref produces W012 (warning) — with --strict it becomes an error → exit 1
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Orphan ref"
"#,
    )]);

    // Without --strict: exit 0 (only warnings)
    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success();

    // With --strict: exit 1 (warnings promoted to errors)
    specforge_cmd()
        .arg("check")
        .arg("--strict")
        .arg(dir.path())
        .assert()
        .code(1);
}

// === print_diagnostics_structured ===

#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "JSON output is valid and parseable"
)]
fn json_format_outputs_valid_json() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    assert!(
        parsed.is_array(),
        "JSON output should be an array of diagnostics"
    );
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "should have at least one diagnostic");

    let first = &arr[0];
    assert!(
        first.get("code").is_some(),
        "diagnostic should have 'code' field"
    );
    assert!(
        first.get("severity").is_some(),
        "diagnostic should have 'severity' field"
    );
    assert!(
        first.get("message").is_some(),
        "diagnostic should have 'message' field"
    );
}

// === print_diagnostics_structured ===

#[specforge_test(
    behavior = "print_diagnostics_structured",
    verify = "error diagnostic is formatted with file:line:col"
)]
fn structured_output_includes_file_line_col() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    // `nonexistent` starts at line 3, column 39.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("main.spec:3:39"),
        "should contain file:line:col in stderr: {}",
        stderr
    );
}

#[specforge_test(
    behavior = "print_diagnostics_structured",
    verify = "diagnostic includes context snippet"
)]
fn structured_output_includes_context_snippet() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    // The offending source line, numbered, with the reference underlined.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(" 3 │ feature gamma \"G\" { behaviors [alpha, nonexistent] }\n"),
        "should contain the source line in stderr: {}",
        stderr
    );
    let lines: Vec<&str> = stderr.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains("feature gamma"))
        .unwrap();
    let column_of = |line: &str, byte: usize| line[..byte].chars().count();
    let source = lines[at];
    let underline = lines[at + 1];
    assert_eq!(
        column_of(underline, underline.find('─').unwrap()),
        column_of(source, source.find("nonexistent").unwrap()),
        "the underline starts under `nonexistent`: {stderr}"
    );
    assert_eq!(
        underline.trim_start_matches([' ', '│']),
        "─────┬─────",
        "and spans its 11 characters: {stderr}"
    );
}

#[specforge_test(
    behavior = "print_diagnostics_structured",
    verify = "suggestion is displayed when available"
)]
fn structured_output_includes_suggestion() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did you mean") && stderr.contains("alpha_parser"),
        "should display did-you-mean suggestion in stderr: {}",
        stderr
    );
}

// === check_mode_for_ci ===

#[specforge_test(
    behavior = "check_mode_for_ci",
    verify = "check mode produces no output files"
)]
fn check_mode_produces_no_output_files() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    let before: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();

    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success();

    let after: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();

    assert_eq!(before, after, "check mode should not produce any new files");
}

#[specforge_test(
    behavior = "check_mode_for_ci",
    verify = "check mode prints diagnostics to stderr"
)]
fn check_mode_prints_to_stderr() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E001") || stderr.contains("error"),
        "diagnostics should be printed to stderr: {}",
        stderr
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "human-format check should not produce stdout output, got: {}",
        stdout
    );
}

// === export_diagnostics_as_json ===

#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "each diagnostic includes code, severity, message, file, line, column"
)]
fn json_diagnostics_have_complete_fields() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    for diag in &diagnostics {
        assert!(diag.get("code").is_some(), "missing 'code': {:?}", diag);
        assert!(
            diag.get("severity").is_some(),
            "missing 'severity': {:?}",
            diag
        );
        assert!(
            diag.get("message").is_some(),
            "missing 'message': {:?}",
            diag
        );
        if let Some(span) = diag.get("span") {
            assert!(
                span.get("file").is_some(),
                "span missing 'file': {:?}",
                span
            );
            assert!(
                span.get("start_line").is_some(),
                "span missing 'start_line': {:?}",
                span
            );
            assert!(
                span.get("start_col").is_some(),
                "span missing 'start_col': {:?}",
                span
            );
        }
    }
}

// === contract tests ===

// Not linked to the Print Diagnostics Structured contract: its
// color_coding_applied clause (errors red, warnings yellow, info blue) is
// not implemented — render_diagnostics renders without color on purpose and
// check never re-colors it. This checks the structured part.
#[test]
fn print_diagnostics_contract_consistency() {
    // Requires: validation_complete fired (diagnostics collected)
    // Ensures: structured format with file:line:col, color-coded severity
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Structured format: code and severity, file:line:col, source line.
    assert!(
        stderr.contains("[E003] Error: unresolved reference 'nonexistent' in entity 'gamma'"),
        "{stderr}"
    );
    assert!(stderr.contains("─[ main.spec:3:39 ]"), "{stderr}");
    assert!(
        stderr.contains(" 3 │ feature gamma \"G\" { behaviors [alpha, nonexistent] }"),
        "{stderr}"
    );
}

#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "Exit Code Reflects Diagnostic Severity: exit code severity mapping holds — validation_complete_fired, exit_zero_on_clean, exit_one_on_errors, strict_mode_enforced"
)]
fn exit_code_contract_consistency() {
    // Requires: validation_complete fired
    // Ensures: exit 0 on clean, exit 1 on errors, strict promotes warnings
    let clean_dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);
    specforge_cmd()
        .arg("check")
        .arg(clean_dir.path())
        .assert()
        .success();

    let error_dir = setup_project(&[("main.spec", r#"feature g "G" { behaviors [nonexistent] }"#)]);
    specforge_cmd()
        .arg("check")
        .arg(error_dir.path())
        .assert()
        .code(1);
}

#[specforge_test(
    behavior = "check_mode_for_ci",
    verify = "Check Mode for CI: CI check mode holds — validation_complete_fired, no_output_files_produced, diagnostics_to_stderr, appropriate_exit_code"
)]
fn check_mode_contract_consistency() {
    // Requires: validation_complete fired
    // Ensures: no output files, diagnostics to stderr, appropriate exit code
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let before_count = fs::read_dir(dir.path()).unwrap().count();

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let after_count = fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(
        before_count, after_count,
        "check must not produce output files"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "diagnostics must go to stderr");

    assert_eq!(output.status.code(), Some(1), "errors must cause exit 1");
}

#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "diagnostics serialized as JSON array to stdout"
)]
fn json_format_outputs_to_stdout() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "JSON output should go to stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");
    assert!(parsed.is_array(), "should be a JSON array");
}

#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "exit code unaffected by format flag"
)]
fn json_format_exit_code_matches_human_format() {
    // With errors: both human and json format should exit 1
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let human_output = specforge_cmd()
        .args(["check", "--format=human"])
        .arg(dir.path())
        .output()
        .unwrap();

    let json_output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert_eq!(
        human_output.status.code(),
        json_output.status.code(),
        "exit code must be same regardless of format flag"
    );
}

#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "Export Diagnostics as JSON: JSON diagnostic export holds — validation_complete_fired, json_array_produced, diagnostic_fields_complete, exit_code_unaffected"
)]
fn json_diagnostics_contract_consistency() {
    // Requires: validation_complete (diagnostics collected)
    // Ensures: JSON array to stdout, complete fields, exit code unaffected
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#,
    )]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    // JSON array to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let diags: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).expect("must produce valid JSON array");
    assert!(!diags.is_empty(), "must contain diagnostics");

    // Complete fields
    for d in &diags {
        assert!(d.get("code").is_some());
        assert!(d.get("severity").is_some());
        assert!(d.get("message").is_some());
    }

    // Exit code reflects severity (errors present → exit 1)
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn json_diagnostics_include_suggestion_when_available() {
    let dir = setup_project(&[(
        "main.spec",
        r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#,
    )]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let diags: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).expect("must produce valid JSON");

    let has_suggestion = diags.iter().any(|d| d.get("suggestion").is_some());
    assert!(
        has_suggestion,
        "at least one diagnostic should have a suggestion: {:?}",
        diags
    );
}

// === typed format flags (C14-10) ===

#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "a typo'd --format fails with a clap error (exit 2), not a bespoke runtime error"
)]
fn unknown_format_value_is_rejected_by_clap() {
    let dir = setup_project(&[("main.spec", r#"behavior alpha "A" { contract "first" }"#)]);

    let output = specforge_cmd()
        .args(["stats", "--format=yaml"])
        .arg(dir.path())
        .env("COLUMNS", "100")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "clap rejects unknown values with exit code 2"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid value"),
        "expected a clap parse error, got: {stderr}"
    );
}
