use super::*;
use tempfile::TempDir;

#[tokio::test]
async fn e2e_initialize_returns_all_capabilities() {
    let mut client = Session::spawn();

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
    // Nothing completes inside a string: `"` triggers nothing (ADR 0023).
    assert!(!trigger_strs.contains(&"\""));

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
    let mut client = Session::spawn();

    let resp = client.initialize(None).await;
    let legend = &resp["result"]["capabilities"]["semanticTokensProvider"]["legend"];
    let token_types: Vec<&str> = legend["tokenTypes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    for expected in specforge_lsp::TOKEN_TYPES {
        assert!(
            token_types.contains(expected),
            "missing token type: {expected}"
        );
    }
}

#[tokio::test]
async fn e2e_initialize_registers_file_watchers() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;

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
    let has_file_watcher = registrations
        .iter()
        .any(|r| r["method"].as_str() == Some("workspace/didChangeWatchedFiles"));
    assert!(
        has_file_watcher,
        "Expected didChangeWatchedFiles registration, got: {registrations:?}"
    );
}

#[tokio::test]
async fn e2e_workspace_indexing_logs_count() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.spec"), "behavior alpha \"Alpha\" {}\n").unwrap();
    std::fs::write(dir.path().join("b.spec"), "behavior beta \"Beta\" {}\n").unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;

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
    let mut client = Session::launch(None, json!({})).await.0;
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
    let (mut client, uri) = Session::with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    client.shutdown().await;
    let resp = client.hover(&uri, 0, 10).await;
    // After shutdown, hover should return null result
    let result = &resp["result"];
    assert!(result.is_null(), "Expected null result after shutdown");
}

#[tokio::test]
async fn e2e_did_open_registers_document() {
    let (mut client, uri) = Session::with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    // If the document was registered, hover on the entity ID should return info
    let resp = client.hover(&uri, 0, 10).await;
    let result = &resp["result"];
    assert!(
        !result.is_null(),
        "Expected hover result for tracked document"
    );
    let md = result["contents"]["value"].as_str().unwrap();
    assert!(md.contains("foo"), "Hover should mention entity ID 'foo'");
}

