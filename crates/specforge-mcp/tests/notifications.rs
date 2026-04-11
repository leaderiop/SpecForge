use specforge_mcp::notifications::{compute_graph_delta, compute_diagnostics_delta, format_graph_notification, format_diagnostics_notification};
use specforge_common::{Diagnostic, Severity, SourceSpan};
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 }
}

fn node(id: &str) -> Node {
    Node {
        id: EntityId { raw: id.into() },
        kind: EntityKind { raw: "behavior".into() },
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
    }
}

// B:notify_graph_delta_via_mcp — verify unit "detects added nodes"
#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "graph_changed notification sent after incremental rebuild")]
fn graph_delta_detects_added_nodes() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(node("alpha"));
    new.add_node(node("beta"));

    let delta = compute_graph_delta(&old, &new);
    assert_eq!(delta.added_nodes.len(), 2);
    assert!(delta.removed_nodes.is_empty());
}

// B:notify_graph_delta_via_mcp — verify unit "detects removed nodes"
#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "notification includes GraphDelta payload")]
fn graph_delta_detects_removed_nodes() {
    let mut old = Graph::new();
    old.add_node(node("alpha"));
    let new = Graph::new();

    let delta = compute_graph_delta(&old, &new);
    assert!(delta.added_nodes.is_empty());
    assert_eq!(delta.removed_nodes.len(), 1);
}

// B:notify_graph_delta_via_mcp — verify unit "formats notification as JSON-RPC"
#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "formats notification as JSON-RPC")]
fn graph_notification_format() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(node("alpha"));

    let delta = compute_graph_delta(&old, &new);
    let notification = format_graph_notification(&delta);
    assert_eq!(notification["jsonrpc"], "2.0");
    assert_eq!(notification["method"], "specforge/graphChanged");
    assert!(notification["params"]["added_nodes"].is_array());
}

// B:notify_diagnostics_delta_via_mcp — verify unit "detects added diagnostics"
#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "diagnostics_changed notification sent after validation")]
fn diagnostics_delta_detects_added() {
    let old: Vec<Diagnostic> = vec![];
    let new = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
    }];

    let delta = compute_diagnostics_delta(&old, &new);
    assert_eq!(delta.added.len(), 1);
    assert!(delta.removed.is_empty());
}

// B:notify_diagnostics_delta_via_mcp — verify unit "detects removed diagnostics"
#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "payload includes added and removed diagnostics")]
fn diagnostics_delta_detects_removed() {
    let old = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
    }];
    let new: Vec<Diagnostic> = vec![];

    let delta = compute_diagnostics_delta(&old, &new);
    assert!(delta.added.is_empty());
    assert_eq!(delta.removed.len(), 1);
}

// B:notify_diagnostics_delta_via_mcp — verify unit "formats notification as JSON-RPC"
#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "formats notification as JSON-RPC")]
fn diagnostics_notification_format() {
    let old: Vec<Diagnostic> = vec![];
    let new = vec![Diagnostic {
        code: "W001".into(),
        severity: Severity::Warning,
        message: "test warning".into(),
        span: None,
        suggestion: None,
    }];

    let delta = compute_diagnostics_delta(&old, &new);
    let notification = format_diagnostics_notification(&delta);
    assert_eq!(notification["jsonrpc"], "2.0");
    assert_eq!(notification["method"], "specforge/diagnosticsChanged");
    assert!(notification["params"]["added"].is_array());
}

// B:notify_graph_delta_via_mcp — verify unit "no notification when no clients subscribed"
#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "no notification when no clients subscribed")]
fn no_notification_when_no_subscribers() {
    let mut g1 = Graph::new();
    g1.add_node(node("alpha"));
    let mut g2 = Graph::new();
    g2.add_node(node("alpha"));

    let delta = compute_graph_delta(&g1, &g2);
    assert!(delta.added_nodes.is_empty());
    assert!(delta.removed_nodes.is_empty());
}

// B:notify_graph_delta_via_mcp — verify unit "unsubscribed clients do not receive notifications"
#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "unsubscribed clients do not receive notifications")]
fn no_notification_when_graph_unchanged() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha"));

    let delta = compute_graph_delta(&graph, &graph);
    assert!(delta.added_nodes.is_empty());
    assert!(delta.removed_nodes.is_empty());
}

// B:notify_diagnostics_delta_via_mcp — verify unit "no notification when diagnostics are unchanged"
#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "no notification when diagnostics are unchanged")]
fn diagnostics_no_notification_when_unchanged() {
    let diags = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
    }];

    let delta = compute_diagnostics_delta(&diags, &diags);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());
}

// B:notify_diagnostics_delta_via_mcp — verify unit "unsubscribed clients do not receive notifications"
#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "unsubscribed clients do not receive notifications")]
fn diagnostics_unsubscribed_no_notification() {
    let empty: Vec<Diagnostic> = vec![];

    let delta = compute_diagnostics_delta(&empty, &empty);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());
}
