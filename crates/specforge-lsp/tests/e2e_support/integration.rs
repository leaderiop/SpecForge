use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn e2e_full_workflow_open_edit_hover_rename() {
    let text = "behavior user_login \"Login\" {}\n";
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;

    // 1. Hover works on initial state
    let resp = client.hover(&uri, 0, 12).await;
    assert!(!resp["result"].is_null(), "Initial hover should work");

    // 2. Change the entity
    client
        .did_change(
            &uri,
            2,
            vec![json!({
                "text": "behavior auth_flow \"Auth Flow\" {}\n"
            })],
        )
        .await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // 3. Hover reflects the change
    let resp = client.hover(&uri, 0, 12).await;
    let md = resp["result"]["contents"]["value"].as_str().unwrap();
    assert!(
        md.contains("auth_flow"),
        "Hover should reflect edit"
    );

    // 4. Rename
    let resp = client.rename(&uri, 0, 12, "login_flow").await;
    assert!(!resp["result"].is_null(), "Rename should succeed");

    // 5. Document symbol reflects current state
    let resp = client.document_symbol(&uri).await;
    let result = &resp["result"];
    // After rename via WorkspaceEdit, the server state may not automatically update
    // (rename returns edits for client to apply). The graph still has the old name.
    // This is correct LSP behavior — the client applies edits and sends didChange.
    assert!(!result.is_null(), "Document symbols should be available");
}

#[tokio::test]
async fn e2e_graph_serves_all_features() {
    let text = concat!(
        "type token \"Token\" {}\n",
        "behavior login \"Login\" {\n",
        "  types [token]\n",
        "}\n",
    );
    let (mut client, uri) = start_server_with_doc(None, "test.spec", text).await;

    // Hover
    let resp = client.hover(&uri, 0, 6).await;
    assert!(!resp["result"].is_null(), "Hover should work");

    // Goto definition
    let resp = client.goto_definition(&uri, 2, 10).await;
    assert!(
        !resp["result"].is_null(),
        "Goto definition should work"
    );

    // References
    let resp = client.references(&uri, 0, 6).await;
    assert!(!resp["result"].is_null(), "References should work");

    // Completion
    let resp = client.completion(&uri, 2, 10).await;
    assert!(!resp["result"].is_null(), "Completion should work");

    // Document symbols
    let resp = client.document_symbol(&uri).await;
    assert!(
        !resp["result"].is_null(),
        "Document symbols should work"
    );
}

#[tokio::test]
async fn e2e_diagnostics_latency() {
    let mut client = start_server(None).await;
    let uri = "file:///test/latency.spec";

    let start = std::time::Instant::now();
    client
        .did_open(uri, "specforge", "behavior foo \"Foo\" {}\n")
        .await;
    let notif = client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    let elapsed = start.elapsed();

    assert!(notif.is_some(), "Should receive diagnostics");
    assert!(
        elapsed.as_millis() < 200,
        "Diagnostics should arrive within 200ms, took {}ms",
        elapsed.as_millis()
    );
}

#[tokio::test]
async fn e2e_multiple_files_cross_reference() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.spec");
    let file_b = dir.path().join("b.spec");
    std::fs::write(&file_b, "type shared_token \"Token\" {}\n").unwrap();
    std::fs::write(
        &file_a,
        "behavior consumer \"Consumer\" {\n  types [shared_token]\n}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    let uri_a = tower_lsp::lsp_types::Url::from_file_path(&file_a)
        .unwrap()
        .to_string();
    let uri_b = tower_lsp::lsp_types::Url::from_file_path(&file_b)
        .unwrap()
        .to_string();

    let text_a = std::fs::read_to_string(&file_a).unwrap();
    let text_b = std::fs::read_to_string(&file_b).unwrap();
    // Open B first so its entities exist, then A so edges from A→B are built
    client.did_open(&uri_b, "specforge", &text_b).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    client.did_open(&uri_a, "specforge", &text_a).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Goto definition from A -> B
    let resp = client.goto_definition(&uri_a, 1, 10).await;
    let result = &resp["result"];
    assert!(
        !result.is_null(),
        "Expected cross-file definition"
    );
    let target = result["uri"].as_str().unwrap();
    assert!(
        target.contains("b.spec"),
        "Definition should point to b.spec"
    );

    // References on B entity -> should include A
    let resp = client.references(&uri_b, 0, 6).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected references");
    let refs = result.as_array().unwrap();
    let has_file_a = refs
        .iter()
        .any(|r| r["uri"].as_str().is_some_and(|u| u.contains("a.spec")));
    assert!(
        has_file_a,
        "References should include file A"
    );
}

