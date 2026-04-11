use serde_json::json;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;
use serde_json::Value;

pub fn read(state: &McpState, kind: &str, id: Option<Value>) -> JsonRpcResponse {
    let entities: Vec<serde_json::Value> = state.graph.nodes_by_kind(kind).iter().map(|n| {
        json!({
            "id": n.id.raw.as_str(),
            "kind": n.kind.raw.as_str(),
            "title": n.title.as_deref().unwrap_or(""),
        })
    }).collect();

    let text = serde_json::to_string(&entities).unwrap();
    JsonRpcResponse::success(id, json!({
        "contents": [{
            "uri": format!("specforge://entities/{}", kind),
            "mimeType": "application/json",
            "text": text,
        }]
    }))
}
