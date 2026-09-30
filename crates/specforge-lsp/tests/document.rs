use specforge_lsp::DocumentBuffer;
use specforge_lsp::backend::word_at_position;
use specforge_test_macros::test as spec;

// -- incremental_document_sync ------------------------------------------------

#[spec(
    behavior = "incremental_document_sync",
    verify = "incremental change applies correctly to source buffer"
)]
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
#[tokio::test]
async fn incremental_sync_reduces_transfer_size() {
    use crate::contracts::wire::Session;
    use serde_json::json;

    let (mut session, init) = Session::start(None).await;
    // The server asks for INCREMENTAL sync: changes, not whole documents.
    assert_eq!(init["capabilities"]["textDocumentSync"], 2);

    let uri = "file:///buffer/sync.spec";
    let padding = "  // a long comment the edit never touches\n".repeat(40);
    let text = format!(
        "behavior login \"Login\" {{\n{padding}  invariants [session_limit]\n}}\n\n\
         invariant session_limit \"Limit\" {{\n}}\n"
    );
    session.open(uri, &text).await;
    assert!(session.diagnostics(uri).await.is_empty());

    // Rename the invariant: only the changed range travels.
    let line = text
        .lines()
        .position(|l| l.starts_with("invariant"))
        .unwrap() as u32;
    let change = json!({"range": {
        "start": {"line": line, "character": 10},
        "end": {"line": line, "character": 23},
    }, "text": "quota"});
    let full = json!({"text": text.replacen("invariant session_limit", "invariant quota", 1)});
    let sent = change.to_string().len();
    assert!(
        sent * 10 < full.to_string().len(),
        "{sent} bytes vs {} for full sync",
        full.to_string().len()
    );
    session
        .notify(
            "textDocument/didChange",
            json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [change]}),
        )
        .await;

    // The server rebuilt the whole document from the range: the reference
    // is now dangling and the outline names the renamed invariant.
    let diagnostics = session.diagnostics(uri).await;
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["unresolved reference 'session_limit' in entity 'login'"]
    );
    let outline = session
        .request(
            "textDocument/documentSymbol",
            json!({"textDocument": {"uri": uri}}),
        )
        .await;
    let names = outline["result"].to_string();
    assert!(
        names.contains("quota") && !names.contains("session_limit"),
        "{names}"
    );
}

// -- utf16_positions -----------------------------------------------------------

#[spec(
    behavior = "incremental_document_sync",
    verify = "incremental change applies correctly to source buffer"
)]
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
    invariant = "lsp_utf16_positions",
    verify = "word_at_position extracts words using utf16 columns"
)]
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
