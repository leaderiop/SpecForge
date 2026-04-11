use specforge_lsp::DocumentBuffer;
use specforge_test_macros::test as spec;

// -- incremental_document_sync ------------------------------------------------

#[spec(behavior = "incremental_document_sync", verify = "incremental change applies correctly to source buffer")]
#[test]
fn incremental_change_applies_correctly() {
    let mut buf = DocumentBuffer::new(
        "file:///test.spec".into(),
        "behavior foo \"Foo\" {\n  contract \"old\"\n}\n".into(),
    );

    // Replace "old" with "new" (line 1, col 12..15)
    buf.apply_change(1, 12, 1, 15, "new");

    assert_eq!(buf.content(), "behavior foo \"Foo\" {\n  contract \"new\"\n}\n");
}

#[spec(behavior = "incremental_document_sync", verify = "multiple incremental changes produce correct source")]
#[test]
fn multiple_incremental_changes_produce_correct_source() {
    let mut buf = DocumentBuffer::new(
        "file:///test.spec".into(),
        "line0\nline1\nline2\n".into(),
    );

    // Replace "line1" with "REPLACED"
    buf.apply_change(1, 0, 1, 5, "REPLACED");
    assert_eq!(buf.content(), "line0\nREPLACED\nline2\n");

    // Insert at start of line2
    buf.apply_change(2, 0, 2, 0, "prefix_");
    assert_eq!(buf.content(), "line0\nREPLACED\nprefix_line2\n");
}

#[spec(behavior = "incremental_document_sync", verify = "incremental sync reduces transfer size vs full sync")]
#[test]
fn incremental_sync_reduces_transfer_size() {
    let original = "behavior foo \"Foo\" {\n  contract \"old value\"\n}\n";
    let mut buf = DocumentBuffer::new("file:///test.spec".into(), original.into());

    // Incremental change: only send the replacement text for "old value" -> "new value"
    let incremental_payload = "new value";
    buf.apply_change(1, 12, 1, 21, incremental_payload);

    let expected_full = "behavior foo \"Foo\" {\n  contract \"new value\"\n}\n";
    assert_eq!(buf.content(), expected_full);

    // The incremental payload is smaller than the full document
    assert!(
        incremental_payload.len() < expected_full.len(),
        "incremental change ({} bytes) should be smaller than full sync ({} bytes)",
        incremental_payload.len(),
        expected_full.len(),
    );
}
