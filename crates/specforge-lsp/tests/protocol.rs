//! What the LSP answers and applies over the protocol, with the decisions in
//! `answers` and `changes` (architecture plan 10): the same behaviours as the
//! synchronous tests beside them, asked through a client. The tests that
//! start `pin_` assert behaviour kept as it was before those modules.

use crate::session::{Session, codes, uri_of};
use serde_json::{Value, json};
use std::time::Duration;
use tempfile::TempDir;

const A_ALPHA: &str = "type alpha \"A\" {}\n";
const A_OMEGA: &str = "type omega \"O\" {}\n";
const B_USES_ALPHA: &str = "behavior user \"U\" {\n  types [alpha]\n}\n";
const CYCLE: &str =
    "behavior alpha \"A\" {\n  types [beta]\n}\nbehavior beta \"B\" {\n  types [alpha]\n}\n";
const A_DANGLING: &str = "behavior login \"Login\" {\n  invariants [session_limit]\n}\n";
const B_PLAIN: &str = "type other \"O\" {}\n";
const LINKED: &str = "type token \"T\" {}\nbehavior login \"L\" {\n  types [token]\n}\n";

/// A project with no extension holding `files`.
fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
    )
    .unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    dir
}

/// Read and drop every diagnostics publication until none arrives for a
/// while: what the server sent so far is behind us.
async fn settle(client: &mut Session) {
    while client
        .wait_for_notification("textDocument/publishDiagnostics", 500)
        .await
        .is_some()
    {}
}

/// Whether diagnostics with `code` are published for `uri` within a few
/// seconds.
async fn publishes(client: &mut Session, uri: &str, code: &str) -> bool {
    client
        .notification_within(
            "textDocument/publishDiagnostics",
            Duration::from_secs(5),
            |p| {
                p["uri"] == uri
                    && p["diagnostics"]
                        .as_array()
                        .is_some_and(|d| codes(d).contains(&code))
            },
        )
        .await
        .is_some()
}

/// Whether diagnostics without `code` are published for `uri` within a few
/// seconds.
async fn recovers(client: &mut Session, uri: &str, code: &str) -> bool {
    client
        .notification_within(
            "textDocument/publishDiagnostics",
            Duration::from_secs(5),
            |p| {
                p["uri"] == uri
                    && p["diagnostics"]
                        .as_array()
                        .is_some_and(|d| !codes(d).contains(&code))
            },
        )
        .await
        .is_some()
}

