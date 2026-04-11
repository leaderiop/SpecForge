use serde_json::Value;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn call(state: &McpState, _args: Value, id: Option<Value>) -> JsonRpcResponse {
    let stats = specforge_emitter::compute_stats_with_diagnostics(
        &state.graph,
        &[],
        &state.diagnostics,
    );

    let entity_counts: Vec<Value> = stats.entities_by_kind.iter()
        .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
        .collect();

    let result = serde_json::json!({
        "entity_counts": entity_counts,
        "coverage_pct": stats.coverage_pct,
        "edge_count": stats.total_edges,
        "orphan_count": stats.orphan_count,
        "diagnostic_summary": {
            "errors": stats.error_count,
            "warnings": stats.warning_count,
            "infos": stats.info_count
        }
    });

    JsonRpcResponse::success(id, serde_json::json!({
        "content": [{
            "type": "text",
            "text": result.to_string()
        }]
    }))
}
