/// Verifies the report write pipeline that atexit invokes.
#[test]
fn atexit_writes_report_on_process_exit() {
    let dir = tempfile::tempdir().unwrap();
    let report_dir = dir.path().join("specforge");
    use specforge_test::registry::{TestOutcome, TestRecordEntry};
    use specforge_test::report;

    let entries = vec![TestRecordEntry {
        entity_kind: "behavior".to_string(),
        entity_id: "test_entity".to_string(),
        test_name: "test_fn".to_string(),
        file: "test.rs".to_string(),
        verify: None,
        outcome: TestOutcome::Pass,
    }];

    report::write_report(&report_dir, "test_binary", &entries).unwrap();

    let path = report_dir.join("test_binary.json");
    assert!(path.exists());

    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(content["schema_version"], "1.0");
    assert_eq!(content["binary_name"], "test_binary");
    assert_eq!(content["entries"][0]["entity_id"], "test_entity");
}
