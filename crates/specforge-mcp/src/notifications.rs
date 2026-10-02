use serde_json::Value;

/// Subscription channel for graph delta notifications (C9-01).
pub const GRAPH_CHANNEL: &str = "specforge/graphChanged";

/// Subscription channel for diagnostics delta notifications (C9-01).
pub const DIAGNOSTICS_CHANNEL: &str = "specforge/diagnosticsChanged";

use crate::state::McpState;
use crate::subscriptions::subscribers;
use specforge_common::Diagnostic;
use specforge_graph::Graph;

/// What changed between two graphs. Node ID lists are sorted; edge lists
/// are sorted by (source, target, label).
pub struct GraphDelta {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    /// Nodes present in both graphs whose kind, title, fields (verify list
    /// included), methods or outgoing edges differ. Source positions are
    /// ignored: moving an entity is not a modification.
    pub modified_nodes: Vec<String>,
    pub added_edges: Vec<EdgeChange>,
    pub removed_edges: Vec<EdgeChange>,
}

impl GraphDelta {
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty()
            && self.removed_nodes.is_empty()
            && self.modified_nodes.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
    }
}

/// One edge in a [`GraphDelta`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct EdgeChange {
    pub source: String,
    pub target: String,
    pub label: String,
}

pub struct DiagnosticsDelta {
    pub added: Vec<Diagnostic>,
    pub removed: Vec<Diagnostic>,
}

pub fn compute_graph_delta(old: &Graph, new: &Graph) -> GraphDelta {
    use std::collections::BTreeSet;

    let old_ids: BTreeSet<&str> = old.nodes().iter().map(|n| n.id.raw.as_str()).collect();
    let new_ids: BTreeSet<&str> = new.nodes().iter().map(|n| n.id.raw.as_str()).collect();
    let old_edges = edge_set(old);
    let new_edges = edge_set(new);

    GraphDelta {
        added_nodes: new_ids
            .difference(&old_ids)
            .map(|s| s.to_string())
            .collect(),
        removed_nodes: old_ids
            .difference(&new_ids)
            .map(|s| s.to_string())
            .collect(),
        modified_nodes: old_ids
            .intersection(&new_ids)
            .filter(|id| node_content(old, id) != node_content(new, id))
            .map(|s| s.to_string())
            .collect(),
        added_edges: new_edges.difference(&old_edges).cloned().collect(),
        removed_edges: old_edges.difference(&new_edges).cloned().collect(),
    }
}

fn edge_set(graph: &Graph) -> std::collections::BTreeSet<EdgeChange> {
    graph
        .edges()
        .iter()
        .map(|e| EdgeChange {
            source: e.source.to_string(),
            target: e.target.to_string(),
            label: e.label.to_string(),
        })
        .collect()
}

/// A node's content with every source position stripped, for comparing
/// the same node across two builds: kind, title, fields keyed by name
/// (verify list included), methods, and sorted outgoing edges.
fn node_content(graph: &Graph, id: &str) -> Value {
    let Some(node) = graph.node(id) else {
        return Value::Null;
    };
    let mut fields: Vec<(String, Value)> = node
        .fields
        .entries()
        .iter()
        .map(|entry| {
            let value = serde_json::json!({
                "value": serde_json::to_value(&entry.value).unwrap_or_default(),
                "annotations": serde_json::to_value(&entry.annotations).unwrap_or_default(),
            });
            (entry.key.to_string(), without_spans(value))
        })
        .collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    let methods: Vec<Value> = node
        .methods
        .iter()
        .map(|m| {
            serde_json::json!({
                "name": m.name,
                "params": serde_json::to_value(&m.params).unwrap_or_default(),
                "returns": m.returns,
            })
        })
        .collect();
    let mut outgoing: Vec<(String, String)> = graph
        .edges_from(id)
        .iter()
        .map(|e| (e.label.to_string(), e.target.to_string()))
        .collect();
    outgoing.sort();
    serde_json::json!({
        "kind": node.kind.raw.as_str(),
        "title": node.title,
        "fields": fields,
        "methods": methods,
        "outgoing": outgoing,
    })
}

