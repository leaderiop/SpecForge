use serde_json::Value;

use crate::tool::ToolOutcome;
use specforge_ops::view::ProjectView;

/// `specforge.stats`: the stats operation over the call's project view, its
/// diagnostics what the server reports for the project (the view's
/// `reported`). The proof percentage reads the project's recorded tests; a
/// report that is there but unusable is an error result (ADR 0004, D2-e).
pub fn call(view: ProjectView<'_>, _args: crate::args::NoArgs) -> ToolOutcome {
    let stats = match specforge_ops::stats::stats(&view) {
        Ok(stats) => stats,
        Err(error) => return crate::tool::McpError::from(error).into(),
    };

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
        "unconnected_count": stats.unconnected_count,
        "diagnostic_summary": {
            "errors": stats.error_count,
            "warnings": stats.warning_count,
            "infos": stats.info_count
        }
    });

    ToolOutcome::ok(result)
}
