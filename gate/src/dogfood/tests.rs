#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use super::*;

fn write(root: &Path, file: &str) {
    let path = root.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "<testsuites/>").unwrap();
}

#[test]
fn no_report_directory_finds_no_report() {
    let dir = tempfile::tempdir().unwrap();
    assert!(reports(&dir.path().join("target/nextest/gate")).is_empty());
    assert!(
        reports(dir.path()).is_empty(),
        "an empty directory holds none"
    );
}

#[test]
fn a_local_run_finds_its_one_report() {
    let dir = tempfile::tempdir().unwrap();
    let at = report_dir(dir.path(), "gate");
    write(&at, "junit.xml");
    assert_eq!(reports(&at), [at.join("junit.xml")]);
}

/// CI downloads each test partition's report into its own directory: every one is found, in path order, and an XML
/// file that is no nextest report is left out.
#[test]
fn the_partitions_reports_are_found_in_their_directories_and_other_xml_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let at = report_dir(dir.path(), "ci");
    for file in [
        "junit-tests-2/junit.xml",
        "junit-tests-1/junit.xml",
        "junit-tests-1/other.xml",
        "notes.txt",
    ] {
        write(&at, file);
    }
    assert_eq!(
        reports(&at),
        [
            at.join("junit-tests-1/junit.xml"),
            at.join("junit-tests-2/junit.xml")
        ]
    );
}

/// CI keeps each test partition's report for this check, from where nextest writes it under the `ci` profile. A
/// partition whose report is not there fails its upload: the verdict needs one proven requirement, so a lost
/// partition would otherwise read as `no_test` on what only it proves and pass.
#[test]
fn ci_uploads_each_partitions_report_or_fails() {
    let path = specforge_repo_root::repo_root().join(".github/workflows/ci.yml");
    let workflow = std::fs::read_to_string(&path).unwrap();
    let step: Vec<&str> = workflow
        .lines()
        .map(str::trim)
        .skip_while(|line| !line.starts_with("name: junit-tests-"))
        .take_while(|line| !line.is_empty() && !line.starts_with("- "))
        .collect();
    let report = report_dir(Path::new(""), "ci").join("junit.xml");
    let report = report.to_string_lossy().replace('\\', "/");
    assert!(
        step.contains(&format!("path: {report}").as_str()),
        "{}: the junit-tests upload does not keep {report}: {step:?}",
        path.display()
    );
    assert!(
        step.contains(&"if-no-files-found: error"),
        "{}: the junit-tests upload passes with no report: {step:?}",
        path.display()
    );
}

#[test]
fn the_anchored_entities_are_the_guest_named_set() {
    let anchors = serde_json::json!({"entities": [
        {"entity_id": "FEATURE-001", "kind": "requirement"},
        {"entity_id": "FEATURE-002", "kind": "requirement"},
        {"entity_id": "FEATURE-001", "kind": "requirement"},
    ]});
    assert_eq!(
        anchored(&anchors),
        BTreeSet::from(["FEATURE-001".to_string(), "FEATURE-002".to_string()])
    );
    assert!(anchored(&serde_json::json!({"entities": []})).is_empty());
}

#[test]
fn a_failed_requirement_verdict_still_logs_the_x2_error() {
    let lines = ["2 of 3 proved".to_string(), "R-01 has no test".to_string()];
    let x2_error = || Err("X2 ledger: collecting with rust-test: boom".to_string());
    let (logged, result) = outcome(true, &lines, x2_error());
    assert_eq!(
        logged.as_deref(),
        Some("X2 ledger: collecting with rust-test: boom")
    );
    assert_eq!(result, Err("R-01 has no test".to_string()));
    let (logged, result) = outcome(true, &lines, Ok(()));
    assert_eq!(
        (logged, result),
        (None, Err("R-01 has no test".to_string()))
    );
    // Requirements pass: X2's own outcome is the check's, nothing extra logged.
    let (logged, result) = outcome(false, &lines, x2_error());
    assert_eq!(logged, None);
    assert_eq!(result, x2_error());
    assert_eq!(outcome(false, &lines, Ok(())), (None, Ok(())));
}
