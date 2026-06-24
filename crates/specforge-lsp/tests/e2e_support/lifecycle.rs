use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn e2e_initialize_returns_all_capabilities() {
    let (client_to_server, server_stdin) = tokio::io::duplex(1024 * 64);
    let (server_stdout, server_to_client) = tokio::io::duplex(1024 * 64);
    let (service, socket) = LspService::new(Backend::new);
    let server_task = tokio::spawn(async move {
        Server::new(server_stdin, server_stdout, socket)
            .serve(service)
            .await;
    });
    let mut client = LspClient {
        writer: client_to_server,
        reader: server_to_client,
        next_id: Arc::new(Mutex::new(1)),
        server_task,
    };

    let resp = client.initialize(None).await;
    let caps = &resp["result"]["capabilities"];

    // textDocumentSync = 2 (INCREMENTAL)
    assert_eq!(caps["textDocumentSync"], 2);
    assert_eq!(caps["hoverProvider"], true);
    assert_eq!(caps["definitionProvider"], true);
    assert_eq!(caps["referencesProvider"], true);

    // completionProvider with trigger characters
    let triggers = caps["completionProvider"]["triggerCharacters"]
        .as_array()
        .unwrap();
    let trigger_strs: Vec<&str> = triggers.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(trigger_strs.contains(&" "));
    assert!(trigger_strs.contains(&"["));
    assert!(trigger_strs.contains(&"\""));

    // renameProvider with prepareProvider
    assert_eq!(caps["renameProvider"]["prepareProvider"], true);

    // code action provider
    assert_eq!(caps["codeActionProvider"], true);

    // symbol providers
    assert_eq!(caps["documentSymbolProvider"], true);
    assert_eq!(caps["workspaceSymbolProvider"], true);

    // semantic tokens
    assert_eq!(caps["semanticTokensProvider"]["full"], true);
    let legend = &caps["semanticTokensProvider"]["legend"];
    let token_types = legend["tokenTypes"].as_array().unwrap();
    assert_eq!(token_types.len(), specforge_lsp::TOKEN_TYPES.len());

    // formatting
    assert_eq!(caps["documentFormattingProvider"], true);
    assert_eq!(caps["documentRangeFormattingProvider"], true);

    // server_info
    let server_info = &resp["result"]["serverInfo"];
    assert_eq!(
        server_info["name"], "specforge-lsp",
        "server_info.name must be 'specforge-lsp'"
    );
    assert!(
        server_info["version"].is_string(),
        "server_info.version must be present"
    );
}

#[tokio::test]
async fn e2e_initialize_semantic_legend() {
    let (client_to_server, server_stdin) = tokio::io::duplex(1024 * 64);
    let (server_stdout, server_to_client) = tokio::io::duplex(1024 * 64);
    let (service, socket) = LspService::new(Backend::new);
    let server_task = tokio::spawn(async move {
        Server::new(server_stdin, server_stdout, socket)
            .serve(service)
            .await;
    });
    let mut client = LspClient {
        writer: client_to_server,
        reader: server_to_client,
        next_id: Arc::new(Mutex::new(1)),
        server_task,
    };

    let resp = client.initialize(None).await;
    let legend = &resp["result"]["capabilities"]["semanticTokensProvider"]["legend"];
    let token_types: Vec<&str> = legend["tokenTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    for expected in specforge_lsp::TOKEN_TYPES {
        assert!(token_types.contains(expected), "missing token type: {expected}");
    }
}

#[tokio::test]
async fn e2e_initialize_registers_file_watchers() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;

    // After initialized(), server should send client/registerCapability
    // for workspace/didChangeWatchedFiles with *.spec glob pattern.
    // The registration notification comes before the logMessage.
    let notif = client
        .wait_for_notification("client/registerCapability", 5000)
        .await;
    assert!(
        notif.is_some(),
        "Expected client/registerCapability for file watchers"
    );
    let params = &notif.unwrap()["params"];
    let registrations = params["registrations"].as_array().unwrap();
    let has_file_watcher = registrations.iter().any(|r| {
        r["method"].as_str() == Some("workspace/didChangeWatchedFiles")
    });
    assert!(
        has_file_watcher,
        "Expected didChangeWatchedFiles registration, got: {registrations:?}"
    );
}

