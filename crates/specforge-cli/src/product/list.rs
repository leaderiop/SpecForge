use serde::Serialize;
use specforge_graph::Graph;

#[derive(Debug, Default)]
pub struct ListFilter {
    pub kind: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ListResult {
    pub entities: Vec<ListEntity>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ListEntity {
    pub id: String,
    pub title: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    pub incoming_edges: usize,
    pub outgoing_edges: usize,
}

pub fn list_entities(graph: &Graph, filter: &ListFilter) -> ListResult {
    let mut entities: Vec<ListEntity> = graph
        .nodes()
        .into_iter()
        .filter(|n| n.kind.raw == *filter.kind)
        .filter(|n| {
            if let Some(ref status) = filter.status {
                get_field_value(n, "status").as_deref() == Some(status.as_str())
            } else {
                true
            }
        })
        .filter(|n| {
            if let Some(ref priority) = filter.priority {
                get_field_value(n, "priority").as_deref() == Some(priority.as_str())
            } else {
                true
            }
        })
        .map(|n| {
            ListEntity {
                id: n.id.raw.to_string(),
                title: n.title.clone(),
                kind: n.kind.raw.to_string(),
                status: get_field_value(n, "status"),
                priority: get_field_value(n, "priority"),
                incoming_edges: graph.edges_to(n.id.raw.as_str()).len(),
                outgoing_edges: graph.edges_from(n.id.raw.as_str()).len(),
            }
        })
        .collect();

    entities.sort_by(|a, b| a.id.cmp(&b.id));
    let total = entities.len();

    if let Some(offset) = filter.offset {
        entities = entities.into_iter().skip(offset).collect();
    }
    if let Some(limit) = filter.limit {
        entities.truncate(limit);
    }

    ListResult { entities, total }
}

fn get_field_value(node: &specforge_graph::Node, field_name: &str) -> Option<String> {
    node.fields.entries().iter().find_map(|e| {
        if e.key == field_name {
            match &e.value {
                specforge_parser::FieldValue::String(s) => Some(s.clone()),
                specforge_parser::FieldValue::Identifier(s) => Some(s.clone()),
                _ => None,
            }
        } else {
            None
        }
    })
}
