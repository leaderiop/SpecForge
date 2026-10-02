use serde::Serialize;
use serde_json::Value;
use specforge_graph::{FieldValue, Graph, Node};
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
    /// The fields an extension declares `headline` (a contract, a status),
    /// by name.
    #[serde(flatten)]
    headline: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    verify: Option<Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, Value>,
}

/// Keys a context node has of its own, which no headline field may take.
const NODE_KEYS: [&str; 5] = ["id", "kind", "title", "verify", "fields"];

/// The fields of `node` its extension declares `headline`, as text, for the
/// context node's top level. Empty without a registry.
pub(crate) fn headline_fields(
    node: &Node,
    registry: Option<&FieldRegistry>,
) -> BTreeMap<String, Value> {
    let Some(registry) = registry else {
        return BTreeMap::new();
    };
    node.fields
        .entries()
        .iter()
        .filter(|entry| is_headline(node, entry.key.as_str(), registry))
        .filter(|entry| !NODE_KEYS.contains(&entry.key.as_str()))
        .filter_map(|entry| match &entry.value {
            FieldValue::String(s) | FieldValue::Identifier(s) => {
                Some((entry.key.to_string(), Value::String(s.clone())))
            }
            _ => None,
        })
        .collect()
}

fn is_headline(node: &Node, field: &str, registry: &FieldRegistry) -> bool {
    registry
        .get(node.kind.raw.as_str(), field)
        .is_some_and(|f| f.headline)
}

/// The fields of `node` its extension declares normative (the text that
/// states what the entity promises), except those a context node already
/// carries at its top level (its headline fields and obligations). Empty
/// without a registry.
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
        .filter(|entry| !matches!(entry.value, FieldValue::VerifyList(_)))
        .filter(|entry| !is_headline(node, entry.key.as_str(), registry))
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
            let verify = crate::json::obligations_json(n);

            ContextNode {
                id: n.id.raw.to_string(),
                kind: n.kind.raw.to_string(),
                title: n.title.clone(),
                headline: headline_fields(n, registry),
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
