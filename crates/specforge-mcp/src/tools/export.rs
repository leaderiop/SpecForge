use serde_json::Value;
use specforge_emitter::{emit, EmitFormat, EmitOptions};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("graph");
    let scope = args.get("scope").and_then(|v| v.as_str());

    let fmt = match format {
        "context" => EmitFormat::Context,
        "brief" => EmitFormat::Brief,
        "graph" => EmitFormat::Json,
        _ => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Unknown format: {}", format)),
    };

    let options = EmitOptions { format: fmt, scope, ..EmitOptions::default() };

    match emit(&state.graph, &options) {
        Ok(json_str) => {
            JsonRpcResponse::success(id, serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": json_str
                }]
            }))
        }
        Err(err) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, err.to_string()),
    }
}
