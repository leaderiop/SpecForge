use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: entity_id"),
    };

    if state.graph.node(entity_id).is_none() {
        return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Entity not found: {}", entity_id));
    }

    let locations: Vec<Value> = state.graph.edges_to(entity_id).iter()
        .filter_map(|edge| {
            state.graph.node(edge.source.as_str()).map(|n| {
                serde_json::json!({
                    "referencing_entity_id": n.id.raw,
                    "source_span": {
                        "file": n.source_span.file,
                        "start_line": n.source_span.start_line,
                        "start_col": n.source_span.start_col,
                        "end_line": n.source_span.end_line,
                        "end_col": n.source_span.end_col,
                    }
                })
            })
        })
        .collect();

    let result = serde_json::json!({
        "entity_id": entity_id,
        "locations": locations
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": result.to_string()
        }]
    }))
}