#[tokio::test]
async fn e2e_workspace_indexing_logs_count() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior alpha \"Alpha\" {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.spec"),
        "behavior beta \"Beta\" {}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;

    // After initialized(), the server should log an indexing message
    let notif = client
        .wait_for_notification("window/logMessage", 5000)
        .await;
    assert!(notif.is_some(), "Expected window/logMessage notification");
    let msg = notif.unwrap()["params"]["message"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        msg.contains("indexed 2"),
        "Expected 'indexed 2' in message, got: {msg}"
    );
}

#[tokio::test]
async fn e2e_shutdown_releases_state() {
    let mut client = start_server(None).await;
    // Drain the logMessage notification from initialized()
    client
        .wait_for_notification("window/logMessage", 2000)
        .await;
    let resp = client.shutdown().await;
    // shutdown should return null result (success)
    assert!(
        resp.get("result").is_some(),
        "Expected result field in shutdown response, got: {resp}"
    );
}

#[tokio::test]
async fn e2e_requests_after_shutdown_fail() {
    let (mut client, uri) =
        start_server_with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    client.shutdown().await;
    let resp = client.hover(&uri, 0, 10).await;
    // After shutdown, hover should return null result
    let result = &resp["result"];
    assert!(result.is_null(), "Expected null result after shutdown");
}

#[tokio::test]
async fn e2e_did_open_registers_document() {
    let (mut client, uri) =
        start_server_with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    // If the document was registered, hover on the entity ID should return info
    let resp = client.hover(&uri, 0, 10).await;
    let result = &resp["result"];
    assert!(!result.is_null(), "Expected hover result for tracked document");
    let md = result["contents"]["value"].as_str().unwrap();
    assert!(md.contains("foo"), "Hover should mention entity ID 'foo'");
}

#[tokio::test]
async fn e2e_did_open_publishes_empty_diagnostics() {
    let mut client = start_server(None).await;
    let uri = "file:///test/clean.spec";
    client
        .did_open(
            uri,
            "specforge",
            "behavior foo \"Foo\" {\n  contract \"Does something\"\n  category \"core\"\n  features [some_feature]\n}\nfeature some_feature \"SF\" {\n  problem \"Needs solving\"\n}\n",
        )
        .await;
    let notif = client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    assert!(notif.is_some(), "Expected publishDiagnostics");
    let notif_val = notif.unwrap();
    let diags = notif_val["params"]["diagnostics"].as_array().unwrap();
    assert!(diags.is_empty(), "Expected no diagnostics for valid spec");
}

