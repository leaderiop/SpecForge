use serde::Serialize;
use specforge_graph::{Graph, Node};

use crate::json::Export;

#[derive(Serialize)]
pub(crate) struct BriefNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

/// `n` as the brief export writes an entity: its id, kind and title.
pub(crate) fn brief_node(n: &Node) -> BriefNode {
    BriefNode {
        id: n.id.raw.to_string(),
        kind: n.kind.raw.to_string(),
        title: n.title.clone(),
    }
}

/// Emit a brief (minimal) JSON representation of the graph.
pub fn emit_brief(graph: &Graph) -> String {
    let nodes = graph.nodes().into_iter().map(brief_node).collect();
    Export::plain(None, graph, nodes)
        .to_json()
        .expect("graph serialization cannot fail")
}
