mod brief;
mod context;
mod diagnostics;
mod entities_by_kind;
mod entity;
mod graph;
mod schema;

use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn handle_resource_read(
    state: &mut McpState,
    params: Value,
    id: Option<Value>,
) -> JsonRpcResponse {
    if !state.is_initialized() {
        return JsonRpcResponse::error(id, error_codes::INVALID_REQUEST, "Server not initialized");
    }

    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            return JsonRpcResponse::error(
                id,
                error_codes::INVALID_PARAMS,
                "Missing required parameter: uri",
            );
        }
    };

    state.push_event("mcp_resource_read", serde_json::json!({"uri": uri}));

    match uri.as_str() {
        "specforge://graph" => graph::read(state, id),
        "specforge://schema" => schema::read(state, id),
        // query strings (?max_tokens=N) ride on the resource URIs
        u if u == "specforge://context" || u.starts_with("specforge://context?") => {
            context::read(state, u, id)
        }
        u if u == "specforge://brief" || u.starts_with("specforge://brief?") => {
            brief::read(state, u, id)
        }
        "specforge://diagnostics" => diagnostics::read(state, id),
        _ if uri.starts_with("specforge://graph/") => {
            let entity_id = &uri["specforge://graph/".len()..];
            entity::read(state, entity_id, id)
        }
        _ if uri.starts_with("specforge://entities/") => {
            let kind = &uri["specforge://entities/".len()..];
            entities_by_kind::read(state, kind, id)
        }
        _ => JsonRpcResponse::error(
            id,
            error_codes::INVALID_PARAMS,
            format!("Unknown resource URI: {}", uri),
        ),
    }
}
