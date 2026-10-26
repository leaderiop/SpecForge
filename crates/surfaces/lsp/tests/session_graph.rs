//! The requests only SpecForge's editor client asks (ADR 0268): the session's graph, the entity
//! under the cursor, and the notification that follows a rebuild.

use crate::session::Session;
use serde_json::{Value, json};
use specforge_test_macros::test as spec;

const TEXT: &str = "type user \"User\" {}\nbehavior login \"Log in\" {\n  contract \"returns an auth token\"\n  produces [logged_in]\n}\nevent logged_in \"Logged in\" {\n  payload user\n}\n";

/// A product project whose first term names only itself, beside a pair that names each other.
const SELF: &str = "term loner \"Loner\" {\n  definition \"a term only it names\"\n  see_also [loner]\n}\nterm orphan \"Orphan\" {\n  definition \"a term nothing names\"\n}\nterm left \"Left\" {\n  definition \"l\"\n  see_also [right]\n}\nterm right \"Right\" {\n  definition \"r\"\n}\n";

/// The ids of a `specforge/graph` answer's nodes.
fn node_ids(reply: &Value) -> Vec<&str> {
    reply["result"]["graph"]["nodes"]
        .as_array()
        .unwrap_or_else(|| panic!("{reply}"))
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}

#[spec(
    behavior = "session_graph_request",
    verify = "the graph request answers the session's graph, an unsaved buffer's entities included, with each kind's symbol kind"
)]
#[tokio::test]
async fn the_graph_request_answers_the_session_graph() {
    let (mut client, uri, _dir) =
        Session::with_extensions(&["@specforge/software"], "a.spec", TEXT).await;
    // Typed in the editor, never written to disk.
    let typed = format!("{TEXT}type unsaved_one \"Unsaved\" {{}}\n");
    client
        .did_change(&uri, 2, vec![json!({"text": typed})])
        .await;
    client
        .wait_for_notification("textDocument/publishDiagnostics", 5000)
        .await;

    let reply = client.request("specforge/graph", Value::Null).await;
    let ids = node_ids(&reply);
    for id in ["login", "logged_in", "user", "unsaved_one"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
    let edges = reply["result"]["graph"]["edges"].as_array().unwrap();
    assert!(
        edges.iter().any(|e| e["source"] == "login"
            && e["target"] == "logged_in"
            && e["label"] == "produces"),
        "{reply}"
    );
    assert_eq!(
        reply["result"]["kinds"],
        json!({"behavior": 6, "event": 24, "type": 23}),
        "{reply}"
    );
}

#[spec(
    behavior = "session_graph_request",
    verify = "the graph request answers each entity's degree, an entity naming only itself at 0"
)]
#[tokio::test]
async fn the_graph_request_counts_no_self_edge() {
    let (mut client, _uri, _dir) =
        Session::with_extensions(&["@specforge/product"], "a.spec", SELF).await;
    let reply = client.request("specforge/graph", Value::Null).await;
    let edges = reply["result"]["graph"]["edges"].as_array().unwrap();
    assert!(
        edges
            .iter()
            .any(|e| e["source"] == "loner" && e["target"] == "loner" && e["label"] == "see_also"),
        "the export keeps the self-edge: {reply}"
    );
    assert_eq!(
        reply["result"]["degrees"],
        json!({"loner": 0, "orphan": 0, "left": 1, "right": 1}),
        "{reply}"
    );
}

#[spec(
    behavior = "session_graph_request",
    verify = "the entity request answers the node the cursor names, and nothing for a word in a string"
)]
#[tokio::test]
async fn the_entity_request_names_what_the_cursor_names() {
    let (mut client, uri, _dir) =
        Session::with_extensions(&["@specforge/software"], "a.spec", TEXT).await;
    let at = |line: u32, character: u32| json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}});

    // `token`, inside the contract's string, names nothing.
    let reply = client.request("specforge/entity", at(2, 30)).await;
    assert_eq!(reply["result"], Value::Null, "{reply}");

    let reply = client.request("specforge/entity", at(3, 14)).await;
    let node = &reply["result"];
    assert_eq!(node["id"], "logged_in", "{reply}");
    assert_eq!(node["kind"], "event", "{reply}");
    assert_eq!(node["line"], 6, "{reply}");
    assert!(node["file"].is_string(), "{reply}");
    assert!(node["fields"]["payload"].is_string(), "{reply}");

    let reply = client.request("specforge/entity", at(1, 9)).await;
    assert_eq!(reply["result"]["id"], "login", "{reply}");
}
