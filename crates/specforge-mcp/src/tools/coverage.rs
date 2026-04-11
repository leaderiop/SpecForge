use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let results: Vec<Value> = state.graph.nodes().into_iter()
        .filter(|n| {
            if let Some(eid) = entity_filter {
                return n.id.raw == eid;
            }
            if let Some(kind) = kind_filter {
                return n.kind.raw == kind;
            }
            true
        })
        .map(|n| {
            let has_verify = matches!(
                n.fields.get("verify"),
                Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()
            );
            let has_tests = matches!(
                n.fields.get("tests"),
                Some(FieldValue::ReferenceList(refs)) if !refs.is_empty()
            ) || matches!(
                n.fields.get("tests"),
                Some(FieldValue::StringList(refs)) if !refs.is_empty()
            );

            let status = if has_verify && has_tests {
                "covered"
            } else if has_verify {
                "partial"
            } else {
                "uncovered"
            };

            serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "status": status,
                "declared": has_verify,
                "linked": has_tests,
                "evidence_collected": has_tests
            })
        })
        .collect();

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&results).unwrap()
        }]
    }))
}
