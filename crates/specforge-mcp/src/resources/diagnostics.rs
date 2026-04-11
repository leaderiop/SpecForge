use serde_json::Value;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn read(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let json_str = specforge_emitter::serialize_diagnostics(&state.diagnostics);
    JsonRpcResponse::success(id, serde_json::json!({
        "contents": [{
            "uri": "specforge://diagnostics",
            "mimeType": "application/json",
            "text": json_str
        }]
    }))
}
