use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_id = match args.get("entity_id").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, "Missing required parameter: entity_id"),
    };

    let node = match state.graph.node(entity_id) {
        Some(n) => n,
        None => return JsonRpcResponse::error(id, error_codes::INVALID_PARAMS, format!("Entity not found: {}", entity_id)),
    };

    let reference_count = state.graph.edges_to(entity_id).len() + state.graph.edges_from(entity_id).len();

    let contract = node.fields.get("contract").and_then(|v| match v {
        FieldValue::String(s) => Some(s.clone()),
        _ => None,
    });

    let has_verify = matches!(
        node.fields.get("verify"),
        Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()
    );

    let verify_declarations: Option<Vec<String>> = node.fields.get("verify").and_then(|v| match v {
        FieldValue::VerifyList(stmts) => Some(stmts.iter().map(|s| format!("{} {}", s.kind, s.description)).collect()),
        _ => None,
    });

    let references: Vec<String> = state.graph.edges_to(entity_id).iter()
        .map(|e| e.source.to_string())
        .chain(state.graph.edges_from(entity_id).iter().map(|e| e.target.to_string()))
        .collect();

    let entity_diagnostics: Vec<Value> = state.diagnostics.iter()
        .filter(|d| d.message.contains(entity_id))
        .map(|d| serde_json::json!({
            "code": d.code,
            "severity": format!("{:?}", d.severity),
            "message": d.message
        }))
        .collect();

    let coverage_status = if has_verify { "partial" } else { "uncovered" };

    let result = serde_json::json!({
        "entity_id": node.id.raw,
        "kind": node.kind.raw,
        "title": node.title,
        "testable": has_verify,
        "reference_count": reference_count,
        "source_span": {
            "file": node.source_span.file,
            "start_line": node.source_span.start_line,
            "start_col": node.source_span.start_col,
            "end_line": node.source_span.end_line,
            "end_col": node.source_span.end_col,
        },
        "contract": contract,
        "verify_declarations": verify_declarations,
        "references": references,
        "coverage_status": coverage_status,
        "diagnostics": entity_diagnostics
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": result.to_string()
        }]
    }))
}