/// The names `workspace/symbol` answers for `query`.
async fn symbols(client: &mut Session, query: &str) -> Vec<String> {
    client.workspace_symbol(query).await["result"]
        .as_array()
        .map(|found| {
            found
                .iter()
                .filter_map(|s| s["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn whole(text: &str) -> Vec<Value> {
    vec![json!({"text": text})]
}

#[tokio::test]
async fn a_closed_unsaved_buffer_is_read_from_disk() {
    let dir = project(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]);
    let (mut client, _) = Session::start(Some(dir.path())).await;
    let a = uri_of(&dir.path().join("a.spec"));
    let b = uri_of(&dir.path().join("b.spec"));

    client.open(&a, A_ALPHA).await;
    client.did_change(&a, 2, whole(A_OMEGA)).await;
    assert!(
        publishes(&mut client, &b, "E003").await,
        "the edit compiled"
    );
    settle(&mut client).await;
    client.close(&a).await;

    // b.spec is compiled against the disk's alpha again.
    assert!(
        recovers(&mut client, &b, "E003").await,
        "b.spec's next diagnostics have no E003"
    );
    assert_eq!(symbols(&mut client, "alpha").await, ["alpha"]);
    assert!(symbols(&mut client, "omega").await.is_empty());
}

#[tokio::test]
async fn pin_hover_puts_the_diagnostic_under_the_cursor_first() {
    let (mut client, uri, _dir) = Session::with_extensions(&[], "main.spec", CYCLE).await;
    let hover = client.hover(&uri, 0, 10).await;
    let value = hover["result"]["contents"]["value"].as_str().unwrap();
    assert!(
        value.starts_with("**W061** · Reference cycle detected"),
        "{value}"
    );
    assert!(
        value.contains("\n\n---\n\n**behavior** `alpha` — A"),
        "{value}"
    );
    assert!(!value.contains("**Diagnostics**"), "{value}");
}

#[tokio::test]
async fn pin_definition_is_a_link_only_for_a_link_client() {
    let links = json!({"textDocument": {"definition": {"linkSupport": true}}});
    let (mut client, uri, _dir, _) =
        Session::with_extensions_as(&[], "main.spec", LINKED, links).await;
    let result = client.goto_definition(&uri, 2, 10).await["result"].clone();
    let links = result.as_array().expect("a link client gets an array");
    assert_eq!(links.len(), 1, "{result}");
    let range = |v: &Value| {
        (
            v["start"]["line"].as_u64().unwrap(),
            v["start"]["character"].as_u64().unwrap(),
            v["end"]["line"].as_u64().unwrap(),
            v["end"]["character"].as_u64().unwrap(),
        )
    };
    assert_eq!(range(&links[0]["targetRange"]), (0, 0, 0, 17));
    assert_eq!(range(&links[0]["targetSelectionRange"]), (0, 5, 0, 10));
    assert_eq!(range(&links[0]["originSelectionRange"]), (2, 9, 2, 14));

    let (mut client, uri, _dir) = Session::with_extensions(&[], "main.spec", LINKED).await;
    let result = client.goto_definition(&uri, 2, 10).await["result"].clone();
    assert!(result.is_object(), "{result}");
    assert_eq!(range(&result["range"]), (0, 5, 0, 10));
}

#[tokio::test]
async fn a_reload_reapplies_the_open_buffers() {
    let dir = project(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]);
    let (mut client, _) = Session::start(Some(dir.path())).await;
    let a = uri_of(&dir.path().join("a.spec"));
    let b = uri_of(&dir.path().join("b.spec"));
    client.open(&a, A_ALPHA).await;
    client.did_change(&a, 2, whole(A_OMEGA)).await;
    assert!(
        publishes(&mut client, &b, "E003").await,
        "the edit compiled"
    );

    // The configuration changes on disk: the environment reloads, and the
    // open buffer is still the truth for its file.
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.2.0","extensions":[]}"#,
    )
    .unwrap();
    let config = uri_of(&dir.path().join("specforge.json"));
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": config, "type": 2}]}),
        )
        .await;
    let log = client
        .notification("window/logMessage", |p| {
            p["message"]
                .as_str()
                .is_some_and(|m| m.contains("extension environment changed"))
        })
        .await;
    assert!(log.is_some(), "the reload is announced");

    assert_eq!(symbols(&mut client, "omega").await, ["omega"]);
    assert!(symbols(&mut client, "alpha").await.is_empty());
}

#[tokio::test]
async fn a_disk_change_to_an_open_document_is_ignored_over_the_protocol() {
    let dir = project(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]);
    let (mut client, _) = Session::start(Some(dir.path())).await;
    let a = uri_of(&dir.path().join("a.spec"));
    let b = uri_of(&dir.path().join("b.spec"));
    client.open(&a, A_ALPHA).await;
    client.did_change(&a, 2, whole(A_OMEGA)).await;
    assert!(
        publishes(&mut client, &b, "E003").await,
        "the edit compiled"
    );
    settle(&mut client).await;

    std::fs::write(dir.path().join("a.spec"), "type zeta \"Z\" {}\n").unwrap();
    client
        .notify(
            "workspace/didChangeWatchedFiles",
            json!({"changes": [{"uri": a, "type": 2}]}),
        )
        .await;
    // Nothing is published and the project keeps the buffer's text.
    assert!(
        client
            .wait_for_notification("textDocument/publishDiagnostics", 600)
            .await
            .is_none()
    );
    assert_eq!(symbols(&mut client, "omega").await, ["omega"]);
    assert!(symbols(&mut client, "zeta").await.is_empty());
}

#[tokio::test]
async fn closing_a_detached_buffer_drops_it() {
    let (mut client, test) =
        Session::with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    let other = "file:///test/b.spec";
    client
        .open(other, "behavior user \"U\" {\n  types [foo]\n}\n")
        .await;
    settle(&mut client).await;

    client.close(&test).await;
    assert!(
        publishes(&mut client, other, "E003").await,
        "b.spec now names a missing foo"
    );
    assert!(symbols(&mut client, "foo").await.is_empty());
}

