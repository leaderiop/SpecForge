use super::*;

#[tokio::test]
async fn e2e_completion_entity_ids() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "behavior login \"Login\" {\n",
        "  types [tok]\n",
        "}\n",
    );
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Completion inside ref list at "tok" (line 2, col 11)
    let resp = client.completion(&uri, 2, 11).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected completion result");
    let items = result.as_array().unwrap();
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|i| i["label"].as_str())
        .collect();
    assert!(
        ids.contains(&"token"),
        "Expected 'token' in completions, got: {ids:?}"
    );
}

#[tokio::test]
async fn e2e_completion_entity_with_title() {
    let text = "type token \"Auth Token\" {}\nbehavior b \"B\" {\n  types [tok]\n}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    let resp = client.completion(&uri, 2, 11).await;
    let items = resp["result"].as_array().unwrap();
    let token_item = items.iter().find(|i| i["label"] == "token");
    assert!(token_item.is_some(), "Expected token completion item");
    let detail = token_item.unwrap()["detail"].as_str().unwrap();
    assert!(
        detail.contains("Auth Token"),
        "Expected title in detail, got: {detail}"
    );
}

#[tokio::test]
async fn e2e_completion_keywords_at_top_level() {
    let text = "behavior foo \"Foo\" {}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    // Completion at column 0 (top level, line start)
    let resp = client.completion(&uri, 1, 0).await;
    let result = &resp["result"];
    assert!(!result.is_null());
    let items = result.as_array().unwrap();
    let labels: Vec<&str> = items
        .iter()
        .filter_map(|i| i["label"].as_str())
        .collect();
    assert!(
        labels.contains(&"behavior"),
        "Expected 'behavior' keyword, got: {labels:?}"
    );
    assert!(
        labels.contains(&"type"),
        "Expected 'type' keyword, got: {labels:?}"
    );
}

#[tokio::test]
async fn e2e_completion_no_keywords_inside_block() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Completion inside entity body (line 1, col 5) — character >= 2
    let resp = client.completion(&uri, 1, 5).await;
    let result = &resp["result"];
    if !result.is_null() {
        let items = result.as_array().unwrap();
        let keyword_items: Vec<&Value> = items
            .iter()
            .filter(|i| i["kind"] == 14) // CompletionItemKind::KEYWORD = 14
            .collect();
        assert!(
            keyword_items.is_empty(),
            "Expected no keyword completions inside block"
        );
    }
}
