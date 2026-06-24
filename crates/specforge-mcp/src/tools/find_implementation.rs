use serde_json::{json, Value};

use specforge_common::inference::anchors;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return JsonRpcResponse::success(
                id,
                json!({
                    "content": [{ "type": "text", "text": json!({"error": "Missing required parameter: entity_id"}).to_string() }]
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

    let sources: Vec<Value> = manifest
        .anchors
        .iter()
        .filter(|a| a.entity_id == entity_id)
        .map(|a| {
            json!({
                "file": a.file,
                "line": a.line,
                "symbol_name": a.symbol_name,
                "item_kind": a.item_kind,
                "scanner": a.scanner,
            })
        })
        .collect();

    let result = json!({
        "entity_id": entity_id,
        "implementations": sources,
        "count": sources.len(),
    });

    JsonRpcResponse::success(
        id,
        json!({
            "content": [{ "type": "text", "text": result.to_string() }]
        }),
    )
}
