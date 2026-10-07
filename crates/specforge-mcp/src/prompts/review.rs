//! `specforge://prompts/review`: the coverage gaps of an entity's
//! neighbourhood, or of the whole graph.

use std::collections::HashSet;

use serde_json::{Value, json};
use specforge_ops::coverage::{CoverageQuery, CoverageRow, coverage};

use crate::args::Arguments;
use crate::prompt::{PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::McpError;
use crate::tool::entity_not_found;
use crate::tools::coverage::row_json;

/// `specforge://prompts/review`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to review (optional, reviews all if omitted)
    entity_id: Option<String>,
    // MCP prompt arguments have no `default` field, so the description
    // says it (ADR 0033 D9).
    /// Neighbor hops around entity_id to include (default 1)
    #[arg(default = 1)]
    depth: usize,
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let view = call.view();
    let graph = view.graph();
    let entity_filter = args.entity_id.as_deref();

    // The entity and its neighbors up to `depth` hops, or the whole graph.
    let in_scope: Option<HashSet<String>> = match entity_filter {
        Some(entity_id) => {
            let sub = graph
                .subgraph_depth(entity_id, args.depth)
                .ok_or_else(|| entity_not_found(entity_id))?;
            Some(sub.nodes().iter().map(|n| n.id.raw.to_string()).collect())
        }
        None => None,
    };
    // The coverage view's rows (the entities that count toward coverage,
    // as `specforge.coverage` lists them) in scope; an unusable report is
    // the McpError the coverage tool returns.
    let rows: Vec<CoverageRow> = coverage(&view, &CoverageQuery::default())
        .map_err(McpError::from)?
        .rows
        .into_iter()
        .filter(|row| {
            in_scope
                .as_ref()
                .is_none_or(|ids| ids.contains(&row.entity_id))
        })
        .collect();

    let mut findings: Vec<Value> = Vec::new();
    for row in &rows {
        if !row.declared() {
            findings.push(json!({
                "entity_id": row.entity_id,
                "severity": "warning",
                "message": format!("Entity '{}' has no verify declarations", row.entity_id)
            }));
        }
        let has_edges = !graph.edges_from(&row.entity_id).is_empty()
            || !graph.edges_to(&row.entity_id).is_empty();
        if !has_edges {
            findings.push(json!({
                "entity_id": row.entity_id,
                "severity": "info",
                "message": format!("Entity '{}' is an orphan (no edges)", row.entity_id)
            }));
        }
    }

    let payload = json!({
        "entity_id": entity_filter.unwrap_or("*"),
        "findings": findings,
        "coverage_summary": rows.iter().map(row_json).collect::<Vec<_>>(),
    });

    let scope = entity_filter.unwrap_or("the entire graph");
    let instruction = format!(
        "Analyze the following coverage report for {}. \
         Identify the highest-priority gaps to address. \
         Focus on entities marked 'uncovered' and orphan nodes that may indicate missing relationships.",
        scope
    );

    Ok(Rendered {
        instruction,
        payload,
    })
}
