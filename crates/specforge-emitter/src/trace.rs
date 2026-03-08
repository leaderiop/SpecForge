use serde::Serialize;
use specforge_graph::Graph;
use std::collections::{HashSet, VecDeque};

use crate::json::SCHEMA_VERSION;

#[derive(Debug, Serialize)]
pub struct TraceChain {
    pub entity_id: String,
    pub entity_kind: String,
    pub upstream: Vec<TraceLink>,
    pub downstream: Vec<TraceLink>,
}

#[derive(Debug, Serialize)]
pub struct TraceLink {
    pub entity_id: String,
    pub entity_kind: String,
    pub edge_label: String,
    pub depth: usize,
}

pub fn trace(graph: &Graph, entity_id: &str) -> Result<TraceChain, String> {
    let root = graph.node(entity_id).ok_or_else(|| {
        format!("E001: unresolved entity '{}' — not found in graph", entity_id)
    })?;

    let upstream = directed_bfs(graph, entity_id, Direction::Upstream);
    let downstream = directed_bfs(graph, entity_id, Direction::Downstream);

    Ok(TraceChain {
        entity_id: entity_id.to_string(),
        entity_kind: root.kind.raw.clone(),
        upstream,
        downstream,
    })
}

pub fn trace_all(graph: &Graph) -> Vec<TraceChain> {
    let mut chains: Vec<TraceChain> = graph
        .nodes()
        .iter()
        .filter_map(|n| trace(graph, &n.id.raw).ok())
        .collect();
    chains.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    chains
}

pub fn detect_trace_gaps(graph: &Graph) -> Vec<String> {
    let node_ids: HashSet<&str> = graph.nodes().iter().map(|n| n.id.raw.as_str()).collect();
    let mut gaps = Vec::new();
    for edge in graph.edges() {
        if !node_ids.contains(edge.source.as_str()) {
            gaps.push(format!(
                "dangling edge source '{}' in edge {} -> {} ({})",
                edge.source, edge.source, edge.target, edge.label
            ));
        }
        if !node_ids.contains(edge.target.as_str()) {
            gaps.push(format!(
                "dangling edge target '{}' in edge {} -> {} ({})",
                edge.target, edge.source, edge.target, edge.label
            ));
        }
    }
    gaps.sort();
    gaps.dedup();
    gaps
}

pub fn serialize_trace_all(chains: &[TraceChain]) -> String {
    #[derive(Serialize)]
    struct TraceAllOutput<'a> {
        schema_version: &'static str,
        traces: &'a [TraceChain],
    }

    let output = TraceAllOutput {
        schema_version: SCHEMA_VERSION,
        traces: chains,
    };

    serde_json::to_string_pretty(&output).expect("trace_all serialization cannot fail")
}

pub fn serialize_trace(chain: &TraceChain) -> String {
    #[derive(Serialize)]
    struct TraceOutput<'a> {
        schema_version: &'static str,
        entity_id: &'a str,
        entity_kind: &'a str,
        upstream: &'a [TraceLink],
        downstream: &'a [TraceLink],
    }

    let output = TraceOutput {
        schema_version: SCHEMA_VERSION,
        entity_id: &chain.entity_id,
        entity_kind: &chain.entity_kind,
        upstream: &chain.upstream,
        downstream: &chain.downstream,
    };

    serde_json::to_string_pretty(&output).expect("trace serialization cannot fail")
}

enum Direction {
    Upstream,
    Downstream,
}

fn directed_bfs(graph: &Graph, start: &str, direction: Direction) -> Vec<TraceLink> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut links = Vec::new();

    visited.insert(start.to_string());
    queue.push_back((start.to_string(), 0usize));

    while let Some((current, depth)) = queue.pop_front() {
        for edge in graph.edges() {
            let (neighbor, label) = match direction {
                Direction::Upstream => {
                    if edge.target == current {
                        (&edge.source, &edge.label)
                    } else {
                        continue;
                    }
                }
                Direction::Downstream => {
                    if edge.source == current {
                        (&edge.target, &edge.label)
                    } else {
                        continue;
                    }
                }
            };

            if visited.insert(neighbor.clone()) {
                if let Some(node) = graph.node(neighbor) {
                    links.push(TraceLink {
                        entity_id: neighbor.clone(),
                        entity_kind: node.kind.raw.clone(),
                        edge_label: label.clone(),
                        depth: depth + 1,
                    });
                    queue.push_back((neighbor.clone(), depth + 1));
                }
            }
        }
    }

    links.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.entity_id.cmp(&b.entity_id)));
    links
}
