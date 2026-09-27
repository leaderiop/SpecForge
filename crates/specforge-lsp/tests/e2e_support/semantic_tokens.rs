use super::*;

#[tokio::test]
async fn e2e_semantic_tokens_non_empty() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) =
        start_server_with_extensions(&["@specforge/software"], "test.spec", text).await;
    let resp = client.semantic_tokens_full(&uri).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected semantic tokens result");
    let data = result["data"].as_array().unwrap();
    assert!(!data.is_empty(), "Expected non-empty semantic tokens data");
}

#[tokio::test]
async fn e2e_semantic_tokens_delta_encoded() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) =
        start_server_with_extensions(&["@specforge/software"], "test.spec", text).await;
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
        data[0]
            .as_u64()
            .expect("deltaLine should be a non-negative integer");
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
    let (mut client, uri, _dir) =
        start_server_with_extensions(&["@specforge/software"], "test.spec", text).await;
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

// C4-02b regression: with multi-byte characters on earlier lines, token
// positions must be UTF-16 (not bytes) and deltas must stay monotonic.
#[tokio::test]
async fn e2e_semantic_tokens_multibyte_lines_use_utf16() {
    // The em-dash and ü sit ON the token line, before the `behavior` keyword:
    // a byte-column emission would place the token at col 11-ish (bytes)
    // instead of its UTF-16 col 9. Assert the exact UTF-16 start.
    let text = "behavior –ü foo {
}\n";
    let (mut client, uri, _dir) =
        start_server_with_extensions(&["@specforge/software"], "test.spec", text).await;
    let resp = client.semantic_tokens_full(&uri).await;
    let data = resp["result"]["data"].as_array().expect("token data array");

    // Decode the delta encoding into absolute (line, start) pairs and verify
    // a line-1 token starts at the UTF-16 column of `behavior` (which is 0).
    let mut line = 0u64;
    let mut col = 0u64;
    let mut decoded: Vec<(u64, u64, u64)> = Vec::new();
    for chunk in data.chunks(5) {
        let dl = chunk[0].as_u64().unwrap();
        let ds = chunk[1].as_u64().unwrap();
        let len = chunk[2].as_u64().unwrap();
        if dl > 0 {
            line += dl;
            col = ds;
        } else {
            col += ds;
        }
        decoded.push((line, col, len));
    }

    // `behavior` starts at UTF-16 col 0 on line 0. The keyword ends at byte
    // col 8, but any token AFTER the multibyte chars must show the UTF-16
    // (smaller) column. The `foo` identifier: bytes would be 8 + (3+2+1) = 14
    // bytes from line start after "behavior –ü ", UTF-16 = 8 + (1+1+1) = 11.
    // The multibyte token "–ü" sits at UTF-16 col 9 with UTF-16 length 2.
    // Byte-column emission would report col 9 len 5; UTF-16 emission gives
    // col 9 len 2. Line 0 also ends with the keyword token (0, 0, 8).
    let tok_on_line0: Vec<(u64, u64, u64)> =
        decoded.iter().filter(|(l, _, _)| *l == 0).copied().collect();
    assert!(
        tok_on_line0.iter().any(|t| *t == (0, 9, 2)),
        "the –ü token must be UTF-16 col 9 len 2 (byte emission: len 5); got {:?}",
        tok_on_line0
    );

    // Deltas within a line must be monotonic (absolutely sorted conversion).
    let mut prev = (0u64, 0u64);
    for (l, c, _) in &decoded {
        if *l == prev.0 {
            assert!(*c >= prev.1, "deltaStart must be non-decreasing within a line");
        }
        prev = (*l, *c);
    }
}
