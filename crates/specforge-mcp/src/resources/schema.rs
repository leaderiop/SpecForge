use serde_json::Value;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn read(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let schema = specforge_emitter::generate_schema(
        &state.kind_registry,
        &state.edge_registry,
        &state.field_registry,
        &state.extension_info,
    );
    let schema_json = specforge_emitter::emit_schema(&schema);

    JsonRpcResponse::success(id, serde_json::json!({
        "contents": [{
            "uri": "specforge://schema",
            "mimeType": "application/json",
            "text": schema_json
        }]
    }))
}
