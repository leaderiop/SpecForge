use serde_json::Value;
use specforge_emitter::{emit, EmitFormat, EmitOptions};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn read(state: &McpState, id: Option<Value>) -> JsonRpcResponse {
    let options = EmitOptions { format: EmitFormat::Json, ..EmitOptions::default() };
    let json_str = emit(&state.graph, &options).expect("full graph emit cannot fail");
    let contents: Value = serde_json::from_str(&json_str).unwrap();
    JsonRpcResponse::success(id, resource_contents("specforge://graph", contents))
}

fn resource_contents(uri: &str, contents: Value) -> Value {
    serde_json::json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": contents.to_string()
        }]
    })
}
