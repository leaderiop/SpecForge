use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(entity_id) => match state.graph.node(entity_id) {
            Some(node) => Some(node),
            None => {
                return JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    format!("Entity not found: {entity_id}"),
                );
            }
        },
        None => None,
    };
    let file_path = args.get("file_path").and_then(|v| v.as_str());
    let code = args.get("diagnostic_code").and_then(|v| v.as_str());

    let suggestions: Vec<Value> = state
        .diagnostics
        .iter()
        .filter(|d| entity.is_none_or(|node| super::inspect::belongs_to(d, node)))
        .filter(|d| {
            file_path.is_none_or(|file| d.span.as_ref().is_some_and(|span| span.file == file))
        })
        .filter(|d| code.is_none_or(|code| d.code == code))
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

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&suggestions).unwrap()
            }]
        }),
    )
}
