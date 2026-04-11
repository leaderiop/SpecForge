use serde_json::Value;
use specforge_emitter::{emit, EmitFormat, EmitOptions};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn read(state: &McpState, entity_id: &str, id: Option<Value>) -> JsonRpcResponse {
    if entity_id.is_empty() {
        return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Entity ID must not be empty");
    }

    let options = EmitOptions {
        format: EmitFormat::Json,
        scope: Some(entity_id),
        ..EmitOptions::default()
    };

    match emit(&state.graph, &options) {
        Ok(json_str) => {
            let uri = format!("specforge://graph/{}", entity_id);
            JsonRpcResponse::success(id, serde_json::json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": json_str
                }]
            }))
        }
        Err(_) => {
            JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                format!("Entity not found: {}", entity_id),
            )
        }
    }
}
