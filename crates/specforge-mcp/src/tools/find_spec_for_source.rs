use serde_json::{json, Value};

use specforge_common::inference::anchors;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let file_path = match args.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => {
            return JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": json!({"error": "Missing required parameter: file_path"}).to_string() }]
                }),
            );
        }
    };

    let project_root = match &state.project_root {
        Some(p) => p.clone(),
        None => {
            return JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": json!({"error": "No project root available"}).to_string() }]
                }),
            );
        }
    };

    let manifest = match anchors::load_anchor_manifest(&project_root) {
        Ok(m) => m,
        Err(e) => {
            return JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": json!({"error": e}).to_string() }]
                }),
            );
        }
    };

    let entities: Vec<Value> = manifest
        .anchors
        .iter()
        .filter(|a| a.file == file_path)
        .map(|a| {
            json!({
                "entity_id": a.entity_id,
                "line": a.line,
                "symbol_name": a.symbol_name,
                "item_kind": a.item_kind,
                "confidence": a.confidence,
            })
        })
        .collect();

    let result = json!({
        "file_path": file_path,
        "entities": entities,
        "count": entities.len(),
    });

    JsonRpcResponse::success(
        id,
        json!({
            "content": [{ "type": "text", "text": result.to_string() }]
        }),
    )
}
