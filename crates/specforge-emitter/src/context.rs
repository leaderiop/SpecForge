use serde::Serialize;
use serde_json::Value;
use specforge_graph::{Graph, Node};
use specforge_registry::FieldRegistry;
use std::collections::BTreeMap;

use crate::json::{JsonEdge, SCHEMA_VERSION, sorted_edges};

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
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, Value>,
}

/// Fields a context node carries at its top level, never repeated in
/// `fields`.
const TOP_LEVEL_FIELDS: [&str; 3] = ["contract", "status", "verify"];

/// The fields of `node` its extension declares normative (the text that
/// states what the entity promises), except those a context node already
/// carries at its top level. Empty without a registry.
pub(crate) fn normative_fields(
    node: &Node,
    registry: Option<&FieldRegistry>,
) -> BTreeMap<String, Value> {
    let Some(registry) = registry else {
        return BTreeMap::new();
    };
    node.fields
        .entries()
        .iter()
        .filter(|entry| !TOP_LEVEL_FIELDS.contains(&entry.key.as_str()))
        .filter(|entry| {
            registry
                .get(node.kind.raw.as_str(), entry.key.as_str())
                .is_some_and(|field| field.normative)
        })
        .map(|entry| {
            (
                entry.key.to_string(),
                crate::json::field_value_to_json(&entry.value),
            )
        })
        .collect()
}

pub fn emit_context(graph: &Graph) -> String {
    emit_context_with_fields(graph, None)
}

/// The context export, with each entity's normative fields when the field
/// registry that declares them is given.
pub fn emit_context_with_fields(graph: &Graph, registry: Option<&FieldRegistry>) -> String {
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
            let verify = n.fields.get("verify").map(crate::json::field_value_to_json);

            ContextNode {
                id: n.id.raw.to_string(),
                kind: n.kind.raw.to_string(),
                title: n.title.clone(),
                contract,
                status,
                verify,
                fields: normative_fields(n, registry),
            }
        })
        .collect();

    let output = ContextGraph {
        schema_version: SCHEMA_VERSION,
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string(&output).expect("graph serialization cannot fail")
}
