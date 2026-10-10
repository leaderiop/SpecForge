use serde::Serialize;
use specforge_common::shape::Shape;

use crate::args::NoArgs;
use crate::reply::Answered;
use crate::tool::McpError;
use specforge_ops::view::ProjectView;

/// `specforge.stats`'s reply (`McpStatsResult`).
#[derive(Debug, Serialize, Shape)]
pub struct Reply {
    entity_counts: Vec<EntityCount>,
    declared_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof_pct: Option<f64>,
    edge_count: usize,
    unconnected_count: usize,
    diagnostic_summary: DiagnosticSummary,
}

/// One kind and how many entities of it the project holds.
#[derive(Debug, Serialize, Shape)]
pub struct EntityCount {
    kind: String,
    count: usize,
}

/// How many diagnostics of each severity the project reports.
#[derive(Debug, Serialize, Shape)]
pub struct DiagnosticSummary {
    errors: usize,
    warnings: usize,
    infos: usize,
}

/// `specforge.stats`: the stats operation over the call's project view, its
/// diagnostics what the server reports for the project (the view's
/// `reported`). The proof percentage reads the project's recorded tests; a
/// report that is there but unusable is an error result (ADR 0004, D2-e).
pub fn call(view: ProjectView<'_>, _args: NoArgs) -> Answered<Reply> {
    let stats = specforge_ops::stats::stats(&view).map_err(McpError::from)?;
    Ok(Reply {
        entity_counts: stats
            .entities_by_kind
            .iter()
            .map(|(kind, count)| EntityCount {
                kind: kind.clone(),
                count: *count,
            })
            .collect(),
        declared_pct: stats.declared_pct,
        proof_pct: stats.proof_pct,
        edge_count: stats.total_edges,
        unconnected_count: stats.unconnected_count,
        diagnostic_summary: DiagnosticSummary {
            errors: stats.error_count,
            warnings: stats.warning_count,
            infos: stats.info_count,
        },
    }
    .into())
}
