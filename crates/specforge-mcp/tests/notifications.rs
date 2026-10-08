use crate::support::*;
use serde_json::json;
use specforge_common::{Diagnostic, Severity, SourceSpan};
use specforge_graph::{Graph, Node};
use specforge_mcp::McpServer;
use specforge_mcp::notifications::{
    DIAGNOSTICS_CHANNEL, GRAPH_CHANNEL, compute_diagnostics_delta, compute_graph_delta,
    format_diagnostics_notification, format_graph_notification,
};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;

/// A feature naming `ghost`, which no entity declares: the compile reports
/// E003.
const BROKEN: &str = "feature broken \"Broken\" {\n    behaviors [ghost]\n}\n";

/// A server over a project of `files`, `@test/ext` declaring the software
/// kinds.
fn served(files: &[(&str, &str)]) -> Served {
    files
        .iter()
        .fold(TestProject::new(), |project, (path, text)| {
            project.file(path, text)
        })
        .serve(&[TestExtension::software()])
}

/// Subscribe the default client to `uri`'s delta notifications.
fn subscribe(server: &mut McpServer, uri: &str) {
    let reply = call(server, "resources/subscribe", json!({"uri": uri}));
    assert_eq!(reply["result"], json!({}), "{reply}");
}

/// Any request: it brings the served project up to date with disk, and
/// the update's notifications are queued for the subscribers.
fn any_request(server: &mut McpServer) {
    let reply = call_tool(server, "specforge.stats", json!({}));
    assert!(reply["error"].is_null(), "{reply}");
}

/// B:notify_graph_delta_via_mcp — verify unit "enqueue delivers one
/// notification per subscribed channel and suppresses empty channels"
#[test]
fn enqueue_delivers_graph_and_diagnostics_to_subscribers() {
    let mut server = served(&[]);
    subscribe(&mut server, "specforge://graph");
    subscribe(&mut server, "specforge://diagnostics");

    // One file adds an entity and a diagnostic.
    server.write("broken.spec", BROKEN);
    any_request(&mut server);

    let sent = server.take_notifications();
    assert_eq!(sent.len(), 2, "one notification per subscribed channel");
    assert_eq!(sent[0]["method"], GRAPH_CHANNEL);
    assert_eq!(sent[1]["method"], DIAGNOSTICS_CHANNEL);

    // Taking them empties the queue (the captured client sink).
    assert!(server.take_notifications().is_empty());
}

/// B:notify_graph_delta_via_mcp — verify unit "enqueue suppresses
/// channels without subscribers and unchanged graphs"
#[test]
fn enqueue_suppresses_unsubscribed_and_unchanged() {
    let mut server = served(&[]);
    // Only the diagnostics channel is subscribed.
    subscribe(&mut server, "specforge://diagnostics");

    // Graph changed but nobody subscribes; diagnostics unchanged anyway.
    server.write("alpha.spec", "behavior alpha \"Alpha\" {\n}\n");
    any_request(&mut server);
    assert!(
        server.take_notifications().is_empty(),
        "graph delta must be suppressed without subscribers"
    );

    // Diagnostics changed and the channel is subscribed.
    server.write("broken.spec", BROKEN);
    any_request(&mut server);
    let sent = server.take_notifications();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["method"], DIAGNOSTICS_CHANNEL);
}

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".into(),
        start_line: 1,
        start_col: 0,
        end_line: 5,
        end_col: 0,
    }
}

fn node(id: &str) -> Node {
    Node {
        id: EntityId { raw: id.into() },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    }
}

