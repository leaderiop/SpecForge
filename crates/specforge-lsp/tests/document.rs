use specforge_lsp::DocumentBuffer;
use specforge_lsp::backend::word_at_position;
use specforge_test_macros::test as spec;

// -- incremental_document_sync ------------------------------------------------

#[spec(
    behavior = "incremental_document_sync",
    verify = "incremental change applies correctly to source buffer"
)]
#[test]
fn incremental_change_applies_correctly() {
    let mut buf = DocumentBuffer::new(
        "file:///test.spec".into(),
        "behavior foo \"Foo\" {\n  contract \"old\"\n}\n".into(),
    );

    // Replace "old" with "new" (line 1, col 12..15)
    buf.apply_change(1, 12, 1, 15, "new");

    assert_eq!(
        buf.content(),
        "behavior foo \"Foo\" {\n  contract \"new\"\n}\n"
    );
}

#[spec(
    behavior = "incremental_document_sync",
    verify = "multiple incremental changes produce correct source"
)]
#[test]
fn multiple_incremental_changes_produce_correct_source() {
    let mut buf = DocumentBuffer::new("file:///test.spec".into(), "line0\nline1\nline2\n".into());

    // Replace "line1" with "REPLACED"
    buf.apply_change(1, 0, 1, 5, "REPLACED");
    assert_eq!(buf.content(), "line0\nREPLACED\nline2\n");

    // Insert at start of line2
    buf.apply_change(2, 0, 2, 0, "prefix_");
    assert_eq!(buf.content(), "line0\nREPLACED\nprefix_line2\n");
}

#[spec(
    behavior = "incremental_document_sync",
    verify = "incremental sync reduces transfer size vs full sync"
)]
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

// -- utf16_positions -----------------------------------------------------------

#[spec(
    behavior = "utf16_positions",
    verify = "utf16 columns resolve to byte offsets across emoji and CJK chars"
)]
#[test]
fn utf16_columns_resolve_to_byte_offsets_after_multibyte_chars() {
    // Line 0 layout: `behavior foo "🚀世" {`
    //   `behavior foo "` = 14 UTF-16 units / 14 bytes
    //   🚀 = 2 UTF-16 units / 4 bytes (units 14..16, bytes 14..18)
    //   世 = 1 UTF-16 unit / 3 bytes (unit 16..17, bytes 18..21)
    //   `"` at unit 17..18 (byte 21..22), `{` at unit 19 (byte 23)
    let mut buf = DocumentBuffer::new(
        "file:///test.spec".into(),
        "behavior foo \"🚀世\" {\n}\n".into(),
    );

    // Replace the emoji via UTF-16 columns 14..16.
    buf.apply_change(0, 14, 0, 16, "🌟");
    assert_eq!(buf.content(), "behavior foo \"🌟世\" {\n}\n");

    // Replace the CJK char via UTF-16 column 16..17 (byte offset must land
    // after the 4-byte emoji, i.e. at byte 18).
    buf.apply_change(0, 16, 0, 17, "界");
    assert_eq!(buf.content(), "behavior foo \"🌟界\" {\n}\n");

    // Column after all multibyte chars: insert at unit 18 = byte 22 (right
    // after the closing quote).
    buf.apply_change(0, 18, 0, 18, " // 🌟");
    assert_eq!(buf.content(), "behavior foo \"🌟界\" // 🌟 {\n}\n");
}

#[spec(
    behavior = "utf16_positions",
    verify = "word_at_position extracts words using utf16 columns"
)]
#[test]
fn word_at_position_handles_utf16_columns() {
    // `contract ` = 9 units/bytes, 🚀 = 2 units/4 bytes, ` ` = 1 unit/byte,
    // `alpha_beta` spans units 12..22 (bytes 14..24).
    let content = "contract 🚀 alpha_beta\n";

    // Column 12 (word start in UTF-16 units) maps to byte 14, not byte 12
    // (which is inside the emoji).
    assert_eq!(
        word_at_position(content, 0, 12),
        Some("alpha_beta".to_string())
    );

    // Column past the word end (unit 22 = byte 24) still scans back to it.
    assert_eq!(
        word_at_position(content, 0, 22),
        Some("alpha_beta".to_string())
    );

    // A column inside the emoji's surrogate pair clamps to the emoji start.
    assert_eq!(word_at_position(content, 0, 10), None);

    // A column beyond the line's UTF-16 length yields no word.
    assert_eq!(word_at_position(content, 0, 23), None);
}
