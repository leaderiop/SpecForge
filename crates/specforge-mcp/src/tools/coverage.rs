use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

/// Entity ids with recorded tests in the project's `specforge-report.json`
/// (written by `specforge collect`). Tests link themselves to entities by
/// annotation (ADR 0002), so recorded results are the linkage.
pub(crate) fn entities_with_recorded_tests(state: &McpState) -> std::collections::HashSet<String> {
    let Some(root) = &state.project_root else {
        return Default::default();
    };
    let Ok(raw) = std::fs::read_to_string(root.join("specforge-report.json")) else {
        return Default::default();
    };
    let Ok(report) = serde_json::from_str::<specforge_emitter::analyze::TestReport>(&raw) else {
        return Default::default();
    };
    report
        .results
        .into_iter()
        .filter(|(_, entity)| !entity.tests.is_empty())
        .map(|(id, _)| id)
        .collect()
}

pub fn call(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let recorded = entities_with_recorded_tests(state);
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let results: Vec<Value> = state
        .graph
        .nodes()
        .into_iter()
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
            let has_tests = recorded.contains(n.id.raw.as_str());

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

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap()
            }]
        }),
    )
}
