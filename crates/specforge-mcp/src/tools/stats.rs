use serde_json::Value;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, _args: Value) -> ToolOutcome {
    // Coverage is over the kinds the extensions declare testable; with no
    // testable kind named, it could only ever be 0.
    let testable_kinds: Vec<&str> =
        specforge_emitter::coverage::testable_kinds(&state.kind_registry)
            .into_iter()
            .collect();
    let stats = specforge_emitter::compute_stats_with_diagnostics(
        &state.graph,
        &testable_kinds,
        &state.diagnostics,
    );

    let entity_counts: Vec<Value> = stats
        .entities_by_kind
        .iter()
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

    ToolOutcome::ok(result)
}
