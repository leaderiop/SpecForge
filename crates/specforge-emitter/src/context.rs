use serde::Serialize;
use serde_json::Value;
use specforge_graph::Graph;

use crate::json::{sorted_edges, JsonEdge, SCHEMA_VERSION};

#[derive(Serialize)]
struct ContextGraph {
    schema_version: &'static str,
    nodes: Vec<ContextNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct ContextNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<Value>,
}

pub fn emit_context(graph: &Graph) -> String {
    let nodes: Vec<ContextNode> = graph
        .nodes()
        .iter()
        .map(|n| {
            let contract = n.fields.get("contract").and_then(|v| match v {
                specforge_graph::FieldValue::String(s) => Some(s.clone()),
                _ => None,
            });
            let status = n.fields.get("status").and_then(|v| match v {
                specforge_graph::FieldValue::Identifier(s) => Some(s.clone()),
                specforge_graph::FieldValue::String(s) => Some(s.clone()),
                _ => None,
            });
            let verify = n.fields.get("verify").map(|v| {
                crate::json::field_value_to_json(v)
            });

            ContextNode {
                id: n.id.raw.clone(),
                kind: n.kind.raw.clone(),
                title: n.title.clone(),
                contract,
                status,
                verify,
            }
        })
        .collect();

    let output = ContextGraph {
        schema_version: SCHEMA_VERSION,
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}
