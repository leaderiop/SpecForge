use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[allow(deprecated)]
fn specforge_cmd() -> Command {
    Command::cargo_bin("specforge").unwrap()
}

// B:collect_cli_command — verify unit "specforge collect --help exits 0"
#[test]
fn test_collect_help_exits_0() {
    specforge_cmd()
        .args(["collect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Collect test results"));
}

// B:collect_cli_command — verify unit "specforge collect in empty dir exits 1"
#[test]
fn test_collect_in_empty_dir_exits_1() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .args(["collect", "--path", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no specforge project found"));
}

// B:collect_cli_command — verify unit "specforge collect --collector rust with project"
#[test]
fn test_collect_with_explicit_collector() {
    let dir = TempDir::new().unwrap();
    // Create specforge.json so project is found
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"test","version":"0.1.0"}"#,
    )
    .unwrap();

    // A report is required: collect without one errors (E045).
    specforge_cmd()
        .args([
            "collect",
            "--path",
            dir.path().to_str().unwrap(),
            "--collector",
            "rust",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no report files found"));

    // With a report, collect succeeds and prints the collected summary.
    std::fs::write(
        dir.path().join("runner-report.json"),
        r#"{"entity_results":[]}"#,
    )
    .unwrap();
    specforge_cmd()
        .args([
            "collect",
            "--path",
            dir.path().to_str().unwrap(),
            "--collector",
            "rust",
            "--report",
            dir.path().join("runner-report.json").to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("collected 1 report file(s)"));
}

// B:collect_cli_command — verify unit "specforge collect without collector auto-detects"
#[test]
fn test_collect_without_collector_no_auto_detect() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"test","version":"0.1.0"}"#,
    )
    .unwrap();

    // No reports found (and none passed) → E045 error.
    specforge_cmd()
        .args(["collect", "--path", dir.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no report files found"));
}

// B:collect_cli_command — verify unit "specforge collect json format"
#[test]
fn test_collect_json_format_in_empty_dir() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .args([
            "collect",
            "--path",
            dir.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"error\""));
}