#[tokio::test]
async fn e2e_did_open_publishes_empty_diagnostics() {
    let mut client = Session::launch(None, json!({})).await.0;
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
    let mut client = Session::launch(None, json!({})).await.0;
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
async fn e2e_resolver_diagnostic_e003_unresolved_reference() {
    let mut client = Session::launch(None, json!({})).await.0;
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
    // Should have at least one E003 for unresolved reference
    let has_e003 = diags.iter().any(|d| {
        d["code"].as_str() == Some("E003")
            && d["message"]
                .as_str()
                .is_some_and(|m| m.contains("unresolved"))
    });
    assert!(
        has_e003,
        "Expected E003 unresolved reference diagnostic, got: {diags:?}"
    );
    // Its typed payload rides in the LSP diagnostic's `data`, which a
    // client echoes back with a code-action request.
    let e003 = diags.iter().find(|d| d["code"] == "E003").unwrap();
    assert_eq!(
        e003["data"],
        serde_json::json!({
            "kind": "unresolved_reference",
            "target": "nonexistent",
            "entity": "foo",
            "field": "types",
        }),
        "{e003}"
    );
}

#[tokio::test]
async fn e2e_validator_warnings_appear_in_editor() {
    let mut client = Session::launch(None, json!({})).await.0;
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
    std::fs::write(dir.path().join("a.spec"), "behavior alpha \"Alpha\" {}\n").unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
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
        .notify(
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
    std::fs::write(dir.path().join("a.spec"), "behavior alpha \"Alpha\" {}\n").unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Create a new .spec file externally
    std::fs::write(dir.path().join("b.spec"), "behavior gamma \"Gamma\" {}\n").unwrap();

    let file_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("b.spec"))
        .unwrap()
        .to_string();
    client
        .notify(
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
    std::fs::write(dir.path().join("a.spec"), "behavior alpha \"Alpha\" {}\n").unwrap();
    std::fs::write(dir.path().join("b.spec"), "behavior beta \"Beta\" {}\n").unwrap();

    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    client
        .wait_for_notification("window/logMessage", 5000)
        .await;

    // Verify beta exists
    let resp = client.workspace_symbol("beta").await;
    assert!(
        resp["result"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"].as_str() == Some("beta")),
        "beta should exist initially"
    );

    // Delete b.spec
    std::fs::remove_file(dir.path().join("b.spec")).unwrap();

    let file_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("b.spec"))
        .unwrap()
        .to_string();
    client
        .notify(
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
    assert!(
        beta_gone,
        "beta should be removed after file deletion, got: {result:?}"
    );
}

#[tokio::test]
async fn e2e_did_close_clears_tracking() {
    let (mut client, uri) = Session::with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
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

#[spec(
    behavior = "lsp_shutdown",
    verify = "requests after shutdown return InvalidRequest"
)]
#[tokio::test]
async fn e2e_requests_after_shutdown_are_invalid() {
    let text = "behavior alpha \"Alpha\" {}\n";
    let (mut client, uri) = Session::with_doc(None, "test.spec", text).await;

    let resp = client.shutdown().await;
    assert!(resp["error"].is_null(), "{resp}");

    let hover = client.hover(&uri, 0, 10).await;
    assert_eq!(hover["error"]["code"], -32600, "{hover}");
}

/// Top-level keyword completions: the kinds the loaded extensions declare.
async fn top_level_keywords(client: &mut Session, uri: &str) -> Vec<String> {
    let resp = client.completion(uri, 0, 0).await;
    resp["result"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|i| i["kind"] == 14) // CompletionItemKind::KEYWORD
        .filter_map(|i| i["label"].as_str().map(str::to_string))
        .collect()
}

#[spec(
    invariant = "lsp_extension_reload_consistency",
    verify = "removing an extension while LSP is running removes kinds from KindRegistry atomically"
)]
#[tokio::test]
async fn e2e_removing_every_extension_clears_the_kinds() {
    let text = "behavior login \"Login\" {\n  contract \"logs in\"\n}\n";
    let (mut client, uri, dir) =
        Session::with_extensions(&["@specforge/software"], "main.spec", text).await;
    let before = top_level_keywords(&mut client, &uri).await;
    assert!(before.iter().any(|k| k == "behavior"), "{before:?}");

    // Every extension is removed from specforge.json.
    let config = json!({"name": "test", "version": "0.1.0", "extensions": []});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    let config_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("specforge.json"))
        .unwrap()
        .to_string();
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": config_uri, "type": 2}]}),
        )
        .await;
    let reloaded = client
        .wait_for_notification("window/logMessage", 5000)
        .await
        .expect("the reload is announced");
    assert!(
        reloaded["params"]["message"]
            .as_str()
            .unwrap()
            .contains("reloaded 0 extension(s)"),
        "{reloaded}"
    );

    let after = top_level_keywords(&mut client, &uri).await;
    assert!(
        !after.iter().any(|k| k == "behavior"),
        "software's kinds outlive its removal: {after:?}"
    );
}

/// Wait for a `window/logMessage` whose message contains `needle`.
async fn wait_for_log(client: &mut Session, needle: &str, timeout_ms: u64) -> Option<String> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let log = client
            .wait_for_notification("window/logMessage", remaining.as_millis() as u64)
            .await?;
        let message = log["params"]["message"].as_str().unwrap_or("").to_string();
        if message.contains(needle) {
            return Some(message);
        }
    }
}

