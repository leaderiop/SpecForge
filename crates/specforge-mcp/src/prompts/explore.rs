//! `specforge://prompts/explore`: where to start exploring the graph.

use serde::Deserialize;
use serde_json::Value;
use specforge_graph::Graph;
use std::collections::{HashMap, VecDeque};

use crate::prompt::{PromptArgs, PromptOutcome, Rendered};
use crate::target::Call;

#[derive(Debug, Deserialize)]
pub struct Args {
    #[serde(default)]
    entity_id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
}

impl PromptArgs for Args {
    const DESCRIPTIONS: &'static [(&'static str, &'static str)] = &[
        ("entity_id", "Starting entity (optional)"),
        ("kind", "Filter by entity kind"),
    ];
}

/// Breadth-first search from `start` over edges in both directions: one
/// `McpRelationshipPath` per entity reached, in BFS order, carrying the
/// edge labels along the shortest path. With `kind`, only paths ending at
/// an entity of that kind are kept.
fn bfs_paths(graph: &Graph, start: &str, kind: Option<&str>) -> Vec<Value> {
    if graph.node(start).is_none() {
        return Vec::new();
    }
    // Entity id -> edge labels on the shortest path from `start`.
    let mut labels: HashMap<String, Vec<String>> = HashMap::from([(start.to_string(), vec![])]);
    let mut order: Vec<String> = Vec::new();
    let mut queue = VecDeque::from([start.to_string()]);
    while let Some(current) = queue.pop_front() {
        let path = labels[&current].clone();
        let mut neighbors: Vec<(String, String)> = graph
            .edges_from(&current)
            .iter()
            .map(|e| (e.target.to_string(), e.label.to_string()))
            .chain(
                graph
                    .edges_to(&current)
                    .iter()
                    .map(|e| (e.source.to_string(), e.label.to_string())),
            )
            .collect();
        neighbors.sort();
        for (next, label) in neighbors {
            if labels.contains_key(&next) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(label);
            labels.insert(next.clone(), next_path);
            order.push(next.clone());
            queue.push_back(next);
        }
    }
    order
        .into_iter()
        .filter(|id| kind.is_none_or(|kind| graph.node(id).is_some_and(|n| n.kind.raw == kind)))
        .map(|id| {
            let edge_types = &labels[&id];
            serde_json::json!({
                "from_entity": start,
                "to_entity": id,
                "edge_types": edge_types,
                "path_length": edge_types.len(),
            })
        })
        .collect()
}

pub fn render(call: &Call<'_>, args: Args) -> PromptOutcome {
    let graph = call.view().graph;
    let entity_filter = args.entity_id.as_deref();
    let kind_filter = args.kind.as_deref();

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

    let relationship_paths = match entity_filter {
        Some(start) => bfs_paths(graph, start, kind_filter),
        None => Vec::new(),
    };

    let payload = serde_json::json!({
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