// B:notify_graph_delta_via_mcp — verify unit "formats notification as JSON-RPC"
#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "notification includes GraphDelta payload"
)]
fn graph_notification_format() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(node("alpha"));

    let delta = compute_graph_delta(&old, &new);
    let notification = format_graph_notification(&delta);
    assert_eq!(notification["jsonrpc"], "2.0");
    assert_eq!(notification["method"], "specforge/graphChanged");
    assert_eq!(
        notification["params"],
        serde_json::json!({
            "added_nodes": ["alpha"],
            "removed_nodes": [],
            "modified_nodes": [],
            "added_edges": [],
            "removed_edges": []
        })
    );

    // And the other way round: alpha removed.
    let back = format_graph_notification(&compute_graph_delta(&new, &old));
    assert_eq!(back["params"]["added_nodes"], serde_json::json!([]));
    assert_eq!(
        back["params"]["removed_nodes"],
        serde_json::json!(["alpha"])
    );
}

#[test]
fn diagnostics_delta_detects_added() {
    let old: Vec<Diagnostic> = vec![];
    let new = vec![Diagnostic::new(specforge_common::codes::E001, "test error")];

    let delta = compute_diagnostics_delta(&old, &new);
    assert_eq!(delta.added.len(), 1);
    assert!(delta.removed.is_empty());
}

#[test]
fn diagnostics_delta_detects_removed() {
    let old = vec![Diagnostic::new(specforge_common::codes::E001, "test error")];
    let new: Vec<Diagnostic> = vec![];

    let delta = compute_diagnostics_delta(&old, &new);
    assert!(delta.added.is_empty());
    assert_eq!(delta.removed.len(), 1);
}

// B:notify_diagnostics_delta_via_mcp — verify unit "formats notification as JSON-RPC"
#[specforge_test(
    behavior = "notify_diagnostics_delta_via_mcp",
    verify = "payload includes added and removed diagnostics"
)]
fn diagnostics_notification_format() {
    let old: Vec<Diagnostic> = vec![];
    let new = vec![Diagnostic::untyped(
        "W001",
        Severity::Warning,
        "test warning",
    )];

    let delta = compute_diagnostics_delta(&old, &new);
    let notification = format_diagnostics_notification(&delta);
    assert_eq!(notification["jsonrpc"], "2.0");
    assert_eq!(notification["method"], "specforge/diagnosticsChanged");
    assert_eq!(
        notification["params"],
        serde_json::json!({
            "added": [{"code": "W001", "severity": "Warning", "message": "test warning"}],
            "removed": []
        })
    );

    // A fixed warning shows up as removed.
    let fixed = format_diagnostics_notification(&compute_diagnostics_delta(&new, &old));
    assert_eq!(
        fixed["params"],
        serde_json::json!({
            "added": [],
            "removed": [{"code": "W001", "severity": "Warning", "message": "test warning"}]
        })
    );
}

// B:notify_diagnostics_delta_via_mcp — verify unit "no notification when diagnostics are unchanged"
#[specforge_test(
    behavior = "notify_diagnostics_delta_via_mcp",
    verify = "no notification when diagnostics are unchanged"
)]
fn diagnostics_no_notification_when_unchanged() {
    let diags = vec![Diagnostic::new(specforge_common::codes::E001, "test error")];

    let delta = compute_diagnostics_delta(&diags, &diags);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());

    // A subscribed client gets nothing when a compile leaves the
    // diagnostics as they were (an entity added, broken.spec's E003 kept)
    // ...
    let mut server = served(&[("broken.spec", BROKEN)]);
    subscribe(&mut server, "specforge://diagnostics");
    server.write("gamma.spec", "behavior gamma \"Gamma\" {\n}\n");
    any_request(&mut server);
    let sent = server.take_notifications();
    assert!(sent.is_empty(), "{sent:?}");
    // ... and one notification when they change.
    server.remove("broken.spec");
    any_request(&mut server);
    let sent = server.take_notifications();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["method"], DIAGNOSTICS_CHANNEL);
}

#[test]
fn diagnostics_unsubscribed_no_notification() {
    let empty: Vec<Diagnostic> = vec![];

    let delta = compute_diagnostics_delta(&empty, &empty);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());
}
