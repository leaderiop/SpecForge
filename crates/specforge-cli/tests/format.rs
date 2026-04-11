use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use specforge_test_macros::test as specforge_test;

fn setup_project(dir: &std::path::Path) {
    fs::write(dir.join("specforge.json"), r#"{"name":"test","version":"0.1.0"}"#).unwrap();
    let spec_dir = dir.join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
}

fn write_spec(dir: &std::path::Path, name: &str, content: &str) {
    let spec_dir = dir.join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join(name), content).unwrap();
}

// --- Slice 8: format_spec_files ---

#[specforge_test(behavior = "format_spec_files", verify = "formatting all files in spec/ directory succeeds")]
#[test]
fn format_command_formats_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n      contract \"does stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let formatted = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert!(formatted.contains("  contract \"does stuff\""), "should be properly indented: {formatted}");
}

#[specforge_test(behavior = "format_spec_files", verify = "files matching the canonical format are not rewritten")]
#[test]
fn format_does_not_rewrite_unchanged_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    // Already formatted content
    let content = "behavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n";
    write_spec(root, "test.spec", content);

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("0 changed"));
}

#[specforge_test(behavior = "format_spec_files", verify = "changed files are printed to stdout")]
#[test]
fn format_prints_changed_file_names() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "bad.spec", "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("bad.spec"));
}

// --- Slice 9: check_formatting ---

#[specforge_test(behavior = "check_formatting", verify = "already formatted files exit with code 0")]
#[test]
fn check_already_formatted_exits_zero() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n  contract \"does stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--check", "--path", root.to_str().unwrap()])
        .assert()
        .success();
}

#[specforge_test(behavior = "check_formatting", verify = "unformatted files exit with code 1")]
#[test]
fn check_unformatted_exits_one() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--check", "--path", root.to_str().unwrap()])
        .assert()
        .code(1);
}

#[specforge_test(behavior = "check_formatting", verify = "check mode writes no files to disk")]
#[test]
fn check_mode_writes_no_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    let original = "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--check", "--path", root.to_str().unwrap()])
        .assert()
        .code(1);

    let after = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(after, original, "check mode should not modify files");
}

// --- Slice 10: show_formatting_diff ---

#[specforge_test(behavior = "show_formatting_diff", verify = "diff output uses unified format")]
#[test]
fn diff_shows_unified_format() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--diff", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("+++"))
        .stdout(predicate::str::contains("@@"));
}

#[specforge_test(behavior = "show_formatting_diff", verify = "diff mode writes no files to disk")]
#[test]
fn diff_mode_writes_no_files() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    let original = "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--diff", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    let after = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(after, original, "diff mode should not modify files");
}

#[specforge_test(behavior = "show_formatting_diff", verify = "unchanged files produce no diff output")]
#[test]
fn diff_unchanged_produces_no_output() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--diff", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().or(predicate::str::contains("---").not()));
}

// --- Slice 11: format_from_stdin ---

#[specforge_test(behavior = "format_from_stdin", verify = "stdin content is formatted and written to stdout")]
#[test]
fn stdin_formats_and_writes_to_stdout() {
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--stdin"])
        .write_stdin("behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("  contract \"stuff\""));
}

#[specforge_test(behavior = "format_from_stdin", verify = "stdin mode does not read or write files")]
#[test]
fn stdin_mode_does_not_read_files() {
    // stdin mode should work even without a project
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--stdin"])
        .write_stdin("behavior foo \"Foo\" {\n  contract \"stuff\"\n}\n")
        .assert()
        .success();
}

// --- Integration: formatting all files in spec/ directory ---

#[specforge_test(behavior = "format_spec_files", verify = "formatting all files in spec/ directory succeeds")]
#[test]
fn format_integration_all_spec_files_in_directory() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // Create multiple spec files in subdirectories
    let behaviors_dir = root.join("spec").join("behaviors");
    let types_dir = root.join("spec").join("types");
    fs::create_dir_all(&behaviors_dir).unwrap();
    fs::create_dir_all(&types_dir).unwrap();

    fs::write(
        behaviors_dir.join("auth.spec"),
        "behavior login \"Login\" {\n      contract \"authenticates user\"\n}\n",
    ).unwrap();
    fs::write(
        types_dir.join("core.spec"),
        "type user \"User\" {\n      name \"string\"\n}\n",
    ).unwrap();
    fs::write(
        root.join("spec").join("main.spec"),
        "use behaviors/auth\nuse types/core\n",
    ).unwrap();

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success();

    // Verify all files were formatted
    let auth = fs::read_to_string(behaviors_dir.join("auth.spec")).unwrap();
    assert!(auth.contains("  contract \"authenticates user\""), "auth.spec should be formatted: {auth}");
}

