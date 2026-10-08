//! The notifications' JSON shapes, in both subscription eras.

use serde_json::{Value, json};
use specforge_project::GraphDelta;

use super::DiagnosticsDelta;

/// The `_meta` key tying a notification to its `subscriptions/listen`.
pub(super) const SUBSCRIPTION_ID_META: &str = "io.modelcontextprotocol/subscriptionId";
/// The handshake era's graph delta (C9-01).
pub(super) const GRAPH_CHANGED: &str = "specforge/graphChanged";
/// The handshake era's diagnostics delta (C9-01).
pub(super) const DIAGNOSTICS_CHANGED: &str = "specforge/diagnosticsChanged";

/// `notifications/subscriptions/acknowledged` for the stream `id`, naming
/// the resources honoured.
pub(super) fn acknowledged(id: &Value, uris: &[String]) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/subscriptions/acknowledged",
        "params": {
            "_meta": { SUBSCRIPTION_ID_META: id },
            "notifications": { "resourceSubscriptions": uris },
        },
    })
}

/// `notifications/resources/updated` for `uri`, tagged with the stream's id
/// when it is sent on one.
pub(super) fn resource_updated(uri: &str, stream: Option<&Value>) -> Value {
    let mut params = json!({ "uri": uri });
    if let Some(id) = stream {
        params["_meta"] = json!({ SUBSCRIPTION_ID_META: id });
    }
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/resources/updated",
        "params": params,
    })
}

/// `specforge/graphChanged`: the delta with nodes named by ID.
pub(super) fn graph_changed(delta: &GraphDelta) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": GRAPH_CHANGED,
        "params": {
            "added_nodes": delta.added_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "removed_nodes": delta.removed_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "modified_nodes": delta.modified_nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
            "added_edges": delta.added_edges,
            "removed_edges": delta.removed_edges
        }
    })
}

/// `specforge/diagnosticsChanged`: what was added and removed, each as
/// `{code, severity, message}`.
pub(super) fn diagnostics_changed(delta: &DiagnosticsDelta) -> Value {
    let named = |diagnostics: &[specforge_common::Diagnostic]| -> Vec<Value> {
        diagnostics
            .iter()
            .map(|d| {
                json!({
                    "code": d.code,
                    "severity": format!("{:?}", d.severity),
                    "message": d.message
                })
            })
            .collect()
    };
    json!({
        "jsonrpc": "2.0",
        "method": DIAGNOSTICS_CHANGED,
        "params": {
            "added": named(&delta.added),
            "removed": named(&delta.removed)
        }
    })
}
