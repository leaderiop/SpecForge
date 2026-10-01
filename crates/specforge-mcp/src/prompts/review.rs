use serde_json::Value;

use crate::protocol::{JsonRpcResponse, error_codes};
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

    // The entity and its neighbors up to `depth` hops, or the whole graph.
    let in_scope: Option<std::collections::HashSet<String>> = match entity_filter {
        Some(entity_id) => match state.graph.subgraph_depth(entity_id, depth) {
            Some(sub) => Some(sub.nodes().iter().map(|n| n.id.raw.to_string()).collect()),
            None => {
                return JsonRpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    format!("Entity not found: {entity_id}"),
                );
            }
        },
        None => None,
    };
    // Coverage is about testable entities only, as `specforge.coverage` reports.
    let testable = specforge_emitter::coverage::testable_kinds(&state.kind_registry);
    let mut nodes: Vec<_> = state
        .graph
        .nodes()
        .into_iter()
        .filter(|n| {
            in_scope
                .as_ref()
                .is_none_or(|ids| ids.contains(n.id.raw.as_str()))
        })
        .filter(|n| testable.contains(n.kind.raw.as_str()))
        .collect();
    nodes.sort_by(|a, b| a.id.raw.as_str().cmp(b.id.raw.as_str()));

    let mut findings: Vec<Value> = Vec::new();
    let mut coverage: Vec<Value> = Vec::new();
    // A prompt has no isError result: an unusable report is a JSON-RPC
    // error carrying the same McpError the coverage tool returns.
    let report = match crate::tools::coverage::recorded_report(state) {
        Ok(report) => report,
        Err(e) => {
            return JsonRpcResponse::error_with_data(
                id,
                error_codes::INTERNAL_ERROR,
                e.to_string(),
                crate::tools::coverage::report_mcp_error(&e, "specforge://prompts/review")
                    .to_json(),
            );
        }
    };

    // The same classification `specforge.coverage` reports.
    let project = specforge_emitter::coverage::ProjectCoverage::compute(
        &state.graph,
        crate::tools::coverage::coverage_registries(state),
        report.as_ref(),
    );
    for node in &nodes {
        let Some(verdict) = project.verdict(node.id.raw.as_str()) else {
            continue;
        };
        let has_verify = verdict.obligations > 0;
        coverage.push(serde_json::json!({
            "entity_id": node.id.raw,
            "kind": node.kind.raw,
            "status": crate::tools::coverage::status_name(verdict.status()),
            "declared": has_verify,
            "linked": verdict.tests > 0,
            "evidence_collected": verdict.tests > 0,
            "obligations": verdict.obligations,
            "proven": verdict.proven,
            "unproven": verdict.unproven,
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

    JsonRpcResponse::success(
        id,
        serde_json::json!({
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
        }),
    )
}
