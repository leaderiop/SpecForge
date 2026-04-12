use serde_json::Value;
use specforge_graph::FieldValue;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());

    let nodes: Vec<_> = state.graph.nodes().into_iter()
        .filter(|n| entity_filter.is_none() || entity_filter == Some(n.id.raw.as_str()))
        .collect();

    let mut findings: Vec<Value> = Vec::new();
    let mut coverage: Vec<Value> = Vec::new();

    for node in &nodes {
        let has_verify = matches!(
            node.fields.get("verify"),
            Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()
        );

        let status = if has_verify { "partial" } else { "uncovered" };

        coverage.push(serde_json::json!({
            "entity_id": node.id.raw,
            "kind": node.kind.raw,
            "status": status,
            "declared": has_verify,
            "linked": false,
            "evidence_collected": false
        }));

        if !has_verify {
            findings.push(serde_json::json!({
                "entity_id": node.id.raw,
                "severity": "warning",
                "message": format!("Entity '{}' has no verify declarations", node.id.raw)
            }));
        }

        // Check for orphans
        let has_edges = !state.graph.edges_from(node.id.raw.as_str()).is_empty()
            || !state.graph.edges_to(node.id.raw.as_str()).is_empty();
        if !has_edges {
            findings.push(serde_json::json!({
                "entity_id": node.id.raw,
                "severity": "info",
                "message": format!("Entity '{}' is an orphan (no edges)", node.id.raw)
            }));
        }
    }

    let result = serde_json::json!({
        "entity_id": entity_filter.unwrap_or("*"),
        "findings": findings,
        "coverage_summary": coverage
    });

    let scope = entity_filter.unwrap_or("the entire graph");
    let instruction = format!(
        "Analyze the following coverage report for {}. \
         Identify the highest-priority gaps to address. \
         Focus on entities marked 'uncovered' and orphan nodes that may indicate missing relationships.",
        scope
    );

    JsonRpcResponse::success(id, serde_json::json!({
        "messages": [
            {
                "role": "user",
                "content": { "type": "text", "text": instruction }
            },
            {
                "role": "assistant",
                "content": { "type": "text", "text": result.to_string() }
            }
        ]
    }))
}
