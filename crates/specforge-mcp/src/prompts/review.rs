//! `specforge://prompts/review`: the coverage gaps of an entity's
//! neighbourhood, or of the whole graph.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::prompt::{PromptArgs, PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::entity_not_found;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default)]
    entity_id: Option<String>,
    #[serde(default = "one", deserialize_with = "crate::args::count")]
    depth: usize,
}

fn one() -> usize {
    1
}

impl PromptArgs for Args {
    const DESCRIPTIONS: &'static [(&'static str, &'static str)] = &[
        (
            "entity_id",
            "Entity ID to review (optional, reviews all if omitted)",
        ),
        (
            "depth",
            "Neighbor hops around entity_id to include (default 1)",
        ),
    ];
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let view = call.view();
    let graph = view.graph;
    let entity_filter = args.entity_id.as_deref();

    // The entity and its neighbors up to `depth` hops, or the whole graph.
    let in_scope: Option<std::collections::HashSet<String>> = match entity_filter {
        Some(entity_id) => {
            let sub = graph
                .subgraph_depth(entity_id, args.depth)
                .ok_or_else(|| entity_not_found(entity_id))?;
            Some(sub.nodes().iter().map(|n| n.id.raw.to_string()).collect())
        }
        None => None,
    };
    // Coverage is about testable entities only, as `specforge.coverage` reports.
    let testable = specforge_project::coverage::testable_kinds(&view.registries.kinds);
    let mut nodes: Vec<_> = graph
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
    // The same classification `specforge.coverage` reports, from the
    // project view's memo; an unusable report is the McpError the coverage
    // tool returns.
    let project = view
        .coverage()
        .map_err(|e| crate::tools::coverage::report_mcp_error(&e))?;
    for node in &nodes {
        let Some(verdict) = project.verdict(node.id.raw.as_str()) else {
            continue;
        };
        let has_verify = verdict.obligations > 0;
        coverage.push(json!({
            "entity_id": node.id.raw,
            "kind": node.kind.raw,
            "status": specforge_ops::coverage::status_name(verdict.status()),
            "declared": has_verify,
            "linked": verdict.tests > 0,
            "evidence_collected": verdict.tests > 0,
            "obligations": verdict.obligations,
            "proven": verdict.proven,
            "unproven": verdict.unproven,
        }));

        if !has_verify {
            findings.push(json!({
                "entity_id": node.id.raw,
                "severity": "warning",
                "message": format!("Entity '{}' has no verify declarations", node.id.raw)
            }));
        }

        // Check for orphans
        let has_edges = !graph.edges_from(node.id.raw.as_str()).is_empty()
            || !graph.edges_to(node.id.raw.as_str()).is_empty();
        if !has_edges {
            findings.push(json!({
                "entity_id": node.id.raw,
                "severity": "info",
                "message": format!("Entity '{}' is an orphan (no edges)", node.id.raw)
            }));
        }
    }

    let payload = json!({
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

    Ok(Rendered {
        instruction,
        payload,
    })
}
