use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: entity_id"),
    };

    let node = match state.graph.node(entity_id) {
        Some(n) => n,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Entity not found: {}", entity_id)),
    };

    let result = serde_json::json!({
        "entity_id": node.id.raw,
        "file_path": node.source_span.file,
        "line": node.source_span.start_line,
        "column": node.source_span.start_col
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": result.to_string()
        }]
    }))
}
