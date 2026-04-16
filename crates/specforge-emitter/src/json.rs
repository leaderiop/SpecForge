use serde::Serialize;
use serde_json::Value;
use specforge_graph::{FieldMap, FieldValue, Graph};
use std::collections::BTreeMap;

pub(crate) const SCHEMA_VERSION: &str = "0.1.0";

#[derive(Serialize)]
struct JsonGraph {
    schema_version: &'static str,
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct JsonNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    file: String,
    line: usize,
    fields: BTreeMap<String, Value>,
}

#[derive(Serialize)]
pub(crate) struct JsonEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

pub(crate) fn field_map_to_json(fields: &FieldMap) -> BTreeMap<String, Value> {
    let mut map = BTreeMap::new();
    for entry in fields.entries() {
        map.insert(entry.key.to_string(), field_value_to_json(&entry.value));
    }
    map
}

pub(crate) fn field_value_to_json(value: &FieldValue) -> Value {
    match value {
        FieldValue::String(s) => Value::String(s.clone()),
        FieldValue::Identifier(s) => Value::String(s.clone()),
        FieldValue::Integer(n) => Value::Number((*n).into()),
        FieldValue::Boolean(b) => Value::Bool(*b),
        FieldValue::Date(s) => Value::String(s.clone()),
        FieldValue::ReferenceList(refs) => {
            Value::Array(refs.iter().map(|r: &String| Value::String(r.clone())).collect())
        }
        FieldValue::VariantList(variants) => {
            Value::Array(variants.iter().map(|v: &String| Value::String(v.clone())).collect())
        }
        FieldValue::StringList(list) => {
            Value::Array(list.iter().map(|s: &String| Value::String(s.clone())).collect())
        }
        FieldValue::VerifyList(stmts) => Value::Array(
            stmts
                .iter()
                .map(|v| {
                    serde_json::json!({
                        "kind": v.kind,
                        "description": v.description,
                    })
                })
                .collect(),
        ),
        FieldValue::Block(inner) => {
            Value::Object(field_map_to_json(inner).into_iter().collect())
        }
        FieldValue::MixedList(items) => {
            Value::Array(items.iter().map(field_value_to_json).collect())
        }
    }
}

pub(crate) fn sorted_edges(graph: &Graph) -> Vec<JsonEdge> {
    let mut edges: Vec<JsonEdge> = graph
        .edges()
        .iter()
        .map(|e| JsonEdge {
            source: e.source.to_string(),
            target: e.target.to_string(),
            label: e.label.to_string(),
        })
        .collect();
    edges.sort_by(|a, b| (&a.source, &a.target, &a.label).cmp(&(&b.source, &b.target, &b.label)));
    edges
}

pub fn emit_json(graph: &Graph) -> String {
    let nodes: Vec<JsonNode> = graph
        .nodes()
        .iter()
        .map(|n| JsonNode {
            id: n.id.raw.to_string(),
            kind: n.kind.raw.to_string(),
            title: n.title.clone(),
            file: n.source_span.file.to_string(),
            line: n.source_span.start_line,
            fields: field_map_to_json(&n.fields),
        })
        .collect();

    let output = JsonGraph {
        schema_version: SCHEMA_VERSION,
        nodes,
        edges: sorted_edges(graph),
    };

    serde_json::to_string_pretty(&output).expect("graph serialization cannot fail")
}