// --- Property: stdin formatting is idempotent ---

#[specforge_test(behavior = "format_from_stdin", verify = "stdin formatting is idempotent")]
#[test]
fn stdin_formatting_is_idempotent() {
    let input = "behavior foo \"Foo\" {\n      contract   \"stuff\"\n    types [a, b]\n}\n";

    let first = Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--stdin"])
        .write_stdin(input)
        .output()
        .unwrap();
    let first_output = String::from_utf8(first.stdout).unwrap();

    let second = Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--stdin"])
        .write_stdin(first_output.as_str())
        .output()
        .unwrap();
    let second_output = String::from_utf8(second.stdout).unwrap();

    assert_eq!(first_output, second_output, "stdin formatting should be idempotent");
}

// --- Property: stdin formatting converges to canonical form ---

#[specforge_test(behavior = "format_from_stdin", verify = "stdin formatting converges to canonical form")]
#[test]
fn stdin_formatting_converges_to_canonical_form() {
    let variants = [
        "behavior foo \"Foo\" {\n      contract   \"stuff\"\n}\n",
        "behavior foo \"Foo\" {\n\tcontract \"stuff\"\n}\n",
        "behavior foo \"Foo\" {\n    contract     \"stuff\"\n}\n",
    ];

    let mut outputs = Vec::new();
    for variant in &variants {
        let result = Command::cargo_bin("specforge")
            .unwrap()
            .args(["format", "--stdin"])
            .write_stdin(*variant)
            .output()
            .unwrap();
        outputs.push(String::from_utf8(result.stdout).unwrap());
    }

    assert_eq!(outputs[0], outputs[1], "variant 0 vs 1 should converge");
    assert_eq!(outputs[1], outputs[2], "variant 1 vs 2 should converge");
}

// --- Contract: format_spec_files ---

#[specforge_test(behavior = "format_spec_files", verify = "requires/ensures consistency for spec file formatting")]
#[test]
fn format_spec_files_contract_requires_ensures() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    // Requires: spec files available, format config loaded
    write_spec(root, "test.spec", "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n");

    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("test.spec")); // ensures: summary_printed

    // ensures: formatted_output_written
    let content = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert!(content.contains("  contract \"stuff\""), "formatted output should be written");

    // ensures: unchanged_files_preserved (re-run should show 0 changed)
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("0 changed"));
}

// --- Contract: check_formatting ---

#[specforge_test(behavior = "check_formatting", verify = "requires/ensures consistency for formatting check")]
#[test]
fn check_formatting_contract_requires_ensures() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);

    // Unformatted file
    write_spec(root, "bad.spec", "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n");

    // ensures: no_files_written, exit_code_correct, unformatted_paths_printed
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--check", "--path", root.to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("bad.spec"));

    // Verify file was NOT modified
    let content = fs::read_to_string(root.join("spec/bad.spec")).unwrap();
    assert!(content.contains("      contract"), "check mode should not modify files");
}

// --- Contract: show_formatting_diff ---

#[specforge_test(behavior = "show_formatting_diff", verify = "requires/ensures consistency for formatting diff")]
#[test]
fn show_formatting_diff_contract_requires_ensures() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    setup_project(root);
    let original = "behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n";
    write_spec(root, "test.spec", original);

    // ensures: no_files_written, unified_diff_produced
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--diff", "--path", root.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("+++"));

    let after = fs::read_to_string(root.join("spec/test.spec")).unwrap();
    assert_eq!(after, original, "diff mode should not modify files");
}

// --- Contract: format_from_stdin ---

#[specforge_test(behavior = "format_from_stdin", verify = "requires/ensures consistency for stdin formatting")]
#[test]
fn format_from_stdin_contract_requires_ensures() {
    // ensures: stdout_produced, no_files_touched
    Command::cargo_bin("specforge")
        .unwrap()
        .args(["format", "--stdin"])
        .write_stdin("behavior foo \"Foo\" {\n      contract \"stuff\"\n}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("  contract \"stuff\""));
}
