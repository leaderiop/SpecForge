use serde_json::Value;
use specforge_common::Diagnostic;
use specforge_graph::Graph;

use crate::state::McpState;

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
    let old_ids: std::collections::HashSet<String> = old.nodes().iter().map(|n| n.id.raw.to_string()).collect();
    let new_ids: std::collections::HashSet<String> = new.nodes().iter().map(|n| n.id.raw.to_string()).collect();

    GraphDelta {
        added_nodes: new_ids.difference(&old_ids).cloned().collect(),
        removed_nodes: old_ids.difference(&new_ids).cloned().collect(),
        added_edges: new.edge_count().saturating_sub(old.edge_count()),
        removed_edges: old.edge_count().saturating_sub(new.edge_count()),
    }
}

pub fn compute_diagnostics_delta(old: &[Diagnostic], new: &[Diagnostic]) -> DiagnosticsDelta {
    let old_keys: std::collections::HashSet<String> = old.iter()
        .map(|d| format!("{}:{}:{}", d.code, d.message, d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")))
        .collect();
    let new_keys: std::collections::HashSet<String> = new.iter()
        .map(|d| format!("{}:{}:{}", d.code, d.message, d.span.as_ref().map(|s| s.file.as_str()).unwrap_or("")))
        .collect();

    let added: Vec<Diagnostic> = new.iter()
        .filter(|d| {
            let key = format!("{}:{}:{}", d.code, d.message, d.span.as_ref().map(|s| s.file.as_str()).unwrap_or(""));
            !old_keys.contains(&key)
        })
        .cloned()
        .collect();

    let removed: Vec<Diagnostic> = old.iter()
        .filter(|d| {
            let key = format!("{}:{}:{}", d.code, d.message, d.span.as_ref().map(|s| s.file.as_str()).unwrap_or(""));
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
    let added: Vec<Value> = delta.added.iter().map(|d| serde_json::json!({
        "code": d.code,
        "severity": format!("{:?}", d.severity),
        "message": d.message
    })).collect();

    let removed: Vec<Value> = delta.removed.iter().map(|d| serde_json::json!({
        "code": d.code,
        "severity": format!("{:?}", d.severity),
        "message": d.message
    })).collect();

    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "specforge/diagnosticsChanged",
        "params": {
            "added": added,
            "removed": removed
        }
    })
}

pub fn pending_notifications(state: &mut McpState) -> Vec<Value> {
    let mut notifications = Vec::new();

    let graph_delta = compute_graph_delta(&Graph::new(), &state.graph);
    if !graph_delta.added_nodes.is_empty() || !graph_delta.removed_nodes.is_empty() {
        notifications.push(format_graph_notification(&graph_delta));
        state.push_event("mcp_delta_notified", serde_json::json!({"kind": "graph"}));
    }

    let diag_delta = compute_diagnostics_delta(&state.previous_diagnostics, &state.diagnostics);
    if !diag_delta.added.is_empty() || !diag_delta.removed.is_empty() {
        notifications.push(format_diagnostics_notification(&diag_delta));
        state.push_event("mcp_delta_notified", serde_json::json!({"kind": "diagnostics"}));
    }

    notifications
}
