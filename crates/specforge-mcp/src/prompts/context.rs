use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required argument: entity_id"),
    };

    let node = match state.graph.node(entity_id) {
        Some(n) => n,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Entity not found: {}", entity_id)),
    };

    let contract_text = node.fields.get("contract")
        .and_then(|v| match v {
            FieldValue::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let upstream: Vec<String> = state.graph.edges_to(entity_id).iter()
        .map(|e| e.source.to_string())
        .collect();

    let downstream: Vec<String> = state.graph.edges_from(entity_id).iter()
        .map(|e| e.target.to_string())
        .collect();

    let verify_expectations: Vec<String> = node.fields.get("verify")
        .and_then(|v| match v {
            FieldValue::VerifyList(stmts) => Some(stmts.iter().map(|s| format!("{} {}", s.kind, s.description)).collect()),
            _ => None,
        })
        .unwrap_or_default();

    let result = serde_json::json!({
        "entity_id": entity_id,
        "kind": node.kind.raw,
        "contract_text": contract_text,
        "upstream_entities": upstream,
        "downstream_entities": downstream,
        "verify_expectations": verify_expectations
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": result.to_string()
            }
        }]
    }))
}
