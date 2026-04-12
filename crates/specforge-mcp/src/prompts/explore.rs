use serde_json::Value;

use crate::protocol::JsonRpcResponse;
use crate::state::McpState;

pub fn get(state: &McpState, args: Value, id: Option<Value>) -> JsonRpcResponse {
    let entity_filter = args.get("entity_id").and_then(|v| v.as_str());
    let kind_filter = args.get("kind").and_then(|v| v.as_str());

    let matching: Vec<String> = state.graph.nodes().into_iter()
        .filter(|n| {
            if let Some(eid) = entity_filter
                && n.id.raw != eid { return false; }
            if let Some(kind) = kind_filter
                && n.kind.raw != kind { return false; }
            true
        })
        .map(|n| n.id.raw.to_string())
        .collect();

    // High connectivity: nodes with most edges (exclude zero-edge nodes)
    let mut connectivity: Vec<(String, usize)> = state.graph.nodes().into_iter()
        .map(|n| {
            let count = state.graph.edges_from(n.id.raw.as_str()).len() + state.graph.edges_to(n.id.raw.as_str()).len();
            (n.id.raw.to_string(), count)
        })
        .collect();
    connectivity.sort_by(|a, b| b.1.cmp(&a.1));
    let high_connectivity: Vec<String> = connectivity.iter()
        .filter(|(_, count)| *count > 0)
        .take(10)
        .map(|(id, _)| id.clone())
        .collect();

    // Orphans: no edges at all
    let orphans: Vec<String> = connectivity.iter()
        .filter(|(_, count)| *count == 0)
        .map(|(id, _)| id.clone())
        .collect();

    // Starting points: high out-degree, low in-degree
    let mut starting_points: Vec<(String, i64)> = state.graph.nodes().into_iter()
        .map(|n| {
            let out = state.graph.edges_from(n.id.raw.as_str()).len() as i64;
            let in_ = state.graph.edges_to(n.id.raw.as_str()).len() as i64;
            (n.id.raw.to_string(), out - in_)
        })
        .collect();
    starting_points.sort_by(|a, b| b.1.cmp(&a.1));
    let starting_points: Vec<String> = starting_points.iter().take(5).map(|(id, _)| id.clone()).collect();

    let result = serde_json::json!({
        "matching_entities": matching,
        "relationship_paths": [],
        "starting_points": starting_points,
        "high_connectivity": high_connectivity,
        "orphan_nodes": orphans
    });

    let instruction = "Explore the spec graph using the data below. \
         Start with high-connectivity nodes to understand the core structure, \
         then investigate orphan nodes that may need relationships. \
         Use starting_points for top-down traversal.";

    JsonRpcResponse::success(id, serde_json::json!({
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
    }))
}
