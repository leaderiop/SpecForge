use serde_json::Value;
use std::collections::{HashMap, VecDeque};

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

/// Breadth-first search from `start` over edges in both directions: one
/// `McpRelationshipPath` per entity reached, in BFS order, carrying the
/// edge labels along the shortest path. With `kind`, only paths ending at
/// an entity of that kind are kept.
fn bfs_paths(state: &McpState, start: &str, kind: Option<&str>) -> Vec<Value> {
    if state.graph().node(start).is_none() {
        return Vec::new();
    }
    // Entity id -> edge labels on the shortest path from `start`.
    let mut labels: HashMap<String, Vec<String>> = HashMap::from([(start.to_string(), vec![])]);
    let mut order: Vec<String> = Vec::new();
    let mut queue = VecDeque::from([start.to_string()]);
    while let Some(current) = queue.pop_front() {
        let path = labels[&current].clone();
        let mut neighbors: Vec<(String, String)> = state
            .graph()
            .edges_from(&current)
            .iter()
            .map(|e| (e.target.to_string(), e.label.to_string()))
            .chain(
                state
                    .graph()
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
        .filter(|id| {
            kind.is_none_or(|kind| state.graph().node(id).is_some_and(|n| n.kind.raw == kind))
        })
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

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let matching: Vec<String> = state
        .graph()
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
    let mut connectivity: Vec<(String, usize)> = state
        .graph()
        .nodes()
        .into_iter()
        .map(|n| {
            let count = state.graph().edges_from(n.id.raw.as_str()).len()
                + state.graph().edges_to(n.id.raw.as_str()).len();
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
    let mut starting_points: Vec<(String, i64)> = state
        .graph()
        .nodes()
        .into_iter()
        .map(|n| {
            let out = state.graph().edges_from(n.id.raw.as_str()).len() as i64;
            let in_ = state.graph().edges_to(n.id.raw.as_str()).len() as i64;
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
        Some(start) => bfs_paths(state, start, kind_filter),
        None => Vec::new(),
    };

    let result = serde_json::json!({
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

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "messages": [
                {
                    "role": "user",
                    "content": { "type": "text", "text": instruction }
                },
                {
                    "role": "assistant",
                    "content": { "type": "text", "text": result.to_string() }
                }
            ]
        }),
    )
}
