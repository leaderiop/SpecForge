use specforge_common::{Diagnostic, Severity, SourceSpan};
use specforge_graph::{Graph, Node};
use specforge_mcp::notifications::{
    DIAGNOSTICS_CHANNEL, GRAPH_CHANNEL, compute_diagnostics_delta, compute_graph_delta,
    enqueue_compile_notifications, format_diagnostics_notification, format_graph_notification,
};
use specforge_mcp::state::McpState;
use specforge_mcp::subscriptions;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, parse_expression};
use specforge_test::prelude::*;

/// B:notify_graph_delta_via_mcp — verify unit "enqueue delivers one
/// notification per subscribed channel and suppresses empty channels"
#[test]
fn enqueue_delivers_graph_and_diagnostics_to_subscribers() {
    let mut state = McpState::new();
    subscriptions::subscribe(&mut state, "c1", GRAPH_CHANNEL);
    subscriptions::subscribe(&mut state, "c1", DIAGNOSTICS_CHANNEL);

    let previous = Graph::new();
    let mut current = Graph::new();
    current.add_node(node("alpha"));
    state.serve_graph(current, Vec::new());
    state.surface_diagnostics = vec![Diagnostic {
        code: "V001".into(),
        severity: Severity::Error,
        message: "boom".into(),
        span: None,
        suggestion: None,
        data: None,
    }];

    let delta = compute_graph_delta(&previous, state.graph());
    enqueue_compile_notifications(&mut state, &delta, &[]);

    assert_eq!(
        state.notification_outbox.len(),
        2,
        "one notification per subscribed channel"
    );
    assert_eq!(state.notification_outbox[0]["method"], GRAPH_CHANNEL);
    assert_eq!(state.notification_outbox[1]["method"], DIAGNOSTICS_CHANNEL);

    // Draining empties the outbox (the captured client sink).
    let drained = specforge_mcp::notifications::pending_notifications(&mut state);
    assert_eq!(drained.len(), 2);
    assert!(state.notification_outbox.is_empty());
}

/// B:notify_graph_delta_via_mcp — verify unit "enqueue suppresses
/// channels without subscribers and unchanged graphs"
#[test]
fn enqueue_suppresses_unsubscribed_and_unchanged() {
    let mut state = McpState::new();
    // Only the diagnostics channel is subscribed.
    subscriptions::subscribe(&mut state, "c1", DIAGNOSTICS_CHANNEL);

    let mut graph = Graph::new();
    graph.add_node(node("alpha"));
    state.serve_graph(graph, Vec::new());

    // Graph changed but nobody subscribes; diagnostics unchanged anyway.
    let delta = compute_graph_delta(&Graph::new(), state.graph());
    enqueue_compile_notifications(&mut state, &delta, &[]);
    assert!(
        state.notification_outbox.is_empty(),
        "graph delta must be suppressed without subscribers"
    );

    // Diagnostics changed and the channel is subscribed.
    state.surface_diagnostics = vec![Diagnostic {
        code: "V001".into(),
        severity: Severity::Error,
        message: "boom".into(),
        span: None,
        suggestion: None,
        data: None,
    }];
    let delta = compute_graph_delta(&Graph::new(), state.graph());
    enqueue_compile_notifications(&mut state, &delta, &[]);
    assert_eq!(state.notification_outbox.len(), 1);
    assert_eq!(state.notification_outbox[0]["method"], DIAGNOSTICS_CHANNEL);
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

#[test]
fn graph_delta_detects_added_nodes() {
    let old = Graph::new();
    let mut new = Graph::new();
    new.add_node(node("alpha"));
    new.add_node(node("beta"));

    let delta = compute_graph_delta(&old, &new);
    assert_eq!(delta.added_nodes.len(), 2);
    assert!(delta.removed_nodes.is_empty());
}

#[test]
fn graph_delta_detects_removed_nodes() {
    let mut old = Graph::new();
    old.add_node(node("alpha"));
    let new = Graph::new();

    let delta = compute_graph_delta(&old, &new);
    assert!(delta.added_nodes.is_empty());
    assert_eq!(delta.removed_nodes.len(), 1);
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
    let new = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
        data: None,
    }];

    let delta = compute_diagnostics_delta(&old, &new);
    assert_eq!(delta.added.len(), 1);
    assert!(delta.removed.is_empty());
}

