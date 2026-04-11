use serde_json::Value;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());

    let suggestions: Vec<Value> = state.diagnostics.iter()
        .filter(|d| {
            if let Some(eid) = entity_filter {
                d.message.contains(eid)
            } else {
                true
            }
        })
        .filter_map(|d| {
            d.suggestion.as_ref().map(|sug| {
                serde_json::json!({
                    "title": sug,
                    "kind": "quickfix",
                    "diagnostic_code": d.code,
                    "edits": []
                })
            })
        })
        .collect();

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&suggestions).unwrap()
        }]
    }))
}