#[tokio::test]
async fn pin_closing_a_clean_source_with_errors_clears_them() {
    let dir = project(&[("a.spec", A_DANGLING), ("b.spec", B_PLAIN)]);
    let (mut client, _) = Session::start(Some(dir.path())).await;
    let a = uri_of(&dir.path().join("a.spec"));
    client.open(&a, A_DANGLING).await;
    assert!(publishes(&mut client, &a, "E003").await);
    settle(&mut client).await;

    client.close(&a).await;
    // Encodes the bug: the handler's empty publish is all the editor gets.
    let cleared = client
        .notification_within(
            "textDocument/publishDiagnostics",
            Duration::from_secs(5),
            |p| p["uri"] == a,
        )
        .await
        .expect("the close publishes");
    assert_eq!(cleared["diagnostics"], json!([]));
    assert!(
        client
            .wait_for_notification("textDocument/publishDiagnostics", 500)
            .await
            .is_none(),
        "nothing follows the empty set"
    );
}

#[tokio::test]
async fn pin_the_open_sequence_reaches_the_client_in_order() {
    let dir = project(&[("a.spec", A_DANGLING)]);
    let root = dir.path().to_str().unwrap();
    let (mut client, _) = Session::launch(Some(root), json!({})).await;
    let end = |m: &Value| m["method"] == "$/progress" && m["params"]["value"]["kind"] == "end";
    let messages = client
        .messages_until(Duration::from_secs(10), end)
        .await
        .expect("workspace indexing never ended");
    let shape = |m: &Value| match m["method"].as_str().unwrap() {
        "$/progress" => format!(
            "$/progress {}",
            m["params"]["value"]["kind"].as_str().unwrap()
        ),
        other => other.to_string(),
    };
    let kept = [
        "client/registerCapability",
        "window/workDoneProgress/create",
        "$/progress begin",
        "textDocument/publishDiagnostics",
        "client/unregisterCapability",
        "window/logMessage",
        "$/progress end",
    ];
    let sequence: Vec<String> = messages
        .iter()
        .map(shape)
        .filter(|s| kept.contains(&s.as_str()))
        .collect();
    assert_eq!(
        sequence,
        [
            "client/registerCapability",
            "window/workDoneProgress/create",
            "$/progress begin",
            "textDocument/publishDiagnostics",
            "client/unregisterCapability",
            "client/registerCapability",
            "window/logMessage",
            "$/progress end",
        ]
    );
    let published = messages
        .iter()
        .find(|m| m["method"] == "textDocument/publishDiagnostics")
        .unwrap();
    assert_eq!(
        published["params"]["uri"],
        uri_of(&dir.path().join("a.spec"))
    );
    let diagnostics = published["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(codes(diagnostics), ["E003"]);
    let log = messages
        .iter()
        .find(|m| m["method"] == "window/logMessage")
        .unwrap();
    assert!(
        log["params"]["message"]
            .as_str()
            .unwrap()
            .contains("indexed 1 .spec files"),
        "{log}"
    );
}

#[tokio::test]
async fn pin_initialize_answers_a_static_result() {
    let mut client = Session::spawn();
    let resp = client.initialize(None).await;
    assert_eq!(
        resp["result"],
        json!({
            "capabilities": {
                "codeActionProvider": true,
                "completionProvider": {"triggerCharacters": [" ", "["]},
                "definitionProvider": true,
                "documentFormattingProvider": true,
                "documentRangeFormattingProvider": true,
                "documentSymbolProvider": true,
                "hoverProvider": true,
                "referencesProvider": true,
                "renameProvider": {"prepareProvider": true},
                "semanticTokensProvider": {
                    "full": true,
                    "legend": {
                        "tokenModifiers": ["declaration", "reference"],
                        "tokenTypes": specforge_lsp::TOKEN_TYPES,
                    },
                },
                "textDocumentSync": 2,
                "workspaceSymbolProvider": true,
            },
            "serverInfo": {"name": "specforge-lsp", "version": env!("CARGO_PKG_VERSION")},
        })
    );
}