/// Verify that cross-file references work immediately after workspace indexing,
/// before any file is explicitly opened. This catches the bug where
/// index_workspace adds nodes but doesn't build edges.
#[tokio::test]
async fn e2e_workspace_index_builds_edges_immediately() {
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
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Open file A and immediately try go-to-definition on 'token' reference
    let uri_a = tower_lsp::lsp_types::Url::from_file_path(&file_a)
        .unwrap()
        .to_string();
    let text_a = std::fs::read_to_string(&file_a).unwrap();
    client.did_open(&uri_a, "specforge", &text_a).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Go-to-definition on 'token' (line 1, inside [token])
    let resp = client.goto_definition(&uri_a, 1, 10).await;
    let result = &resp["result"];
    assert!(
        !result.is_null(),
        "Go-to-definition should work immediately after indexing (edges must be built)"
    );
    let target_uri = result["uri"].as_str().unwrap();
    assert!(
        target_uri.contains("b.spec"),
        "Definition should point to b.spec, got: {target_uri}"
    );
}

/// Verify that deleting a file publishes E003 diagnostics for files with broken references.
#[tokio::test]
async fn e2e_delete_file_publishes_broken_reference_diagnostics() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.spec");
    let file_b = dir.path().join("b.spec");
    std::fs::write(&file_b, "type shared_token \"Token\" {}\n").unwrap();
    std::fs::write(
        &file_a,
        "behavior consumer \"Consumer\" {\n  types [shared_token]\n}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    let uri_a = tower_lsp::lsp_types::Url::from_file_path(&file_a)
        .unwrap()
        .to_string();
    let uri_b = tower_lsp::lsp_types::Url::from_file_path(&file_b)
        .unwrap()
        .to_string();

    // Open file A so it's tracked
    let text_a = std::fs::read_to_string(&file_a).unwrap();
    client.did_open(&uri_a, "specforge", &text_a).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Delete file B
    std::fs::remove_file(&file_b).unwrap();
    client
        .send_notification(
            "workspace/didChangeWatchedFiles",
            json!({
                "changes": [{
                    "uri": uri_b,
                    "type": 3  // Deleted
                }]
            }),
        )
        .await;

    // Collect diagnostics — a.spec should now have E003 for broken 'shared_token' ref
    let mut found_a_e003 = false;
    for _ in 0..5 {
        if let Some(notif) = client
            .wait_for_notification("textDocument/publishDiagnostics", 2000)
            .await
        {
            let notif_uri = notif["params"]["uri"].as_str().unwrap_or("");
            let diags = notif["params"]["diagnostics"].as_array().unwrap();
            if notif_uri.contains("a.spec")
                && diags
                    .iter()
                    .any(|d| d["code"].as_str() == Some("E003"))
            {
                found_a_e003 = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(
        found_a_e003,
        "Deleting b.spec should publish E003 under a.spec for broken 'shared_token' reference"
    );
}

/// Verify that cross-file diagnostics land on the correct file's URI.
/// When entity in file A references something deleted from file B,
/// the E003 diagnostic must appear under file A's URI (where the
/// broken reference lives), not under file B's URI (which was edited).
#[tokio::test]
async fn e2e_cross_file_diagnostic_on_correct_uri() {
    let dir = TempDir::new().unwrap();
    let file_a = dir.path().join("a.spec");
    let file_b = dir.path().join("b.spec");
    std::fs::write(&file_b, "type shared_token \"Token\" {}\n").unwrap();
    std::fs::write(
        &file_a,
        "behavior consumer \"Consumer\" {\n  types [shared_token]\n}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    let uri_a = tower_lsp::lsp_types::Url::from_file_path(&file_a)
        .unwrap()
        .to_string();
    let uri_b = tower_lsp::lsp_types::Url::from_file_path(&file_b)
        .unwrap()
        .to_string();

    let text_a = std::fs::read_to_string(&file_a).unwrap();
    let text_b = std::fs::read_to_string(&file_b).unwrap();

    // Open both files
    client.did_open(&uri_b, "specforge", &text_b).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    client.did_open(&uri_a, "specforge", &text_a).await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Delete the type from file B — now consumer's reference is broken
    client
        .did_change(
            &uri_b,
            2,
            vec![json!({ "text": "// empty\n" })],
        )
        .await;

    // Collect ALL publishDiagnostics notifications (there should be multiple:
    // one for b.spec and one for a.spec with the E003).
    let mut diag_notifications = Vec::new();
    for _ in 0..5 {
        if let Some(notif) = client
            .wait_for_notification("textDocument/publishDiagnostics", 2000)
            .await
        {
            diag_notifications.push(notif);
        } else {
            break;
        }
    }

    // The E003 for unresolved 'shared_token' must be published under a.spec's URI
    let a_has_e001 = diag_notifications.iter().any(|notif| {
        let notif_uri = notif["params"]["uri"].as_str().unwrap_or("");
        let diags = notif["params"]["diagnostics"].as_array().unwrap();
        notif_uri.contains("a.spec")
            && diags
                .iter()
                .any(|d| d["code"].as_str() == Some("E003"))
    });
    assert!(
        a_has_e001,
        "E003 for broken reference must appear under a.spec, not b.spec. Got: {diag_notifications:?}"
    );

    // b.spec should NOT have the E003 (the broken ref is in a.spec, not b.spec)
    let b_has_e001 = diag_notifications.iter().any(|notif| {
        let notif_uri = notif["params"]["uri"].as_str().unwrap_or("");
        let diags = notif["params"]["diagnostics"].as_array().unwrap();
        notif_uri.contains("b.spec")
            && diags
                .iter()
                .any(|d| {
                    d["code"].as_str() == Some("E003")
                        && d["message"]
                            .as_str()
                            .is_some_and(|m| m.contains("unresolved"))
                })
    });
    assert!(
        !b_has_e001,
        "E003 for broken reference must NOT appear under b.spec"
    );
}

/// Verify that opening a file BEFORE workspace indexing completes does not leave
/// stale E003 diagnostics. The user opens a file in Neovim immediately, and the
/// LSP should eventually (after indexing) publish corrected diagnostics with no E003
/// for entities that exist in the workspace. This is the exact race condition from
/// the Neovim screenshot bug.
#[tokio::test]
async fn e2e_cross_file_references_no_stale_e001_after_indexing() {
    let dir = TempDir::new().unwrap();

    // Create type definitions
    let types_dir = dir.path().join("types");
    std::fs::create_dir(&types_dir).unwrap();
    std::fs::write(
        types_dir.join("diagnostics.spec"),
        "type Diagnostic \"Diagnostic\" {}\ntype CodePrefix = E | W | I\n",
    )
    .unwrap();
    std::fs::write(
        types_dir.join("core.spec"),
        "type SourceSpan \"Source Span\" {}\n",
    )
    .unwrap();

    // Create invariants
    let inv_dir = dir.path().join("invariants");
    std::fs::create_dir(&inv_dir).unwrap();
    std::fs::write(
        inv_dir.join("core.spec"),
        "invariant multi_error_collection \"Multi-Error Collection\" {}\n",
    )
    .unwrap();
    std::fs::write(
        inv_dir.join("validation.spec"),
        "invariant diagnostic_determinism \"Diagnostic Determinism\" {}\n",
    )
    .unwrap();
    std::fs::write(
        inv_dir.join("zero-entity-core.spec"),
        "invariant zero_domain_knowledge_core \"Zero Domain Knowledge Core\" {}\n",
    )
    .unwrap();

    // Create features
    let feat_dir = dir.path().join("features");
    std::fs::create_dir(&feat_dir).unwrap();
    std::fs::write(
        feat_dir.join("validation.spec"),
        "feature diagnostic_reporting \"Diagnostic Reporting\" {}\n",
    )
    .unwrap();

    // Create behavior file with cross-file references
    let beh_dir = dir.path().join("behaviors");
    std::fs::create_dir(&beh_dir).unwrap();
    let behavior_file = beh_dir.join("error-reporting.spec");
    std::fs::write(
        &behavior_file,
        concat!(
            "behavior format_diag \"Format Diagnostics\" {\n",
            "  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]\n",
            "  types      [Diagnostic, SourceSpan, CodePrefix]\n",
            "  features   [diagnostic_reporting]\n",
            "}\n",
        ),
    )
    .unwrap();

    // Start server but do NOT wait for indexing to complete
    let (client_to_server, server_stdin) = tokio::io::duplex(1024 * 64);
    let (server_stdout, server_to_client) = tokio::io::duplex(1024 * 64);
    let (service, socket) = tower_lsp::LspService::new(specforge_lsp::backend::Backend::new);
    let server_task = tokio::spawn(async move {
        tower_lsp::Server::new(server_stdin, server_stdout, socket)
            .serve(service)
            .await;
    });
    let mut client = LspClient {
        writer: client_to_server,
        reader: server_to_client,
        next_id: Arc::new(Mutex::new(1)),
        server_task,
    };

    let root = dir.path().to_str().unwrap();
    client.initialize(Some(root)).await;
    client.initialized().await;

    // Open the behavior file IMMEDIATELY — don't wait for indexing
    let uri = tower_lsp::lsp_types::Url::from_file_path(&behavior_file)
        .unwrap()
        .to_string();
    let text = std::fs::read_to_string(&behavior_file).unwrap();
    client.did_open(&uri, "specforge", &text).await;

    // Collect ALL diagnostic notifications over 5 seconds
    // The final notification for the behavior file should have zero E003
    let mut final_e001: Vec<String> = Vec::new();
    let mut final_diags: Vec<serde_json::Value> = Vec::new();
    for _ in 0..20 {
        if let Some(notif) = client
            .wait_for_notification("textDocument/publishDiagnostics", 3000)
            .await
        {
            let notif_uri = notif["params"]["uri"].as_str().unwrap_or("");
            if notif_uri.contains("error-reporting.spec") {
                let diags = notif["params"]["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                final_e001 = diags
                    .iter()
                    .filter(|d| d["code"].as_str() == Some("E003"))
                    .map(|d| d["message"].as_str().unwrap_or("").to_string())
                    .collect();
                final_diags = diags;
            }
        } else {
            break;
        }
    }

    // After indexing + re-diagnosis, the LAST diagnostics for the behavior file
    // must have zero E003 (all cross-file references should have resolved)
    assert!(
        final_e001.is_empty(),
        "After indexing completes, cross-file references should resolve. \
         Stale E003 diagnostics remain: {:?}\n\
         All diagnostics: {:?}",
        final_e001,
        final_diags,
    );
}

/// Verify that cross-file references resolve cleanly after workspace indexing.
/// When a behavior file references types, invariants, and features from other files,
/// and all those entities exist in the workspace, there should be zero E003 diagnostics.
/// This is the exact scenario from the Neovim screenshot bug.
#[tokio::test]
async fn e2e_cross_file_references_no_false_e001() {
    let dir = TempDir::new().unwrap();

    // Create type definitions file
    let types_dir = dir.path().join("types");
    std::fs::create_dir(&types_dir).unwrap();
    std::fs::write(
        types_dir.join("diagnostics.spec"),
        concat!(
            "type Diagnostic \"Diagnostic\" {}\n",
            "type CodePrefix = E | W | I\n",
        ),
    )
    .unwrap();
    std::fs::write(
        types_dir.join("core.spec"),
        "type SourceSpan \"Source Span\" {}\n",
    )
    .unwrap();

    // Create invariants file
    let inv_dir = dir.path().join("invariants");
    std::fs::create_dir(&inv_dir).unwrap();
    std::fs::write(
        inv_dir.join("core.spec"),
        "invariant multi_error_collection \"Multi-Error Collection\" {}\n",
    )
    .unwrap();
    std::fs::write(
        inv_dir.join("validation.spec"),
        "invariant diagnostic_determinism \"Diagnostic Determinism\" {}\n",
    )
    .unwrap();
    std::fs::write(
        inv_dir.join("zero-entity-core.spec"),
        "invariant zero_domain_knowledge_core \"Zero Domain Knowledge Core\" {}\n",
    )
    .unwrap();

    // Create features file
    let feat_dir = dir.path().join("features");
    std::fs::create_dir(&feat_dir).unwrap();
    std::fs::write(
        feat_dir.join("validation.spec"),
        "feature diagnostic_reporting \"Diagnostic Reporting\" {}\n",
    )
    .unwrap();

    // Create behavior file with cross-file references (mirrors error-reporting.spec)
    let beh_dir = dir.path().join("behaviors");
    std::fs::create_dir(&beh_dir).unwrap();
    let behavior_file = beh_dir.join("error-reporting.spec");
    std::fs::write(
        &behavior_file,
        concat!(
            "use \"types/diagnostics\"\n",
            "use \"types/core\"\n",
            "use \"invariants/core\"\n",
            "use \"invariants/validation\"\n",
            "use \"invariants/zero-entity-core\"\n",
            "use \"features/validation\"\n",
            "\n",
            "behavior format_diagnostics_with_source_context \"Format Diagnostics\" {\n",
            "  invariants [multi_error_collection, diagnostic_determinism, zero_domain_knowledge_core]\n",
            "  types      [Diagnostic, SourceSpan, CodePrefix]\n",
            "  features   [diagnostic_reporting]\n",
            "}\n",
        ),
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;

    // Wait for indexing to complete (logMessage is sent after index_workspace finishes)
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Open the behavior file
    let uri = tower_lsp::lsp_types::Url::from_file_path(&behavior_file)
        .unwrap()
        .to_string();
    let text = std::fs::read_to_string(&behavior_file).unwrap();
    client.did_open(&uri, "specforge", &text).await;

    // Collect ALL diagnostic notifications (including the re-diagnosis after indexing)
    let mut all_e001_for_behavior: Vec<String> = Vec::new();
    let mut last_diags_for_behavior: Option<Vec<serde_json::Value>> = None;
    for _ in 0..10 {
        if let Some(notif) = client
            .wait_for_notification("textDocument/publishDiagnostics", 2000)
            .await
        {
            let notif_uri = notif["params"]["uri"].as_str().unwrap_or("");
            if notif_uri.contains("error-reporting.spec") {
                let diags = notif["params"]["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let e001s: Vec<String> = diags
                    .iter()
                    .filter(|d| d["code"].as_str() == Some("E003"))
                    .map(|d| d["message"].as_str().unwrap_or("").to_string())
                    .collect();
                last_diags_for_behavior = Some(diags);
                all_e001_for_behavior = e001s;
            }
        } else {
            break;
        }
    }

    // The final diagnostics for the behavior file should have ZERO E003 errors
    // because all referenced entities exist in the workspace
    assert!(
        all_e001_for_behavior.is_empty(),
        "Cross-file references should resolve cleanly after indexing. \
         Got E003 diagnostics: {:?}\n\
         All diagnostics: {:?}",
        all_e001_for_behavior,
        last_diags_for_behavior,
    );
}
