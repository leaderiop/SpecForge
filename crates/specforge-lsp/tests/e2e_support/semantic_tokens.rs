use super::*;

#[tokio::test]
async fn e2e_semantic_tokens_non_empty() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.semantic_tokens_full(&uri).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected semantic tokens result");
    let data = result["data"].as_array().unwrap();
    assert!(
        !data.is_empty(),
        "Expected non-empty semantic tokens data"
    );
}

#[tokio::test]
async fn e2e_semantic_tokens_delta_encoded() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.semantic_tokens_full(&uri).await;
    let data = resp["result"]["data"].as_array().unwrap();
    // Semantic tokens are encoded as groups of 5 integers:
    // [deltaLine, deltaStart, length, tokenType, tokenModifiers]
    assert!(
        data.len() % 5 == 0,
        "Token data length should be multiple of 5"
    );
    // First token's deltaLine must parse as u64 (non-negative by type)
    if !data.is_empty() {
        data[0].as_u64().expect("deltaLine should be a non-negative integer");
    }
    // All delta values should be non-negative (they're unsigned in the protocol)
    for chunk in data.chunks(5) {
        let delta_line = chunk[0].as_u64();
        let delta_start = chunk[1].as_u64();
        assert!(delta_line.is_some(), "deltaLine should be a number");
        assert!(delta_start.is_some(), "deltaStart should be a number");
    }
}

#[tokio::test]
async fn e2e_semantic_tokens_keyword_type() {
    let text = "behavior foo \"Foo\" {}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.semantic_tokens_full(&uri).await;
    let data = resp["result"]["data"].as_array().unwrap();
    // First token should be "behavior" entity kind at line 0, col 0
    // tokenType index 1 = type (entity kind keywords)
    assert!(data.len() >= 5, "Expected at least one token");
    let first_token_type = data[3].as_u64().unwrap();
    assert_eq!(
        first_token_type, 1,
        "First token should be type (entity kind, index 1)"
    );
}
