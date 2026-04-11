use serde_json::Value;
use specforge_emitter::{emit, EmitFormat, EmitOptions};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn read(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let options = EmitOptions { format: EmitFormat::Brief, ..EmitOptions::default() };
    let json_str = emit(&state.graph, &options).expect("full graph emit cannot fail");
    JsonRpcResponse::success(id, serde_json::json!({
        "contents": [{
            "uri": "specforge://brief",
            "mimeType": "application/json",
            "text": json_str
        }]
    }))
}
