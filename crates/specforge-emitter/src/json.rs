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
        map.insert(entry.key.clone(), field_value_to_json(&entry.value));
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
    }
}

pub(crate) fn sorted_edges(graph: &Graph) -> Vec<JsonEdge> {
    let mut edges: Vec<JsonEdge> = graph
        .edges()
        .iter()
        .map(|e| JsonEdge {
            source: e.source.clone(),
            target: e.target.clone(),
            label: e.label.clone(),
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
            id: n.id.raw.clone(),
            kind: n.kind.raw.clone(),
            title: n.title.clone(),
            file: n.source_span.file.clone(),
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
