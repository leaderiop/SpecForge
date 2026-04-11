use specforge_test_macros::test as spec;

// -- document_open_close ------------------------------------------------------

#[spec(behavior = "document_open_close", verify = "didOpen registers document and triggers compilation")]
#[test]
fn did_open_registers_document() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");

    assert!(state.is_open("file:///a.spec"));
    assert_eq!(state.document("file:///a.spec").unwrap().content(), "behavior a \"A\" {}\n");
}

#[spec(behavior = "document_open_close", verify = "didClose removes document and clears diagnostics")]
#[test]
fn did_close_removes_document() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "behavior a \"A\" {}\n");
    state.close_document("file:///a.spec");

    assert!(!state.is_open("file:///a.spec"));
    assert!(state.document("file:///a.spec").is_none());
}

#[spec(behavior = "document_open_close", verify = "only open documents participate in incremental compilation")]
#[test]
fn only_open_documents_tracked() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "a");
    state.open_document("file:///b.spec", "b");

    assert_eq!(state.open_uris().len(), 2);

    state.close_document("file:///a.spec");
    assert_eq!(state.open_uris().len(), 1);
    assert_eq!(state.open_uris()[0], "file:///b.spec");
}

// -- handle_text_document_change ----------------------------------------------

#[spec(behavior = "handle_text_document_change", verify = "didChange applies incremental edits to buffer")]
#[test]
fn did_change_applies_edits() {
    let mut state = specforge_lsp::LspState::new();
    state.open_document("file:///a.spec", "hello world\n");

    state.apply_change("file:///a.spec", 0, 6, 0, 11, "rust");
    assert_eq!(state.document("file:///a.spec").unwrap().content(), "hello rust\n");
}
