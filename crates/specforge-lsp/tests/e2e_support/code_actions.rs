use super::*;

#[tokio::test]
async fn e2e_code_action_missing_verify() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.code_action(&uri, 0, 0, 2, 1).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected code actions");
    let actions = result.as_array().unwrap();
    assert!(
        !actions.is_empty(),
        "Expected at least one code action for missing verify"
    );
    // Check that at least one action has an edit containing "verify"
    let has_verify_action = actions.iter().any(|a| {
        let edit = &a["edit"];
        if edit.is_null() {
            return false;
        }
        let changes = &edit["changes"];
        if changes.is_null() {
            return false;
        }
        changes.as_object().is_some_and(|m| {
            m.values().any(|edits| {
                edits.as_array().is_some_and(|arr| {
                    arr.iter()
                        .any(|e| e["newText"].as_str().is_some_and(|t| t.contains("verify")))
                })
            })
        })
    });
    assert!(has_verify_action, "Expected a verify code action");
}

#[tokio::test]
async fn e2e_code_action_quickfix_kind() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.code_action(&uri, 0, 0, 2, 1).await;
    let actions = resp["result"].as_array().unwrap();
    for action in actions {
        assert_eq!(
            action["kind"], "quickfix",
            "Code action should have kind=quickfix"
        );
    }
}

#[tokio::test]
async fn e2e_code_action_verify_stub_format() {
    let text = "behavior foo \"Foo\" {\n  contract \"test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.code_action(&uri, 0, 0, 2, 1).await;
    let actions = resp["result"].as_array().unwrap();
    // Find a verify-related action and check its edit text format
    let verify_action = actions.iter().find(|a| {
        a["title"]
            .as_str()
            .is_some_and(|t| t.to_lowercase().contains("verify"))
    });
    assert!(verify_action.is_some(), "Expected a verify action");
    let edit_text = verify_action.unwrap()["edit"]["changes"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .as_array()
        .unwrap()[0]["newText"]
        .as_str()
        .unwrap();
    assert!(
        edit_text.contains("verify"),
        "Stub should contain 'verify'"
    );
    assert!(
        edit_text.contains("foo"),
        "Stub should reference entity ID 'foo'"
    );
}

#[tokio::test]
async fn e2e_no_code_action_when_verify_exists() {
    let text = "behavior bar \"Bar\" {\n  contract \"test\"\n  verify unit \"bar test\"\n}\n";
    let (mut client, uri, _dir) = start_server_with_extensions(
        &["@specforge/software"],
        "test.spec", text,
    ).await;
    let resp = client.code_action(&uri, 0, 0, 3, 1).await;
    let result = &resp["result"];
    // Should be null or empty — entity already has verify
    if !result.is_null() {
        let actions = result.as_array().unwrap();
        let verify_actions: Vec<&Value> = actions
            .iter()
            .filter(|a| {
                a["title"]
                    .as_str()
                    .is_some_and(|t| t.to_lowercase().contains("verify"))
            })
            .collect();
        assert!(
            verify_actions.is_empty(),
            "Expected no verify actions when verify exists"
        );
    }
}