#[tokio::test]
async fn e2e_did_open_parse_error_publishes_e001() {
    let mut client = start_server(None).await;
    let uri = "file:///test/broken.spec";
    client.did_open(uri, "specforge", "behavior {").await;
    let notif = client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    assert!(notif.is_some(), "Expected publishDiagnostics");
    let diags = notif.unwrap()["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .to_vec();
    assert!(!diags.is_empty(), "Expected at least one diagnostic");
    // Check severity=1 (ERROR) and code="E001"
    let first = &diags[0];
    assert_eq!(first["severity"], 1);
    assert_eq!(first["code"], "E001");
}

#[tokio::test]
async fn e2e_resolver_diagnostic_e001_unresolved_reference() {
    let mut client = start_server(None).await;
    let uri = "file:///test/resolve.spec";
    // Reference to 'nonexistent' which is not defined anywhere
    client
        .did_open(
            uri,
            "specforge",
            "behavior foo \"Foo\" {\n  types [nonexistent]\n}\n",
        )
        .await;
    let notif = client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    assert!(notif.is_some(), "Expected publishDiagnostics");
    let diags = notif.unwrap()["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .to_vec();
    // Should have at least one E001 for unresolved reference
    let has_e001 = diags
        .iter()
        .any(|d| d["code"].as_str() == Some("E001") && d["message"].as_str().is_some_and(|m| m.contains("unresolved")));
    assert!(
        has_e001,
        "Expected E001 unresolved reference diagnostic, got: {diags:?}"
    );
}

#[tokio::test]
async fn e2e_validator_warnings_appear_in_editor() {
    let mut client = start_server(None).await;
    let uri = "file:///test/validate.spec";
    // 'ref' entity with no incoming refs triggers W012 orphan warning from validator
    // ref uses scheme.kind:id syntax per the grammar
    client
        .did_open(uri, "specforge", "ref gh.issue:42 \"Fix bug\"\n")
        .await;
    let notif = client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;
    assert!(notif.is_some(), "Expected publishDiagnostics");
    let diags = notif.unwrap()["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .to_vec();
    let has_warning = diags.iter().any(|d| {
        // severity=2 is WARNING in LSP
        d["severity"].as_u64() == Some(2)
    });
    assert!(
        has_warning,
        "Expected at least one validator warning, got: {diags:?}"
    );
}

#[tokio::test]
async fn e2e_external_file_change_triggers_recompilation() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior alpha \"Alpha\" {}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    // Drain logMessage from indexing
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Verify alpha exists via workspace symbol search
    let resp = client.workspace_symbol("alpha").await;
    let symbols = resp["result"].as_array().unwrap();
    assert!(!symbols.is_empty(), "alpha should be indexed initially");

    // Simulate external file change: modify a.spec to add a new entity
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior alpha \"Alpha\" {}\nbehavior beta \"Beta\" {}\n",
    )
    .unwrap();

    // Send workspace/didChangeWatchedFiles notification
    let file_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("a.spec"))
        .unwrap()
        .to_string();
    client
        .send_notification(
            "workspace/didChangeWatchedFiles",
            json!({
                "changes": [{
                    "uri": file_uri,
                    "type": 2  // Changed
                }]
            }),
        )
        .await;

    // Wait for diagnostics refresh
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    // Now beta should be findable in workspace symbols
    let resp = client.workspace_symbol("beta").await;
    let symbols = resp["result"].as_array().unwrap();
    assert!(
        symbols.iter().any(|s| s["name"].as_str() == Some("beta")),
        "beta should be indexed after external change, got: {symbols:?}"
    );
}

#[tokio::test]
async fn e2e_new_spec_file_creation_detected() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior alpha \"Alpha\" {}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Create a new .spec file externally
    std::fs::write(
        dir.path().join("b.spec"),
        "behavior gamma \"Gamma\" {}\n",
    )
    .unwrap();

    let file_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("b.spec"))
        .unwrap()
        .to_string();
    client
        .send_notification(
            "workspace/didChangeWatchedFiles",
            json!({
                "changes": [{
                    "uri": file_uri,
                    "type": 1  // Created
                }]
            }),
        )
        .await;

    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    let resp = client.workspace_symbol("gamma").await;
    let symbols = resp["result"].as_array().unwrap();
    assert!(
        symbols.iter().any(|s| s["name"].as_str() == Some("gamma")),
        "gamma should be indexed after file creation, got: {symbols:?}"
    );
}

#[tokio::test]
async fn e2e_deleted_spec_file_removes_entities() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("a.spec"),
        "behavior alpha \"Alpha\" {}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.spec"),
        "behavior beta \"Beta\" {}\n",
    )
    .unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = start_server(Some(root)).await;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Verify beta exists
    let resp = client.workspace_symbol("beta").await;
    assert!(
        resp["result"].as_array().unwrap().iter().any(|s| s["name"].as_str() == Some("beta")),
        "beta should exist initially"
    );

    // Delete b.spec
    std::fs::remove_file(dir.path().join("b.spec")).unwrap();

    let file_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("b.spec"))
        .unwrap()
        .to_string();
    client
        .send_notification(
            "workspace/didChangeWatchedFiles",
            json!({
                "changes": [{
                    "uri": file_uri,
                    "type": 3  // Deleted
                }]
            }),
        )
        .await;

    // Small delay for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // beta should be gone
    let resp = client.workspace_symbol("beta").await;
    let result = &resp["result"];
    let beta_gone = result.is_null()
        || result
            .as_array()
            .is_none_or(|arr| !arr.iter().any(|s| s["name"].as_str() == Some("beta")));
    assert!(beta_gone, "beta should be removed after file deletion, got: {result:?}");
}

#[tokio::test]
async fn e2e_did_close_clears_tracking() {
    let (mut client, uri) =
        start_server_with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    client.did_close(&uri).await;
    // Small delay to let close propagate
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let resp = client.hover(&uri, 0, 10).await;
    let result = &resp["result"];
    assert!(
        result.is_null(),
        "Expected null hover after document closed"
    );
}