#[spec(
    invariant = "lsp_extension_reload_consistency",
    verify = "a specforge.lock change while the LSP is running reloads the environment"
)]
#[tokio::test]
async fn e2e_a_lock_change_reloads_the_environment() {
    let dir = TempDir::new().unwrap();
    let config = json!({
        "name": "test",
        "version": "0.1.0",
        "extensions": ["@specforge/software", "@acme/missing"],
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        dir.path().join("main.spec"),
        "behavior alpha \"Alpha\" {}\n",
    )
    .unwrap();
    let mut client = Session::launch(Some(dir.path().to_str().unwrap()), json!({}))
        .await
        .0;
    wait_for_log(&mut client, "indexed", 10_000)
        .await
        .expect("the project is indexed");

    // The lock now names the extension: the environment reads the lock, so
    // it must load again.
    let lock = json!({
        "lockfile_version": 1,
        "entries": [{"name": "@acme/missing", "version": "1.0.0", "source": "local:missing.wasm", "wasm_hash": "sha256:00"}]
    });
    std::fs::write(dir.path().join("specforge.lock"), lock.to_string()).unwrap();
    let lock_uri = tower_lsp::lsp_types::Url::from_file_path(dir.path().join("specforge.lock"))
        .unwrap()
        .to_string();
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": lock_uri, "type": 1}]}),
        )
        .await;

    let reloaded = wait_for_log(&mut client, "extension environment changed", 10_000).await;
    assert!(
        reloaded.is_some_and(|m| m.contains("reloaded 1 extension(s)")),
        "a specforge.lock change must reload the environment"
    );
}

/// The `didChangeWatchedFiles` watchers of the last registration.
fn registered_globs(registration: &Value) -> Vec<String> {
    registration["params"]["registrations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["method"] == "workspace/didChangeWatchedFiles")
        .flat_map(|r| r["registerOptions"]["watchers"].as_array().unwrap().clone())
        .map(|w| w["globPattern"].as_str().unwrap_or_default().to_string())
        .collect()
}

#[spec(
    invariant = "lsp_extension_reload_consistency",
    verify = "the LSP watches every file its environment is loaded from"
)]
#[tokio::test]
async fn e2e_registered_watchers_cover_the_environment_inputs() {
    let dir = TempDir::new().unwrap();
    let config = json!({
        "name": "test",
        "version": "0.1.0",
        "spec_root": "spec",
        "extensions": ["@specforge/software", "@acme/local=ext/local.wasm", "@acme/installed"],
    });
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::create_dir_all(dir.path().join("spec")).unwrap();
    std::fs::write(dir.path().join("spec/main.spec"), "").unwrap();
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;

    // The static watchers first, then, once the project is open, the ones
    // it is built from.
    let mut globs = Vec::new();
    for _ in 0..2 {
        let registration = client
            .wait_for_notification("client/registerCapability", 10_000)
            .await
            .expect("a watcher registration");
        globs = registered_globs(&registration);
    }
    for expected in [
        format!("{root}/spec/**/*.spec"),
        format!("{root}/specforge.json"),
        format!("{root}/specforge.lock"),
        format!("{root}/ext/local.wasm"),
        format!("{root}/.specforge/extensions/@acme/installed/extension.wasm"),
    ] {
        assert!(globs.contains(&expected), "{expected} not in {globs:?}");
    }
    // A .wasm no extension loads is not watched.
    assert!(!globs.iter().any(|g| g == "**/*.wasm"), "{globs:?}");
}

/// The globs of the two watcher registrations a server makes once the
/// project is open: the static watchers, then the ones it is built from.
async fn registrations_after_open(client: &mut Session) -> Vec<String> {
    let mut globs = Vec::new();
    for _ in 0..2 {
        let registration = client
            .wait_for_notification("client/registerCapability", 10_000)
            .await
            .expect("a watcher registration");
        globs = registered_globs(&registration);
    }
    globs
}

/// A gadget of the docref project naming `../docs/guide.md` (from `spec/`:
/// `docs/guide.md` under the root), which does not exist.
const NAMES_GUIDE: &str = "gadget gadget_one \"G\" {\n  docs [\"../docs/guide.md\"]\n}\n";

