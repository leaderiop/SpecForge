use serde_json::{json, Value};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, arguments: Value, id: Option<Value>) -> JsonRpcResponse {
    let kind = arguments.get("kind").and_then(|v| v.as_str()).unwrap_or("");

    let entities: Vec<Value> = if kind.is_empty() {
        // No kind filter: return all entities
        state.graph.nodes().iter().map(|n| {
            json!({
                "id": n.id.raw.as_str(),
                "kind": n.kind.raw.as_str(),
                "title": n.title.as_deref().unwrap_or(""),
            })
        }).collect()
    } else {
        // Filter by kind
        state.graph.nodes_by_kind(kind).iter().map(|n| {
            json!({
                "id": n.id.raw.as_str(),
                "kind": n.kind.raw.as_str(),
                "title": n.title.as_deref().unwrap_or(""),
            })
        }).collect()
    };

    let text = serde_json::to_string(&entities).unwrap();
    JsonRpcResponse::success(id, json!({
        "content": [{ "type": "text", "text": text }]
    }))
}
