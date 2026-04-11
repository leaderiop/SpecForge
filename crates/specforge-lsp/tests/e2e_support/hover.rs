use super::*;

#[tokio::test]
async fn e2e_hover_returns_markdown() {
    let text = "behavior user_login \"User Login\" {}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Hover on "user_login" (line 0, col 12)
    let resp = client.hover(&uri, 0, 12).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected hover result");
    assert_eq!(result["contents"]["kind"], "markdown");
    let value = result["contents"]["value"].as_str().unwrap();
    assert!(value.contains("behavior"), "Hover should mention kind");
    assert!(
        value.contains("user_login"),
        "Hover should mention entity ID"
    );
    assert!(
        value.contains("User Login"),
        "Hover should mention title"
    );
}

#[tokio::test]
async fn e2e_hover_includes_reference_count() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "behavior login \"Login\" {\n",
        "  types [token]\n",
        "}\n",
    );
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Hover on "token" at line 0
    let resp = client.hover(&uri, 0, 6).await;
    let result = &resp["result"];
    assert!(!result.is_null());
    let value = result["contents"]["value"].as_str().unwrap();
    assert!(
        value.contains("References:") || value.contains("references:") || value.contains("Referenced"),
        "Expected reference count in hover, got: {value}"
    );
}

#[tokio::test]
async fn e2e_hover_on_empty_returns_null() {
    let text = "behavior foo \"Foo\" {}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Past end of file
    let resp = client.hover(&uri, 5, 0).await;
    let result = &resp["result"];
    assert!(result.is_null(), "Expected null hover past end of file");
}

#[tokio::test]
async fn e2e_hover_on_non_entity_word() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;
    // Hover on "contract" keyword (line 1, col 4)
    let resp = client.hover(&uri, 1, 4).await;
    let result = &resp["result"];
    // "contract" is not an entity ID in the graph, so null
    assert!(
        result.is_null(),
        "Expected null hover for non-entity word 'contract'"
    );
}
