use serde_json::Value;

/// Subscription channel for graph delta notifications (C9-01).
pub const GRAPH_CHANNEL: &str = "specforge/graphChanged";

/// Subscription channel for diagnostics delta notifications (C9-01).
pub const DIAGNOSTICS_CHANNEL: &str = "specforge/diagnosticsChanged";

use crate::state::McpState;
use crate::subscriptions::subscribers;
use specforge_common::Diagnostic;
use specforge_graph::Graph;

pub struct GraphDelta {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: usize,
    pub removed_edges: usize,
}

pub struct DiagnosticsDelta {
    pub added: Vec<Diagnostic>,
    pub removed: Vec<Diagnostic>,
}

pub fn compute_graph_delta(old: &Graph, new: &Graph) -> GraphDelta {
    let old_ids: std::collections::HashSet<String> =
        old.nodes().iter().map(|n| n.id.raw.to_string()).collect();
    let new_ids: std::collections::HashSet<String> =
        new.nodes().iter().map(|n| n.id.raw.to_string()).collect();

    GraphDelta {
        added_nodes: new_ids.difference(&old_ids).cloned().collect(),
        removed_nodes: old_ids.difference(&new_ids).cloned().collect(),
        added_edges: new.edge_count().saturating_sub(old.edge_count()),
        removed_edges: old.edge_count().saturating_sub(new.edge_count()),
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
    if !subscribers(state, GRAPH_CHANNEL).is_empty() {
        let graph_delta = compute_graph_delta(previous_graph, &state.graph);
        if !graph_delta.added_nodes.is_empty()
            || !graph_delta.removed_nodes.is_empty()
            || graph_delta.added_edges > 0
            || graph_delta.removed_edges > 0
        {
            state
                .notification_outbox
                .push(format_graph_notification(&graph_delta));
            state.push_event(
                "mcp_delta_notified",
                serde_json::json!({
                    "kind": "graph",
                    "subscribers": subscribers(state, GRAPH_CHANNEL).len(),
                    "added": graph_delta.added_nodes.len(),
                    "removed": graph_delta.removed_nodes.len(),
                    "added_edges": graph_delta.added_edges,
                    "removed_edges": graph_delta.removed_edges,
                }),
            );
        }
    }

    if !subscribers(state, DIAGNOSTICS_CHANNEL).is_empty() {
        let diag_delta = compute_diagnostics_delta(previous_diagnostics, &state.diagnostics);
        if !diag_delta.added.is_empty() || !diag_delta.removed.is_empty() {
            state
                .notification_outbox
                .push(format_diagnostics_notification(&diag_delta));
            state.push_event(
                "mcp_delta_notified",
                serde_json::json!({
                    "kind": "diagnostics",
                    "subscribers": subscribers(state, DIAGNOSTICS_CHANNEL).len(),
                    "added": diag_delta.added.len(),
                    "removed": diag_delta.removed.len(),
                }),
            );
        }
    }
}

/// Drain the server→client notification outbox (the captured client sink).
pub fn pending_notifications(state: &mut McpState) -> Vec<Value> {
    std::mem::take(&mut state.notification_outbox)
}