#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP's watchers follow an edit that names a new file the checks read"
)]
#[tokio::test]
async fn e2e_watchers_follow_an_edit_that_names_a_file() {
    let dir = crate::session::docref_project("gadget gadget_one \"G\" {\n}\n");
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    let globs = registrations_after_open(&mut client).await;
    assert!(
        !globs.iter().any(|g| g.ends_with("docs/guide.md")),
        "{globs:?}"
    );

    let a = dir.path().join("spec/a.spec");
    let uri = uri_of(&a);
    client
        .did_open(&uri, "specforge", "gadget gadget_one \"G\" {\n}\n")
        .await;
    client
        .did_change(&uri, 2, vec![json!({"text": NAMES_GUIDE})])
        .await;

    // The edit names a file the checks read: the watchers are asked for
    // again, and now cover it.
    let third = client
        .notification_within(
            "client/registerCapability",
            std::time::Duration::from_secs(5),
            |_| true,
        )
        .await
        .expect("the watchers did not follow the edit");
    let globs = registered_globs(&json!({"params": third}));
    assert!(
        globs.contains(&format!("{root}/docs/guide.md")),
        "{globs:?}"
    );

    // The file appears; the client reports it, and E016 goes.
    std::fs::write(dir.path().join("docs/guide.md"), "# guide\n").unwrap();
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": uri_of(&dir.path().join("docs/guide.md")), "type": 1}]}),
        )
        .await;
    // Anything published before the file was reported is dropped (the first
    // wait clears what was kept), so only what follows it counts.
    let mut cleared = false;
    while let Some(message) = client
        .wait_for_notification("textDocument/publishDiagnostics", 10_000)
        .await
    {
        let params = &message["params"];
        if params["uri"] == uri.as_str()
            && !codes(params["diagnostics"].as_array().unwrap()).contains(&"E016")
        {
            cleared = true;
            break;
        }
    }
    assert!(cleared, "E016 stayed after the file appeared");
}

#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP watches a missing referenced file and its directory, spelled under the project root"
)]
#[tokio::test]
async fn e2e_a_referenced_file_is_watched_under_its_root() {
    let dir = crate::session::docref_project(NAMES_GUIDE);
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    let globs = registrations_after_open(&mut client).await;
    assert!(
        globs.contains(&format!("{root}/docs/guide.md")),
        "{globs:?}"
    );
    assert!(!globs.iter().any(|g| g.contains("/../")), "{globs:?}");
}

#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP watches a missing referenced file and its directory, spelled under the project root"
)]
#[tokio::test]
async fn e2e_a_missing_files_directory_is_watched() {
    let dir = crate::session::docref_project(NAMES_GUIDE);
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    let globs = registrations_after_open(&mut client).await;
    assert!(globs.contains(&format!("{root}/docs/*")), "{globs:?}");
}

/// An edit that names a file moves the client's watchers, and the session
/// catches up on what changed on disk while they moved (ADR 0035). The file
/// is written after the edit, before the client answers the
/// re-registration, so no event can report it: only the catch-up can.
#[spec(
    behavior = "bring_session_up_to_date",
    verify = "after the LSP's watchers move, the session catches up on what changed while they did"
)]
#[tokio::test]
async fn e2e_an_edit_naming_a_file_registers_it_and_catches_up() {
    let dir = crate::session::docref_project("gadget gadget_one \"G\" {\n}\n");
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    registrations_after_open(&mut client).await;

    let a = dir.path().join("spec/a.spec");
    let uri = uri_of(&a);
    client
        .did_open(&uri, "specforge", "gadget gadget_one \"G\" {\n}\n")
        .await;
    client
        .did_change(&uri, 2, vec![json!({"text": NAMES_GUIDE})])
        .await;
    // E016 is published; the client has not been read since, so the
    // server's request to move the watchers is not answered yet.
    loop {
        let diagnostics = client.diagnostics(&uri).await;
        if codes(&diagnostics).contains(&"E016") {
            break;
        }
    }
    std::fs::write(dir.path().join("docs/guide.md"), "# guide\n").unwrap();

    // The client answers the re-registration as it reads it.
    let registered = client
        .notification_within(
            "client/registerCapability",
            std::time::Duration::from_secs(5),
            |_| true,
        )
        .await
        .expect("the watchers did not follow the edit");
    let globs = registered_globs(&json!({"params": registered}));
    assert!(
        globs.contains(&format!("{root}/docs/guide.md")),
        "{globs:?}"
    );
    // No `didChangeWatchedFiles` is sent: only the catch-up after the
    // watchers moved can clear E016.
    let cleared = client
        .notification_within(
            "textDocument/publishDiagnostics",
            std::time::Duration::from_secs(10),
            |p| {
                p["uri"] == uri.as_str()
                    && !codes(p["diagnostics"].as_array().unwrap()).contains(&"E016")
            },
        )
        .await;
    assert!(cleared.is_some(), "E016 stayed after the watchers moved");
}

