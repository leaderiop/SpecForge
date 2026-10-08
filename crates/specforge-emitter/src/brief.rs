use serde::Serialize;
use specforge_graph::Node;

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
