use serde_json::Value;

use crate::target::Call;
use crate::tool::ToolOutcome;

pub fn call(call: &mut Call<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    let view = call.view();
    let state = &*call.state;
    // Coverage is the coverage rule's, over the kinds the extensions
    // declare testable, less the entities W004 exempts.
    // The proof percentage reads the project's recorded tests; a report
    // that is there but unusable is an error result (ADR 0004, D2-e).
    let coverage = match view.coverage() {
        Ok(coverage) => coverage,
        Err(error) => return super::coverage::report_error_result(&error, "specforge.stats"),
    };
    let stats = specforge_ops::stats::compute_project_stats(
        state.graph(),
        &coverage.summary,
        &state.diagnostics(),
    );

    let entity_counts: Vec<Value> = stats
        .entities_by_kind
        .iter()
        .map(|(kind, count)| serde_json::json!({ "kind": kind, "count": count }))
        .collect();

    let result = serde_json::json!({
        "entity_counts": entity_counts,
        "declared_pct": stats.declared_pct,
        "proof_pct": stats.proof_pct,
        // Deprecated alias of declared_pct.
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
