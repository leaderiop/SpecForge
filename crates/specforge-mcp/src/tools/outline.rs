use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let file = match args.get("file").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: file"),
    };

    let mut entries: Vec<Value> = state.graph.nodes_in_file(file).iter()
        .map(|n| {
            serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "title": n.title,
                "range": {
                    "file": n.source_span.file,
                    "start_line": n.source_span.start_line,
                    "start_col": n.source_span.start_col,
                    "end_line": n.source_span.end_line,
                    "end_col": n.source_span.end_col,
                }
            })
        })
        .collect();

    // Sort by line number
    entries.sort_by_key(|e| e["range"]["start_line"].as_u64().unwrap_or(0));

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&entries).unwrap()
        }]
    }))
}
