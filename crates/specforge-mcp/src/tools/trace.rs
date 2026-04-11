use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: entity_id"),
    };

    match specforge_emitter::trace(&state.graph, entity_id) {
        Ok(chain) => {
            let mut trace_val: serde_json::Value = serde_json::from_str(&specforge_emitter::serialize_trace(&chain))
                .unwrap_or(serde_json::Value::Null);

            // Add gaps detection
            let mut gaps = Vec::new();
            if chain.upstream.is_empty() {
                gaps.push("no upstream links");
            }
            if chain.downstream.is_empty() {
                gaps.push("no downstream links");
            }
            if let Some(obj) = trace_val.as_object_mut() {
                obj.insert("gaps".into(), serde_json::json!(gaps));
            }

            JsonRpcResponse::success(id, serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": trace_val.to_string()
                }]
            }))
        }
        Err(msg) => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, msg),
    }
}
