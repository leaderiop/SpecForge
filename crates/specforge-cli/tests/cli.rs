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

#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode works in CI environment")]
#[test]
fn self_check_runs_without_crashing() {
    // Find the project's spec/ directory relative to the crate manifest
    let spec_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap() // crates/
        .parent().unwrap() // project root
        .join("spec");

    if !spec_dir.exists() {
        panic!("spec/ directory not found at {:?}", spec_dir);
    }

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(&spec_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("self-check produced invalid JSON: {}\noutput: {}", e, stdout));

    // The spec files currently have known issues (circular imports, cross-file refs).
    // This test ensures the check pipeline doesn't panic on a real 136-file project.
    // Goal: reduce this to zero errors as import handling improves.
    assert!(
        diagnostics.iter().all(|d| {
            let code = d["code"].as_str().unwrap_or("");
            ["E001", "E002", "E003", "W003", "W012", "W060", "W061", "W062"].contains(&code)
        }),
        "self-check should only produce known diagnostic codes"
    );
}

// === check_mode_for_ci ===

#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 0 with no errors")]
#[test]
fn check_clean_project_exits_zero() {
    let dir = setup_project(&[
        ("main.spec", r#"behavior alpha "A" { contract "first" }"#),
    ]);

    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success();
}

// === exit_code_reflects_diagnostic_severity ===

#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 1 with errors")]
#[test]
fn check_project_with_errors_exits_one() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .assert()
        .code(1);
}

#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "exit 1 with warnings in strict mode")]
#[test]
fn strict_mode_promotes_warnings_to_errors() {
    // Orphan ref produces W012 (warning) — with --strict it becomes an error → exit 1
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Orphan ref"
"#),
    ]);

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

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "JSON output is valid and parseable")]
#[test]
fn json_format_outputs_valid_json() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    assert!(parsed.is_array(), "JSON output should be an array of diagnostics");
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty(), "should have at least one diagnostic");

    let first = &arr[0];
    assert!(first.get("code").is_some(), "diagnostic should have 'code' field");
    assert!(first.get("severity").is_some(), "diagnostic should have 'severity' field");
    assert!(first.get("message").is_some(), "diagnostic should have 'message' field");
}

// === print_diagnostics_structured ===

#[specforge_test(behavior = "print_diagnostics_structured", verify = "error diagnostic is formatted with file:line:col")]
#[test]
fn structured_output_includes_file_line_col() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main.spec"), "should contain filename in stderr: {}", stderr);
}

#[specforge_test(behavior = "print_diagnostics_structured", verify = "diagnostic includes context snippet")]
#[test]
fn structured_output_includes_context_snippet() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("feature gamma") || stderr.contains("nonexistent"),
        "should contain source context snippet in stderr: {}",
        stderr
    );
}

#[specforge_test(behavior = "print_diagnostics_structured", verify = "suggestion is displayed when available")]
#[test]
fn structured_output_includes_suggestion() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#),
    ]);

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

#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode produces no output files")]
#[test]
fn check_mode_produces_no_output_files() {
    let dir = setup_project(&[
        ("main.spec", r#"behavior alpha "A" { contract "first" }"#),
    ]);

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

#[specforge_test(behavior = "check_mode_for_ci", verify = "check mode prints diagnostics to stderr")]
#[test]
fn check_mode_prints_to_stderr() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

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

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "each diagnostic includes code, severity, message, file, line, column")]
#[test]
fn json_diagnostics_have_complete_fields() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

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
        assert!(diag.get("severity").is_some(), "missing 'severity': {:?}", diag);
        assert!(diag.get("message").is_some(), "missing 'message': {:?}", diag);
        if let Some(span) = diag.get("span") {
            assert!(span.get("file").is_some(), "span missing 'file': {:?}", span);
            assert!(span.get("start_line").is_some(), "span missing 'start_line': {:?}", span);
            assert!(span.get("start_col").is_some(), "span missing 'start_col': {:?}", span);
        }
    }
}

// === contract tests ===

#[specforge_test(behavior = "print_diagnostics_structured", verify = "requires/ensures consistency for structured diagnostic printing")]
#[test]
fn print_diagnostics_contract_consistency() {
    // Requires: validation_complete fired (diagnostics collected)
    // Ensures: structured format with file:line:col, color-coded severity
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Structured format: file path + error code + source context
    assert!(stderr.contains("main.spec"), "must include file path");
    assert!(stderr.contains("E003"), "must include error code");
}

#[specforge_test(behavior = "exit_code_reflects_diagnostic_severity", verify = "requires/ensures consistency for exit code severity mapping")]
#[test]
fn exit_code_contract_consistency() {
    // Requires: validation_complete fired
    // Ensures: exit 0 on clean, exit 1 on errors, strict promotes warnings
    let clean_dir = setup_project(&[
        ("main.spec", r#"behavior alpha "A" { contract "first" }"#),
    ]);
    specforge_cmd()
        .arg("check")
        .arg(clean_dir.path())
        .assert()
        .success();

    let error_dir = setup_project(&[
        ("main.spec", r#"feature g "G" { behaviors [nonexistent] }"#),
    ]);
    specforge_cmd()
        .arg("check")
        .arg(error_dir.path())
        .assert()
        .code(1);
}

#[specforge_test(behavior = "check_mode_for_ci", verify = "requires/ensures consistency for CI check mode")]
#[test]
fn check_mode_contract_consistency() {
    // Requires: validation_complete fired
    // Ensures: no output files, diagnostics to stderr, appropriate exit code
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let before_count = fs::read_dir(dir.path()).unwrap().count();

    let output = specforge_cmd()
        .arg("check")
        .arg(dir.path())
        .output()
        .unwrap();

    let after_count = fs::read_dir(dir.path()).unwrap().count();
    assert_eq!(before_count, after_count, "check must not produce output files");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "diagnostics must go to stderr");

    assert_eq!(output.status.code(), Some(1), "errors must cause exit 1");
}

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "diagnostics serialized as JSON array to stdout")]
#[test]
fn json_format_outputs_to_stdout() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .arg("check")
        .arg("--format=json")
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty(), "JSON output should go to stdout");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout should be valid JSON");
    assert!(parsed.is_array(), "should be a JSON array");
}

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "exit code unaffected by format flag")]
#[test]
fn json_format_exit_code_matches_human_format() {
    // With errors: both human and json format should exit 1
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

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

    assert_eq!(human_output.status.code(), json_output.status.code(),
        "exit code must be same regardless of format flag");
}

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "requires/ensures consistency for JSON diagnostic export")]
#[test]
fn json_diagnostics_contract_consistency() {
    // Requires: validation_complete (diagnostics collected)
    // Ensures: JSON array to stdout, complete fields, exit code unaffected
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    // JSON array to stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let diags: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("must produce valid JSON array");
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

#[specforge_test(behavior = "export_diagnostics_as_json", verify = "suggestion field included when available")]
#[test]
fn json_diagnostics_include_suggestion_when_available() {
    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#),
    ]);

    let output = specforge_cmd()
        .args(["check", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let diags: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .expect("must produce valid JSON");

    let has_suggestion = diags.iter().any(|d| d.get("suggestion").is_some());
    assert!(has_suggestion, "at least one diagnostic should have a suggestion: {:?}", diags);
}
