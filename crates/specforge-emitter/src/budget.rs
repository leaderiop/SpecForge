use serde::Serialize;
use serde_json::Value;
use specforge_graph::Graph;
use std::collections::HashMap;

use crate::json::{field_map_to_json, sorted_edges, JsonEdge, SCHEMA_VERSION};

#[derive(Serialize)]
struct BudgetedGraph {
    schema_version: &'static str,
    nodes: Vec<BudgetedNode>,
    edges: Vec<JsonEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_budget: Option<TokenBudgetResult>,
}

#[derive(Serialize)]
struct BudgetedNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    file: String,
    line: usize,
    fields: std::collections::BTreeMap<String, Value>,
}

#[derive(Serialize)]
struct TokenBudgetResult {
    strategy: String,
    budget_tokens: usize,
    estimated_tokens: usize,
    truncated_entities: Vec<String>,
}

fn estimate_tokens(s: &str) -> usize {
    // Rough token estimate: ~4 chars per token
    s.len() / 4
}

fn degree_centrality(graph: &Graph) -> HashMap<String, usize> {
    let mut degrees: HashMap<String, usize> = HashMap::new();
    for node in graph.nodes() {
        degrees.entry(node.id.raw.clone()).or_insert(0);
    }
    for edge in graph.edges() {
        *degrees.entry(edge.source.clone()).or_insert(0) += 1;
        *degrees.entry(edge.target.clone()).or_insert(0) += 1;
    }
    degrees
}

pub fn emit_json_with_budget(graph: &Graph, max_tokens: usize) -> String {
    // First, try full output
    let full = crate::json::emit_json(graph);
    if estimate_tokens(&full) <= max_tokens {
        return full;
    }

    // Need to truncate — sort by degree centrality (ascending = least connected first)
    let degrees = degree_centrality(graph);
    let mut nodes_by_priority: Vec<_> = graph.nodes().into_iter().collect();
    nodes_by_priority.sort_by(|a, b| {
        let da = degrees.get(&a.id.raw).copied().unwrap_or(0);
        let db = degrees.get(&b.id.raw).copied().unwrap_or(0);
        da.cmp(&db).then(a.id.raw.cmp(&b.id.raw))
    });

    // Iteratively remove least-connected nodes until within budget
    let mut truncated_ids: Vec<String> = Vec::new();
    let mut kept: Vec<_> = nodes_by_priority.clone();

    loop {
        let kept_ids: std::collections::HashSet<&str> =
            kept.iter().map(|n| n.id.raw.as_str()).collect();

        let nodes: Vec<BudgetedNode> = kept
            .iter()
            .map(|n| BudgetedNode {
                id: n.id.raw.clone(),
                kind: n.kind.raw.clone(),
                title: n.title.clone(),
                file: n.source_span.file.clone(),
                line: n.source_span.start_line,
                fields: field_map_to_json(&n.fields),
            })
            .collect();

        let edges: Vec<JsonEdge> = sorted_edges(graph)
            .into_iter()
            .filter(|e| kept_ids.contains(e.source.as_str()) && kept_ids.contains(e.target.as_str()))
            .collect();

        let output = BudgetedGraph {
            schema_version: SCHEMA_VERSION,
            nodes,
            edges,
            token_budget: if truncated_ids.is_empty() {
                None
            } else {
                Some(TokenBudgetResult {
                    strategy: "prioritize".to_string(),
                    budget_tokens: max_tokens,
                    estimated_tokens: 0, // will be filled after serialization
                    truncated_entities: truncated_ids.clone(),
                })
            },
        };

        let serialized = serde_json::to_string_pretty(&output).expect("serialization cannot fail");
        let est = estimate_tokens(&serialized);

        if est <= max_tokens || kept.len() <= 1 {
            // Re-serialize with correct estimated_tokens
            let final_output = BudgetedGraph {
                schema_version: SCHEMA_VERSION,
                nodes: output.nodes,
                edges: output.edges,
                token_budget: if truncated_ids.is_empty() {
                    None
                } else {
                    Some(TokenBudgetResult {
                        strategy: "prioritize".to_string(),
                        budget_tokens: max_tokens,
                        estimated_tokens: est,
                        truncated_entities: truncated_ids,
                    })
                },
            };
            return serde_json::to_string_pretty(&final_output).expect("serialization cannot fail");
        }

        // Remove the least-connected node
        let removed = kept.remove(0);
        truncated_ids.push(removed.id.raw.clone());
    }
}

pub fn emit_json_with_budget_strategy(
    graph: &Graph,
    max_tokens: usize,
    strategy: &str,
) -> Result<String, String> {
    let full = crate::json::emit_json(graph);
    let est = estimate_tokens(&full);

    if est <= max_tokens {
        return Ok(full);
    }

    match strategy {
        "error" => Err(format!(
            "token budget exceeded: estimated {} tokens, budget is {}",
            est, max_tokens
        )),
        _ => Ok(emit_json_with_budget(graph, max_tokens)),
    }
}
