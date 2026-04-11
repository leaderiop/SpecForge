use serde_json::Value;
use specforge_emitter::{emit, EmitFormat, EmitOptions};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: entity_id"),
    };

    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("graph");
    let include_coverage = args.get("include_coverage").and_then(|v| v.as_bool()).unwrap_or(false);

    let kinds: Vec<&str> = args.get("kinds")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let fmt = match format {
        "context" => EmitFormat::Context,
        "brief" => EmitFormat::Brief,
        _ => EmitFormat::Json,
    };
    let query_result = {
        let options = EmitOptions {
            format: fmt,
            scope: Some(entity_id),
            depth: Some(depth),
            kind_filter: kinds,
            ..EmitOptions::default()
        };
        emit(&state.graph, &options)
    };

    match query_result {
        Ok(json_str) => {
            let mut result: Value = serde_json::from_str(&json_str).unwrap_or(Value::Null);

            if include_coverage && let Some(nodes) = result.get_mut("nodes").and_then(|n| n.as_array_mut()) {
                for node in nodes.iter_mut() {
                    let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let has_verify = state.graph.node(node_id)
                        .map(|n| n.fields.get("verify").is_some())
                        .unwrap_or(false);
                    node.as_object_mut().unwrap().insert(
                        "coverage_status".into(),
                        Value::String(if has_verify { "partial".into() } else { "none".into() }),
                    );
                }
            }

            tool_result(id, result)
        }
        Err(msg) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, msg),
    }
}

fn tool_result(id: Option<Value>, content: Value) -> JsonRpcResponse {
    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": content.to_string()
        }]
    }))
}
