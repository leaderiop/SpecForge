use specforge_test::registry::{TestOutcome, TestRecordEntry};
use specforge_test::report;
use std::fs;

fn entry(id: &str, outcome: TestOutcome) -> TestRecordEntry {
    TestRecordEntry {
        entity_kind: "behavior".to_string(),
        entity_id: id.to_string(),
        test_name: format!("test_{id}"),
        file: "test.rs".to_string(),
        verify: None,
        outcome,
    }
}

#[test]
fn writes_report_json() {
    let dir = tempfile::tempdir().unwrap();
    let entries = vec![
        entry("create_user", TestOutcome::Pass),
        entry("delete_user", TestOutcome::Fail),
    ];

    report::write_report(dir.path(), "my_binary", &entries).unwrap();

    let path = dir.path().join("my_binary.json");
    assert!(path.exists());

    let content: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(content["schema_version"], "1.0");
    assert_eq!(content["binary_name"], "my_binary");
    assert_eq!(content["entries"].as_array().unwrap().len(), 2);
    // Entries are sorted by entity_id
    assert_eq!(content["entries"][0]["entity_id"], "create_user");
    assert_eq!(content["entries"][0]["status"], "pass");
    assert_eq!(content["entries"][1]["entity_id"], "delete_user");
    assert_eq!(content["entries"][1]["status"], "fail");
}

#[test]
fn empty_entries_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();

    report::write_report(dir.path(), "empty_binary", &[]).unwrap();

    let path = dir.path().join("empty_binary.json");
    assert!(!path.exists());
}

#[test]
fn entries_are_sorted_in_report() {
    let dir = tempfile::tempdir().unwrap();
    // Write entries in reverse order
    let entries = vec![
        entry("zebra", TestOutcome::Pass),
        entry("alpha", TestOutcome::Pass),
        entry("middle", TestOutcome::Fail),
    ];

    report::write_report(dir.path(), "sorted_binary", &entries).unwrap();

    let path = dir.path().join("sorted_binary.json");
    let content: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let ids: Vec<&str> = content["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["alpha", "middle", "zebra"]);
}
