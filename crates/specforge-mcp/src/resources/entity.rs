use serde_json::Value;
use specforge_emitter::{EmitFormat, EmitOptions, emit};

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn read(state: &McpState, entity_id: &str, id: Option<Value>) -> JsonRpcResponse {
    if entity_id.is_empty() {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            "Malformed entity ID: must not be empty",
        );
    }
    // A malformed ID (the 400 case) is told apart from a well-formed one
    // that names no entity (the 404 case).
    if !entity_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'))
    {
        return JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!(
                "Malformed entity ID: {:?} may only contain letters, digits, '_', '.', ':' and '-'",
                entity_id
            ),
        );
    }

    let options = EmitOptions {
        format: EmitFormat::Json,
        scope: Some(entity_id),
        // The entity and its immediate neighbors, not everything reachable.
        depth: Some(1),
        ..EmitOptions::default()
    };

    match emit(&state.graph, &options) {
        Ok(json_str) => {
            let uri = format!("specforge://graph/{}", entity_id);
            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": json_str
                    }]
                }),
            )
        }
        Err(_) => JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Entity not found: {}", entity_id),
        ),
    }
}