/// Drop the positions formal expressions carry: a serialized `SpannedExpr`
/// (`{"expr": .., "span": ..}`) becomes its bare `expr`.
fn without_spans(value: Value) -> Value {
    match value {
        Value::Object(mut map) => {
            if map.len() == 2 && map.contains_key("span") && map.contains_key("expr") {
                return without_spans(map.remove("expr").unwrap_or_default());
            }
            Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, without_spans(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.into_iter().map(without_spans).collect()),
        other => other,
    }
}

pub fn compute_diagnostics_delta(old: &[Diagnostic], new: &[Diagnostic]) -> DiagnosticsDelta {
    let old_keys: std::collections::HashSet<String> = old
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{}",
                d.code,
                d.message,
                d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")
            )
        })
        .collect();
    let new_keys: std::collections::HashSet<String> = new
        .iter()
        .map(|d| {
            format!(
                "{}:{}:{}",
                d.code,
                d.message,
                d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")
            )
        })
        .collect();

    let added: Vec<Diagnostic> = new
        .iter()
        .filter(|d| {
            let key = format!(
                "{}:{}:{}",
                d.code,
                d.message,
                d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")
            );
            !old_keys.contains(&key)
        })
        .cloned()
        .collect();

    let removed: Vec<Diagnostic> = old
        .iter()
        .filter(|d| {
            let key = format!(
                "{}:{}:{}",
                d.code,
                d.message,
                d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")
            );
            !new_keys.contains(&key)
        })
        .cloned()
        .collect();

    DiagnosticsDelta { added, removed }
}

pub fn format_graph_notification(delta: &GraphDelta) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "specforge/graphChanged",
        "params": {
            "added_nodes": delta.added_nodes,
            "removed_nodes": delta.removed_nodes,
            "modified_nodes": delta.modified_nodes,
            "added_edges": delta.added_edges,
            "removed_edges": delta.removed_edges
        }
    })
}

pub fn format_diagnostics_notification(delta: &DiagnosticsDelta) -> Value {
    let added: Vec<Value> = delta
        .added
        .iter()
        .map(|d| {
            serde_json::json!({
                "code": d.code,
                "severity": format!("{:?}", d.severity),
                "message": d.message
            })
        })
        .collect();

    let removed: Vec<Value> = delta
        .removed
        .iter()
        .map(|d| {
            serde_json::json!({
                "code": d.code,
                "severity": format!("{:?}", d.severity),
                "message": d.message
            })
        })
        .collect();

    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "specforge/diagnosticsChanged",
        "params": {
            "added": added,
            "removed": removed
        }
    })
}

/// Queue delta notifications after a (re)compile (C9-01). Diffs the previous
/// graph/diagnostics against the freshly compiled state and pushes one
/// notification per subscribed channel onto the server→client outbox.
/// Channels without subscribers are suppressed, and unchanged state emits
/// nothing.
pub fn enqueue_compile_notifications(
    state: &mut McpState,
    previous_graph: &Graph,
    previous_diagnostics: &[Diagnostic],
) {
    // Streams opened with subscriptions/listen (MCP 2026-07-28) hear that a
    // resource they listen to changed.
    if !state.listens.is_empty() {
        let graph_changed = !compute_graph_delta(previous_graph, state.graph()).is_empty();
        let diagnostics_delta =
            compute_diagnostics_delta(previous_diagnostics, &state.diagnostics());
        let diagnostics_changed =
            !diagnostics_delta.added.is_empty() || !diagnostics_delta.removed.is_empty();
        crate::modern::enqueue_resource_updates(state, graph_changed, diagnostics_changed);
    }

    if !subscribers(state, GRAPH_CHANNEL).is_empty() {
        let graph_delta = compute_graph_delta(previous_graph, state.graph());
        if !graph_delta.is_empty() {
            state
                .notification_outbox
                .push(format_graph_notification(&graph_delta));
            state.push_event(
                "mcp_delta_notified",
                serde_json::json!({
                    "notificationType": "graph",
                    "subscriberCount": subscribers(state, GRAPH_CHANNEL).len(),
                    "addedNodes": graph_delta.added_nodes.len(),
                    "removedNodes": graph_delta.removed_nodes.len(),
                    "modifiedNodes": graph_delta.modified_nodes.len(),
                }),
            );
        }
    }

    if !subscribers(state, DIAGNOSTICS_CHANNEL).is_empty() {
        let diag_delta = compute_diagnostics_delta(previous_diagnostics, &state.diagnostics());
        if !diag_delta.added.is_empty() || !diag_delta.removed.is_empty() {
            state
                .notification_outbox
                .push(format_diagnostics_notification(&diag_delta));
            state.push_event(
                "mcp_delta_notified",
                serde_json::json!({
                    "notificationType": "diagnostics",
                    "subscriberCount": subscribers(state, DIAGNOSTICS_CHANNEL).len(),
                    "addedDiagnostics": diag_delta.added.len(),
                    "removedDiagnostics": diag_delta.removed.len(),
                }),
            );
        }
    }
}

/// Drain the server→client notification outbox (the captured client sink).
pub fn pending_notifications(state: &mut McpState) -> Vec<Value> {
    std::mem::take(&mut state.notification_outbox)
}
