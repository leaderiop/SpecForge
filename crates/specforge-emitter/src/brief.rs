use serde::Serialize;
use specforge_graph::Graph;

use crate::json::{sorted_edges, JsonEdge, SCHEMA_VERSION};

#[derive(Serialize)]
struct BriefGraph {
    schema_version: &'static str,
    nodes: Vec<BriefNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct BriefNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

/// Emit a brief (minimal) JSON representation of the graph.
///
/// Returns the JSON string, or an error if serialization fails.
pub fn emit_brief(graph: &Graph) -> String {
    let nodes: Vec<BriefNode> = graph
        .nodes()
        .iter()
        .map(|n| BriefNode {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
        })
        .collect();

    let output = BriefGraph {
        schema_version: SCHEMA_VERSION,
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}
