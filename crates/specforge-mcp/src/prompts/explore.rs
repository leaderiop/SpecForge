//! `specforge://prompts/explore`: where to start exploring the graph.

use serde_json::{Value, json};
use specforge_graph::{Graph, Reached};

use crate::args::Arguments;
use crate::prompt::{PromptOutcome, Rendered};
use crate::target::Call;
use crate::tool::entity_not_found;

/// `specforge://prompts/explore`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Starting entity (optional)
    entity_id: Option<String>,
    /// Filter by entity kind
    kind: Option<String>,
    /// Hops from entity_id the exploration reaches (unbounded if omitted)
    depth: Option<usize>,
}

/// One `McpRelationshipPath` per entity [`Graph::reach`] reached from
/// the root (the root itself aside), nearest first, carrying the edge
/// labels on the path to it. With `kind`, only paths ending at an entity
/// of that kind are kept.
fn relationship_paths(graph: &Graph, reached: &[Reached], kind: Option<&str>) -> Vec<Value> {
    let Some(root) = reached.first() else {
        return Vec::new();
    };
    reached[1..]
        .iter()
        .filter(|r| {
            kind.is_none_or(|kind| {
                graph
                    .node(r.id.as_str())
                    .is_some_and(|n| n.kind.raw == kind)
            })
        })
        .map(|r| {
            let edge_types: Vec<&str> = Graph::reach_path(reached, r.id)
                .iter()
                .map(|label| label.as_str())
                .collect();
            json!({
                "from_entity": root.id.as_str(),
                "to_entity": r.id.as_str(),
                "edge_types": edge_types,
                "path_length": edge_types.len(),
            })
        })
        .collect()
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let graph = call.view().graph();
    let entity_filter = args.entity_id.as_deref();
    let kind_filter = args.kind.as_deref();
    // The entities reached from entity_id, as review's neighbourhood is.
    let reached = match entity_filter {
        Some(start) => Some(
            graph
                .reach(start, args.depth)
                .ok_or_else(|| entity_not_found(graph, start))?,
        ),
        None => None,
    };

    let matching: Vec<String> = graph
        .nodes()
        .into_iter()
        .filter(|n| {
            if let Some(eid) = entity_filter
                && n.id.raw != eid
            {
                return false;
            }
            if let Some(kind) = kind_filter
                && n.kind.raw != kind
            {
                return false;
            }
            true
        })
        .map(|n| n.id.raw.to_string())
        .collect();

    // High connectivity: nodes with most edges (exclude zero-edge nodes)
    let mut connectivity: Vec<(String, usize)> = graph
        .nodes()
        .into_iter()
        .map(|n| {
            let count =
                graph.edges_from(n.id.raw.as_str()).len() + graph.edges_to(n.id.raw.as_str()).len();
            (n.id.raw.to_string(), count)
        })
        .collect();
    connectivity.sort_by_key(|a| std::cmp::Reverse(a.1));
    let high_connectivity: Vec<String> = connectivity
        .iter()
        .filter(|(_, count)| *count > 0)
        .take(10)
        .map(|(id, _)| id.clone())
        .collect();

    // Orphans: no edges at all
    let orphans: Vec<String> = connectivity
        .iter()
        .filter(|(_, count)| *count == 0)
        .map(|(id, _)| id.clone())
        .collect();

    // Starting points: high out-degree, low in-degree
    let mut starting_points: Vec<(String, i64)> = graph
        .nodes()
        .into_iter()
        .map(|n| {
            let out = graph.edges_from(n.id.raw.as_str()).len() as i64;
            let in_ = graph.edges_to(n.id.raw.as_str()).len() as i64;
            (n.id.raw.to_string(), out - in_)
        })
        .collect();
    starting_points.sort_by_key(|a| std::cmp::Reverse(a.1));
    let starting_points: Vec<String> = starting_points
        .iter()
        .take(5)
        .map(|(id, _)| id.clone())
        .collect();

    let relationship_paths = reached
        .as_deref()
        .map(|reached| relationship_paths(graph, reached, kind_filter))
        .unwrap_or_default();

    let payload = json!({
        "matching_entities": matching,
        "relationship_paths": relationship_paths,
        "starting_points": starting_points,
        "high_connectivity": high_connectivity,
        "orphan_nodes": orphans
    });

    let instruction = "Explore the spec graph using the data below. \
         Start with high-connectivity nodes to understand the core structure, \
         then investigate orphan nodes that may need relationships. \
         Use starting_points for top-down traversal.";

    Ok(Rendered {
        instruction: instruction.to_string(),
        payload,
    })
}
