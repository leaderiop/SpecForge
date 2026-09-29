//! `#[ignore]` on a `#[specforge_test]` test is a real ignore: libtest
//! reports it as ignored and nothing is recorded, and `--include-ignored`
//! runs the body and records the result. Runs the `ignore_fixture` test
//! binary, which `cargo test` builds next to this one.

use specforge_test_macros::test as specforge_test;
use std::path::PathBuf;
use std::process::Command;

/// The newest `ignore_fixture-<hash>` binary in this binary's directory.
fn fixture() -> PathBuf {
    let dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("ignore_fixture-")
                && matches!(p.extension().and_then(|e| e.to_str()), None | Some("exe"))
        })
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok())
        .unwrap_or_else(|| {
            panic!(
                "no ignore_fixture binary in {}: run the whole suite (cargo test), not --test ignore_test",
                dir.display()
            )
        })
}

/// Run the fixture with `args`, reports going to a fresh directory.
/// Returns its stdout and the recorded entries.
fn run(args: &[&str]) -> (String, Vec<serde_json::Value>) {
    let reports = tempfile::tempdir().unwrap();
    let out = Command::new(fixture())
        .args(args)
        .env("SPECFORGE_REPORT", reports.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let mut entries = Vec::new();
    for file in std::fs::read_dir(reports.path()).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let report: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            entries.extend(report["entries"].as_array().unwrap().iter().cloned());
        }
    }
    (String::from_utf8(out.stdout).unwrap(), entries)
}

#[specforge_test(
    behavior = "record_test_via_drop_guard",
    verify = "an ignored test runs and is recorded only when libtest is asked to run it"
)]
fn an_ignored_test_is_recorded_only_when_it_runs() {
    let (listed, _) = run(&["--list", "--ignored"]);
    assert!(listed.contains("ignored_by_default: test"), "{listed}");
    assert!(listed.contains("ignored_should_panic: test"), "{listed}");

    let (stdout, entries) = run(&[]);
    assert!(
        stdout.contains("test ignored_by_default ... ignored"),
        "{stdout}"
    );
    assert!(
        stdout.contains("test ignored_should_panic ... ignored"),
        "{stdout}"
    );
    assert!(
        entries.is_empty(),
        "ignored tests record nothing: {entries:?}"
    );

    let (stdout, entries) = run(&["--include-ignored"]);
    assert!(
        stdout.contains("test ignored_by_default ... ok"),
        "{stdout}"
    );
    assert!(
        stdout.contains("test ignored_should_panic - should panic ... ok"),
        "{stdout}"
    );
    let mut recorded: Vec<(String, String)> = entries
        .iter()
        .map(|e| {
            (
                e["test_name"].as_str().unwrap().to_string(),
                e["status"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    recorded.sort();
    assert_eq!(
        recorded,
        vec![
            ("ignored_by_default".to_string(), "pass".to_string()),
            ("ignored_should_panic".to_string(), "pass".to_string()),
        ]
    );
}
