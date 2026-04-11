use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn e2e_goto_definition_entity() {
    let text = "type token \"Token\" {}\nbehavior login \"Login\" {\n  types [token]\n}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // "token" on line 2, col ~10 (inside [token])
    let resp = client.goto_definition(&uri, 2, 10).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected definition result");
    // Should point to line 0 (0-based) where "type token" is defined
    assert_eq!(result["range"]["start"]["line"], 0);
}

#[tokio::test]
async fn e2e_goto_definition_nonexistent() {
    let text = "behavior foo \"Foo\" {}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Position past end of meaningful content
    let resp = client.goto_definition(&uri, 0, 50).await;
    let result = &resp["result"];
    assert!(result.is_null(), "Expected null for nonexistent position");
}

#[tokio::test]
async fn e2e_goto_definition_cross_file() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.spec");
    let file_b = dir.path().join("b.spec");
    std::fs::write(&file_b, "type token \"Token\" {}\n").unwrap();
    std::fs::write(
        &file_a,
        "behavior login \"Login\" {\n  types [token]\n}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;

    // Wait for workspace indexing log
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    let uri_a = tower_lsp::lsp_types::Url::from_file_path(&file_a)
        .unwrap()
        .to_string();
    let text_a = std::fs::read_to_string(&file_a).unwrap();
    client.did_open(&uri_a, "specforge", &text_a).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // goto_definition on "token" in file A should point to file B
    let resp = client.goto_definition(&uri_a, 1, 10).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected cross-file definition");
    let target_uri = result["uri"].as_str().unwrap();
    assert!(
        target_uri.contains("b.spec"),
        "Expected definition in b.spec, got: {target_uri}"
    );
}

#[tokio::test]
async fn e2e_goto_definition_on_use_line() {
    let dir = TempDir::new().unwrap();
    let types_dir = dir.path().join("types");
    std::fs::create_dir_all(&types_dir).unwrap();
    std::fs::write(types_dir.join("core.spec"), "type token \"Token\" {}\n").unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    let main_file = dir.path().join("main.spec");
    let main_uri = tower_lsp::lsp_types::Url::from_file_path(&main_file)
        .unwrap()
        .to_string();
    client
        .did_open(&main_uri, "specforge", "use \"types/core\"\n")
        .await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Cursor on the import path (line 0, col 6 = inside "types/core")
    let resp = client.goto_definition(&main_uri, 0, 7).await;
    let result = &resp["result"];
    assert!(
        !result.is_null(),
        "Expected definition for use import"
    );
    let target_uri = result["uri"].as_str().unwrap();
    assert!(
        target_uri.contains("core.spec"),
        "Expected definition pointing to types/core.spec, got: {target_uri}"
    );
}

#[tokio::test]
async fn e2e_references_returns_all_sites() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "behavior a \"A\" {\n",
        "  types [token]\n",
        "}\n",
        "behavior b \"B\" {\n",
        "  types [token]\n",
        "}\n",
    );
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // "token" at line 0, col 6 (the declaration)
    let resp = client.references(&uri, 0, 6).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected references result");
    let refs = result.as_array().unwrap();
    // declaration + 2 references = 3
    assert!(
        refs.len() >= 3,
        "Expected at least 3 reference locations, got {}",
        refs.len()
    );
}

#[tokio::test]
async fn e2e_references_includes_declaration() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "behavior a \"A\" {\n",
        "  types [token]\n",
        "}\n",
    );
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    let resp = client.references(&uri, 0, 6).await;
    let result = &resp["result"];
    assert!(!result.is_null());
    let refs = result.as_array().unwrap();
    // At least one reference should be at line 0 (the declaration)
    let has_decl = refs
        .iter()
        .any(|r| r["range"]["start"]["line"].as_u64() == Some(0));
    assert!(has_decl, "Expected declaration in references");
}

#[tokio::test]
async fn e2e_references_nonexistent() {
    let text = "behavior foo \"Foo\" {}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Position on a word that isn't an entity ID in the graph references
    let resp = client.references(&uri, 0, 50).await;
    let result = &resp["result"];
    assert!(result.is_null(), "Expected null for nonexistent references");
}
