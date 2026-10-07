//! Characterization of what the LSP answers and applies today, which the
//! decision modules (`answers`, `changes`) move (architecture plan 10).
//! Each `pin_*` test asserts the behaviour as it is; the ticket that fixes
//! the behaviour flips or deletes its pin.

use crate::session::{Session, codes, uri_of};
use serde_json::{Value, json};
use specforge_lsp::{LspState, Target, navigator};
use specforge_project::ProjectSession;
use std::time::Duration;
use tempfile::TempDir;
use tower_lsp::lsp_types::{Position, Url};

/// The text the project is compiled from, and the buffer after a line is
/// inserted at the top (typed, not yet compiled).
const STALE_COMPILED: &str = "type alpha \"A\" {}\ntype beta \"B\" {}\ntype gamma \"C\" {}\n";
const STALE_TYPED: &str =
    "type delta \"D\" {}\ntype alpha \"A\" {}\ntype beta \"B\" {}\ntype gamma \"C\" {}\n";
const A_ALPHA: &str = "type alpha \"A\" {}\n";
const A_OMEGA: &str = "type omega \"O\" {}\n";
const B_USES_ALPHA: &str = "behavior user \"U\" {\n  types [alpha]\n}\n";
const B_USES_OMEGA: &str = "behavior user \"U\" {\n  types [omega]\n}\n";
const CYCLE: &str =
    "behavior alpha \"A\" {\n  types [beta]\n}\nbehavior beta \"B\" {\n  types [alpha]\n}\n";
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

fn url_of(dir: &TempDir, file: &str) -> Url {
    Url::from_file_path(dir.path().join(file)).unwrap()
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

#[test]
fn pin_a_stale_cursor_names_the_compiled_token_at_its_position() {
    let dir = project(&[("a.spec", STALE_COMPILED)]);
    let mut state = LspState::new();
    state.set_session(ProjectSession::open(dir.path()));
    let uri = url_of(&dir, "a.spec");
    state.open_document(uri.as_str(), STALE_COMPILED);
    state.apply_change(uri.as_str(), None, STALE_TYPED);

    // The buffer's `beta` is at 2:6; the compiled text has `gamma` there.
    let doc = state.document(uri.as_str()).unwrap();
    let cursor = doc.at(Position::new(2, 6)).unwrap();
    let nav = navigator(&state);
    let target = cursor.target(&nav, "a.spec");
    assert!(
        matches!(
            &target,
            Some(Target::Entity { id, origin })
                if id.as_str() == "gamma"
                    && (origin.start.line, origin.start.character) == (2, 5)
                    && (origin.end.line, origin.end.character) == (2, 10)
        ),
        "{target:?}"
    );
    let occurrence = cursor.occurrence(&nav, "a.spec").unwrap();
    assert_eq!(occurrence.target.as_str(), "gamma");
    assert_eq!(
        (
            occurrence.span.start_line,
            occurrence.span.start_col,
            occurrence.span.end_line,
            occurrence.span.end_col
        ),
        (3, 6, 3, 11)
    );
}

#[tokio::test]
async fn pin_a_closed_unsaved_buffer_stays_compiled() {
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
    client.close(&a).await;

    // The discarded text is still what the project is compiled from.
    assert_eq!(symbols(&mut client, "omega").await, ["omega"]);
    assert!(symbols(&mut client, "alpha").await.is_empty());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.spec")).unwrap(),
        A_ALPHA
    );
}

#[tokio::test]
async fn pin_a_batch_of_edits_publishes_each_file() {
    let dir = project(&[("a.spec", A_ALPHA), ("b.spec", B_USES_ALPHA)]);
    let (mut client, _) = Session::start(Some(dir.path())).await;
    let a = uri_of(&dir.path().join("a.spec"));
    let b = uri_of(&dir.path().join("b.spec"));
    client.open(&a, A_ALPHA).await;
    client.open(&b, B_USES_ALPHA).await;
    settle(&mut client).await;

    // A rename's edits, applied by the client as two back-to-back changes.
    client.did_change(&a, 2, whole(A_OMEGA)).await;
    client.did_change(&b, 2, whole(B_USES_OMEGA)).await;

    let mut published: Vec<Vec<String>> = Vec::new();
    while let Some(message) = client
        .wait_for_notification("textDocument/publishDiagnostics", 1500)
        .await
    {
        let params = &message["params"];
        if params["uri"] == b.as_str() {
            let list = params["diagnostics"].as_array().unwrap();
            published.push(codes(list).iter().map(|c| c.to_string()).collect());
        }
    }
    assert!(
        published
            .first()
            .is_some_and(|c| c.contains(&"E003".into())),
        "the half-applied edit is published first: {published:?}"
    );
    assert!(
        published
            .get(1)
            .is_some_and(|c| !c.contains(&"E003".into())),
        "then the whole edit: {published:?}"
    );
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
async fn pin_a_reload_reapplies_the_open_buffers() {
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
async fn pin_a_disk_change_to_an_open_document_is_ignored() {
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
async fn pin_closing_a_detached_buffer_keeps_it_compiled() {
    let (mut client, test) =
        Session::with_doc(None, "test.spec", "behavior foo \"Foo\" {}\n").await;
    let other = "file:///test/b.spec";
    client
        .open(other, "behavior user \"U\" {\n  types [foo]\n}\n")
        .await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    client.close(&test).await;
    assert_eq!(symbols(&mut client, "foo").await, ["foo"]);
}
