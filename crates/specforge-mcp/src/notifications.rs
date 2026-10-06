use serde_json::Value;

/// Subscription channel for graph delta notifications (C9-01).
pub const GRAPH_CHANNEL: &str = "specforge/graphChanged";

/// Subscription channel for diagnostics delta notifications (C9-01).
pub const DIAGNOSTICS_CHANNEL: &str = "specforge/diagnosticsChanged";

use crate::state::McpState;
use crate::subscriptions::{Changes, Watched, subscribers};
use specforge_common::Diagnostic;
use specforge_project::Update;
pub use specforge_project::{EdgeChange, GraphDelta, compute_graph_delta};

pub struct DiagnosticsDelta {
    pub added: Vec<Diagnostic>,
    pub removed: Vec<Diagnostic>,
}

/// What names a diagnostic in a delta: its code, its message and its file.
fn identity(diagnostic: &Diagnostic) -> (&str, &str, &str) {
    (
        diagnostic.code.as_str(),
        diagnostic.message.as_str(),
        diagnostic
            .span
            .as_ref()
            .map_or("", |span| span.file.as_str()),
    )
}

pub fn compute_diagnostics_delta(old: &[Diagnostic], new: &[Diagnostic]) -> DiagnosticsDelta {
    let old_keys: std::collections::HashSet<_> = old.iter().map(identity).collect();
    let new_keys: std::collections::HashSet<_> = new.iter().map(identity).collect();

    let added: Vec<Diagnostic> = new
        .iter()
        .filter(|d| !old_keys.contains(&identity(d)))
        .cloned()
        .collect();
    let removed: Vec<Diagnostic> = old
        .iter()
        .filter(|d| !new_keys.contains(&identity(d)))
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
/// (C9-01): what changed in the graph is the update's delta, and the previous
/// diagnostics are diffed against the fresh ones, once, for both
/// subscription eras ([`Changes`]). Streams opened with `subscriptions/listen`
/// (MCP 2026-07-28) hear that a resource they listen to changed; each
/// subscribed handshake channel gets one notification onto the
/// server→client outbox. Channels without subscribers are suppressed, and
/// unchanged state emits nothing.
pub fn enqueue_compile_notifications(
    state: &mut McpState,
    update: &Update,
    previous_diagnostics: &[Diagnostic],
) {
    if state.listens.is_empty() && state.subscriptions.is_empty() {
        return;
    }
    let changes = Changes::of(update, previous_diagnostics, &state.diagnostics());
    if !state.listens.is_empty() {
        crate::modern::enqueue_resource_updates(state, &changes);
    }

    for watched in [Watched::Graph, Watched::Diagnostics] {
        let subscriber_count = subscribers(state, watched).len();
        if subscriber_count == 0 || !changes.touched(watched) {
            continue;
        }
        let (notification, event) = match watched {
            Watched::Graph => (
                format_graph_notification(changes.graph),
                serde_json::json!({
                    "notificationType": "graph",
                    "subscriberCount": subscriber_count,
                    "addedNodes": changes.graph.added_nodes.len(),
                    "removedNodes": changes.graph.removed_nodes.len(),
                    "modifiedNodes": changes.graph.modified_nodes.len(),
                }),
            ),
            Watched::Diagnostics => (
                format_diagnostics_notification(&changes.diagnostics),
                serde_json::json!({
                    "notificationType": "diagnostics",
                    "subscriberCount": subscriber_count,
                    "addedDiagnostics": changes.diagnostics.added.len(),
                    "removedDiagnostics": changes.diagnostics.removed.len(),
                }),
            ),
        };
        state.notification_outbox.push(notification);
        state.push_event("mcp_delta_notified", event);
    }
}

/// Drain the server→client notification outbox (the captured client sink).
pub fn pending_notifications(state: &mut McpState) -> Vec<Value> {
    std::mem::take(&mut state.notification_outbox)
}
