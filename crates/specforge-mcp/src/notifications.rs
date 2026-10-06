use serde_json::Value;

/// Subscription channel for graph delta notifications (C9-01).
pub const GRAPH_CHANNEL: &str = "specforge/graphChanged";

/// Subscription channel for diagnostics delta notifications (C9-01).
pub const DIAGNOSTICS_CHANNEL: &str = "specforge/diagnosticsChanged";

use crate::state::McpState;
use crate::subscriptions::subscribers;
use specforge_common::Diagnostic;
use specforge_project::Update;
pub use specforge_project::{EdgeChange, GraphDelta, compute_graph_delta};

pub struct DiagnosticsDelta {
    pub added: Vec<Diagnostic>,
    pub removed: Vec<Diagnostic>,
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

/// `specforge/graphChanged`: the delta with nodes named by ID.
pub fn format_graph_notification(delta: &GraphDelta) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "specforge/graphChanged",
        "params": {
            "added_nodes": delta.added_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "removed_nodes": delta.removed_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "modified_nodes": delta.modified_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
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

/// Queue delta notifications after an update of the served project
/// (C9-01): its delta is what changed in the graph, and the previous
/// diagnostics are diffed against the fresh ones. One notification per
/// subscribed channel goes onto the server→client outbox.
/// Channels without subscribers are suppressed, and unchanged state emits
/// nothing.
pub fn enqueue_compile_notifications(
    state: &mut McpState,
    update: &Update,
    previous_diagnostics: &[Diagnostic],
) {
    let graph_delta = &update.delta;
    // Streams opened with subscriptions/listen (MCP 2026-07-28) hear that a
    // resource they listen to changed.
    if !state.listens.is_empty() {
        let graph_changed = !graph_delta.is_empty();
        let diagnostics_delta =
            compute_diagnostics_delta(previous_diagnostics, &state.diagnostics());
        let diagnostics_changed =
            !diagnostics_delta.added.is_empty() || !diagnostics_delta.removed.is_empty();
        crate::modern::enqueue_resource_updates(state, graph_changed, diagnostics_changed);
    }

    if !subscribers(state, GRAPH_CHANNEL).is_empty() && !graph_delta.is_empty() {
        state
            .notification_outbox
            .push(format_graph_notification(graph_delta));
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
