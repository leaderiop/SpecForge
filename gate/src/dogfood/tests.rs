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
