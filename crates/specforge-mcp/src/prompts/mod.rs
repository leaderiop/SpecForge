mod context;
mod review;
mod trace;
mod explore;

use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn handle_prompt_get(state: &mut McpState, params: Value, id: Option<Value>) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: name"),
    };

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

    state.push_event("mcp_prompt_invoked", serde_json::json!({"prompt": name}));

    match name.as_str() {
        "specforge://prompts/context" => context::get(state, arguments, id),
        "specforge://prompts/review" => review::get(state, arguments, id),
        "specforge://prompts/trace" => trace::get(state, arguments, id),
        "specforge://prompts/explore" => explore::get(state, arguments, id),
        _ => JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Unknown prompt: {}", name)),
    }
}