#[spec(
    behavior = "classify_project_changes",
    verify = "the LSP's watchers follow an edit that names a new file the checks read"
)]
#[tokio::test]
async fn e2e_a_disk_change_naming_a_file_registers_it() {
    let dir = crate::session::docref_project("gadget gadget_one \"G\" {\n}\n");
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    registrations_after_open(&mut client).await;

    let a = dir.path().join("spec/a.spec");
    std::fs::write(&a, NAMES_GUIDE).unwrap();
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": uri_of(&a), "type": 2}]}),
        )
        .await;

    let registered = client
        .notification_within(
            "client/registerCapability",
            std::time::Duration::from_secs(5),
            |_| true,
        )
        .await
        .expect("the watchers did not follow the change on disk");
    let globs = registered_globs(&json!({"params": registered}));
    assert!(
        globs.contains(&format!("{root}/docs/guide.md")),
        "{globs:?}"
    );
}

/// The catch-up after the watchers move reads what changed on disk, but an
/// open document's buffer is the truth for its file: its file, rewritten
/// meanwhile, does not replace it.
#[spec(
    behavior = "bring_session_up_to_date",
    verify = "the LSP's catch-up keeps an open buffer"
)]
#[tokio::test]
async fn e2e_a_catch_up_keeps_an_open_buffer() {
    let dir = crate::session::docref_project("gadget gadget_one \"G\" {\n}\n");
    let root = dir.path().to_str().unwrap();
    let mut client = Session::launch(Some(root), json!({})).await.0;
    registrations_after_open(&mut client).await;

    let a = dir.path().join("spec/a.spec");
    let uri = uri_of(&a);
    // The buffer names the guide and declares an entity only it holds.
    let buffer = format!("{NAMES_GUIDE}gadget gadget_two \"T\" {{\n}}\n");
    client
        .did_open(&uri, "specforge", "gadget gadget_one \"G\" {\n}\n")
        .await;
    client
        .did_change(&uri, 2, vec![json!({"text": buffer})])
        .await;
    loop {
        let diagnostics = client.diagnostics(&uri).await;
        if codes(&diagnostics).contains(&"E016") {
            break;
        }
    }
    // While the watchers move: the guide appears and the file of the open
    // document is rewritten with something else.
    std::fs::write(dir.path().join("docs/guide.md"), "# guide\n").unwrap();
    std::fs::write(&a, "gadget gadget_zero \"Z\" {\n}\n").unwrap();

    client
        .notification_within(
            "client/registerCapability",
            std::time::Duration::from_secs(5),
            |_| true,
        )
        .await
        .expect("the watchers did not follow the edit");
    client
        .notification_within(
            "textDocument/publishDiagnostics",
            std::time::Duration::from_secs(10),
            |p| {
                p["uri"] == uri.as_str()
                    && !codes(p["diagnostics"].as_array().unwrap()).contains(&"E016")
            },
        )
        .await
        .expect("E016 stayed after the watchers moved");

    // The buffer is still what is compiled: its second entity answers.
    let hover = client.hover(&uri, 3, 10).await;
    let value = hover["result"]["contents"]["value"].as_str().unwrap_or("");
    assert!(value.contains("gadget_two"), "{hover}");
}
