//! C11-01 + C11-02 acceptance: `analyze coverage --min` gates the exit code,
//! and orphaned test records surface a W097 instead of dropping silently.

use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn specforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specforge"))
}

fn seed(path: &Path) {
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(
        path.join("specforge.json"),
        r#"{"name":"cov","spec_root":"src","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    std::fs::write(
        path.join("src/a.spec"),
        "type widget \"Widget\" {\n  id string @unique\n  verify unit \"widget valid\"\n}\n",
    )
    .unwrap();
}

fn report(path: &Path, proven: bool, extra: Option<&str>) -> String {
    let status = if proven { "pass" } else { "fail" };
    let mut json = format!(
        r#"{{"runner":"specforge-test","results":{{"widget":{{"tests":[{{"name":"w test","status":"{status}"}}]}}}}}}"#
    );
    if let Some(orphan) = extra {
        json = json.trim_end_matches('}').to_string();
        json.push_str(
            format!(r#", "{orphan}": {{"tests":[{{"name":"orphan test","status":"pass"}}]}}}}}}"#)
                .as_str(),
        );
    }
    std::fs::write(path.join("specforge-report.json"), &json).unwrap();
    json
}

#[specforge_test(
    behavior = "te_coverage_gate",
    verify = "coverage at or above the threshold passes"
)]
fn min_gate_passes_when_coverage_meets_threshold() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    report(tmp.path(), true, None);
    let out = specforge()
        .args([
            "analyze",
            "--path",
            tmp.path().to_str().unwrap(),
            "coverage",
            "--test-results",
            tmp.path().join("specforge-report.json").to_str().unwrap(),
            "--min",
            "50",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[specforge_test(
    behavior = "te_coverage_gate",
    verify = "coverage below the threshold fails with E048"
)]
fn min_gate_fails_when_coverage_below_threshold() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    // 1 of 1 entities proven = 100%; make it fail by NOT proving: failing test
    let report = r#"{"runner":"r","results":{"widget":{"tests":[{"name":"w","status":"fail"}]}}}"#;
    std::fs::write(tmp.path().join("specforge-report.json"), report).unwrap();
    let out = specforge()
        .args([
            "analyze",
            "--path",
            tmp.path().to_str().unwrap(),
            "coverage",
            "--test-results",
            tmp.path().join("specforge-report.json").to_str().unwrap(),
            "--min",
            "50",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "gate must fail below threshold");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("E048"), "gate names E048: {stderr}");
    assert!(stderr.contains("below the required minimum"), "{stderr}");
}

#[specforge_test(behavior = "te_coverage_gate", verify = "the gate needs test results")]
fn min_requires_test_results() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    let out = specforge()
        .args([
            "analyze",
            "--path",
            tmp.path().to_str().unwrap(),
            "coverage",
            "--min",
            "50",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--min needs test results"), "{stderr}");
}

#[test]
fn orphaned_test_records_warn_with_suggestion() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    // Report proves "widget" and orphans "wodget" (typo of widget).
    let report = r#"{"runner":"r","results":{"widget":{"tests":[{"name":"w","status":"pass"}]},"wodget":{"tests":[{"name":"x","status":"pass"}]}}}"#;
    std::fs::write(tmp.path().join("specforge-report.json"), report).unwrap();
    let out = specforge()
        .args([
            "analyze",
            "--path",
            tmp.path().to_str().unwrap(),
            "coverage",
            "--test-results",
            tmp.path().join("specforge-report.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("W097"), "orphan warning emitted: {stderr}");
    assert!(stderr.contains("wodget"), "names the orphaned id: {stderr}");
    assert!(
        stderr.contains("widget") && stderr.contains("did you mean"),
        "suggests the close match: {stderr}"
    );
    // Warnings do not fail the run.
    assert!(out.status.success(), "orphan warnings don't gate: {stderr}");
}

// C1-06 rot guard: the flagship example's traceability loop must keep
// working — collect from the committed runner report, analyze, and trace.
#[test]
fn todo_app_traceability_loop_stays_wired() {
    // Flat-file copy of the example (spec/, specforge.json, fixture report),
    // skipping build dirs.
    fn walk(from: &Path, to: &Path) -> Vec<(PathBuf, PathBuf)> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            let path = e.path();
            let dest = to.join(e.file_name());
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|n| n == "target" || n == "tests")
                {
                    continue;
                }
                out.extend(walk(&path, &dest));
            } else if path.is_file() {
                out.push((path, dest));
            }
        }
        out
    }

    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/todo-app");
    let tmp = TempDir::new().unwrap();
    // Copy the project (spec + config + committed fixture) so collect writes
    // its report into the temp copy, not the repo.
    let copy = |from: &Path, to: &Path| {
        for e in walk(from, to) {
            std::fs::create_dir_all(e.1.parent().unwrap()).unwrap();
            std::fs::copy(&e.0, &e.1).unwrap();
        }
    };
    copy(&example, tmp.path());

    let specforge = || Command::new(env!("CARGO_BIN_EXE_specforge"));

    // collect the committed fixture report
    let out = specforge()
        .args([
            "collect",
            "--path",
            tmp.path().to_str().unwrap(),
            "--report",
            example.join("runner-report.fixture.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "collect: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // analyze proves the recorded entities
    let out = specforge()
        .args([
            "analyze",
            "--path",
            tmp.path().to_str().unwrap(),
            "coverage",
            "--test-results",
            tmp.path().join("specforge-report.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "analyze: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"entities_proven\":5"),
        "the fixture proves 5 entities: {stdout}"
    );

    // trace surfaces provenance for a proven behavior
    let out = specforge()
        .args([
            "trace",
            "--path",
            tmp.path().to_str().unwrap(),
            "create_task",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "trace failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("create_task"), "trace names the entity");
}
