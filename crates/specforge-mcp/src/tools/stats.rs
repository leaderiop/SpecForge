use serde_json::Value;

use crate::state::McpState;
use crate::tool::ToolOutcome;

pub fn call(state: &McpState, _args: Value) -> ToolOutcome {
    // Coverage is the coverage rule's, over the kinds the extensions
    // declare testable, less the entities W004 exempts.
    let coverage = specforge_emitter::coverage::ProjectCoverage::compute(
        &state.graph,
        super::coverage::coverage_registries(state),
        None,
    );
    let stats = specforge_emitter::compute_project_stats(
        &state.graph,
        &coverage.summary,
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