#[test]
fn diagnostics_delta_detects_removed() {
    let old = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
        data: None,
    }];
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
    let new = vec![Diagnostic {
        code: "W001".into(),
        severity: Severity::Warning,
        message: "test warning".into(),
        span: None,
        suggestion: None,
        data: None,
    }];

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

#[test]
fn no_notification_when_no_subscribers() {
    let mut g1 = Graph::new();
    g1.add_node(node("alpha"));
    let mut g2 = Graph::new();
    g2.add_node(node("alpha"));

    let delta = compute_graph_delta(&g1, &g2);
    assert!(delta.added_nodes.is_empty());
    assert!(delta.removed_nodes.is_empty());
}

#[test]
fn no_notification_when_graph_unchanged() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha"));

    let delta = compute_graph_delta(&graph, &graph);
    assert!(delta.added_nodes.is_empty());
    assert!(delta.removed_nodes.is_empty());
}

// B:notify_diagnostics_delta_via_mcp — verify unit "no notification when diagnostics are unchanged"
#[specforge_test(
    behavior = "notify_diagnostics_delta_via_mcp",
    verify = "no notification when diagnostics are unchanged"
)]
fn diagnostics_no_notification_when_unchanged() {
    let diags = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "test error".into(),
        span: None,
        suggestion: None,
        data: None,
    }];

    let delta = compute_diagnostics_delta(&diags, &diags);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());

    // A subscribed client gets nothing when a compile leaves the
    // diagnostics as they were ...
    let mut state = McpState::new();
    subscriptions::subscribe(&mut state, "c1", DIAGNOSTICS_CHANNEL);
    state.surface_diagnostics = diags.clone();
    let delta = compute_graph_delta(&Graph::new(), state.graph());
    enqueue_compile_notifications(&mut state, &delta, &diags);
    assert!(
        state.notification_outbox.is_empty(),
        "{:?}",
        state.notification_outbox
    );
    // ... and one notification when they change.
    let delta = compute_graph_delta(&Graph::new(), state.graph());
    enqueue_compile_notifications(&mut state, &delta, &[]);
    assert_eq!(state.notification_outbox.len(), 1);
    assert_eq!(state.notification_outbox[0]["method"], DIAGNOSTICS_CHANNEL);
}

#[test]
fn diagnostics_unsubscribed_no_notification() {
    let empty: Vec<Diagnostic> = vec![];

    let delta = compute_diagnostics_delta(&empty, &empty);
    assert!(delta.added.is_empty());
    assert!(delta.removed.is_empty());
}

/// alpha carrying a formal `metric` expression parsed from `src`, declared
/// at `line`.
fn metric_node(src: &str, line: usize) -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        "metric".into(),
        FieldValue::Expression(vec![parse_expression(src).unwrap()]),
    );
    let mut node = node("alpha");
    node.fields = fields;
    node.source_span.start_line = line;
    node.source_span.end_line = line + 4;
    node
}

fn graph_of(node: Node) -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node);
    graph
}

#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "moving an entity is not a modification"
)]
fn expression_positions_are_not_a_modification() {
    // Same expression, shifted: the entity moved and the expression's
    // columns moved with it.
    let before = graph_of(metric_node("latency < 100ms", 1));
    let after = graph_of(metric_node("   latency < 100ms", 9));
    let delta = compute_graph_delta(&before, &after);
    assert!(delta.is_empty(), "{:?}", delta.modified_nodes);

    // A changed bound is a modification.
    let tighter = graph_of(metric_node("latency < 50ms", 1));
    assert_eq!(
        compute_graph_delta(&before, &tighter)
            .modified_nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        ["alpha"]
    );
}
