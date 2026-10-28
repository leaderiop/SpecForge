//! The value bytes a ledger digest, a history check and a pass's `previous` share (ADR 0287): a field's value as
//! every output writes it, except that each reference is the full path of its target (so `phase 03` and
//! `phase v1.0/03` are one value), serialized compactly with sorted keys.

use serde_json::{Map, Value, json};
use specforge_common::Sym;
use specforge_parser::{FieldValue, VerdictMember, Written};

use crate::field_form::{FieldForm, FieldJson, write};
use crate::members::MethodJson;
use crate::scope::Found;
use crate::{FieldForms, Graph, Node, Reading};
use specforge_protocol_types::{FieldType, FieldWriter};

/// `field` of `node` as the ledger and the history checks read it: [`write`], with each reference item (inside a
/// typed block or block list too) the full path of its target, or its written text when it names none. `None` when
/// the node writes no such field.
pub fn resolved(graph: &Graph, forms: &FieldForms, node: &Node, field: &str) -> Option<FieldJson> {
    let entry = node
        .fields
        .entries()
        .iter()
        .find(|e| e.key.as_str() == field)?;
    let form = match forms.reading(node.kind.raw.as_str(), field) {
        Reading::Declared(form) => Some(form),
        _ => None,
    };
    Some(FieldJson::of(walk(graph, node, &entry.value, form)))
}

/// The path `text` names for a reference of `form` held by `node`, as the graph's link resolves it: by the node's
/// scope chain among the kinds the field takes, else (the scope field, which has no chain yet) from the project, where
/// a unique suffix names it (`phase 03` and `phase v1.0/03` are one value). Else the text as written.
fn target(graph: &Graph, node: &Node, form: &FieldForm, text: &str) -> String {
    let kinds = form.target_kinds();
    let admits = |kind: &str| kinds.is_empty() || kinds.iter().any(|k| k == kind);
    let from = node.id.raw;
    match graph.scopes().chain(Some(from), text, &admits) {
        Found::One(path) | Found::Mistyped(path) => return path.to_string(),
        _ => {}
    }
    match graph.scopes().resolve(None, text, &admits) {
        Found::One(path) if Some(path) == node.scope => path.to_string(),
        _ => text.to_string(),
    }
}

/// `value` as JSON under `form`, references resolved for `node`.
fn walk(graph: &Graph, node: &Node, value: &FieldValue, form: Option<&FieldForm>) -> Value {
    let Some(form) = form else {
        return write(value).as_value().clone();
    };
    match (form.field_type(), value) {
        (FieldType::Reference, FieldValue::Identifier(text) | FieldValue::String(text)) => {
            Value::String(target(graph, node, form, text))
        }
        (FieldType::ReferenceList, FieldValue::List(items)) => Value::Array(
            items
                .iter()
                .map(|i| Value::String(target(graph, node, form, &i.text)))
                .collect(),
        ),
        (FieldType::Block, FieldValue::Block(block)) => walk_entries(graph, node, block, form),
        (FieldType::BlockList, FieldValue::Blocks(items)) => Value::Array(
            items
                .iter()
                .map(|item| walk_entries(graph, node, &item.fields, form))
                .collect(),
        ),
        _ => write(value).as_value().clone(),
    }
}

/// A block's entries, each under its member's form; the first of a key written twice.
fn walk_entries(
    graph: &Graph,
    node: &Node,
    block: &specforge_parser::FieldMap,
    form: &FieldForm,
) -> Value {
    let mut out = Map::new();
    for entry in block.entries() {
        let key = entry.key.to_string();
        if out.contains_key(&key) {
            continue;
        }
        let member = form.members().and_then(|m| m.get(&key));
        out.insert(key, walk(graph, node, &entry.value, member));
    }
    Value::Object(out)
}

/// The bytes a digest is taken over: the JSON, compact, keys sorted.
pub fn bytes(value: &FieldJson) -> Vec<u8> {
    serde_json::to_vec(value.as_value()).expect("a JSON value serializes")
}

/// The bytes an entity's digest is taken over (ADR 0290): its id, kind and title, its fields as [`resolved`] writes
/// them, its `verify` statements and its method signatures. A field a compute pass or a verdict gives is left out, and
/// so are the entity's own verdict statements, positions and comments: what a judge read is what its author wrote.
pub fn entity_bytes(graph: &Graph, forms: &FieldForms, node: &Node) -> Vec<u8> {
    let kind = node.kind.raw.as_str();
    let mut fields = Map::new();
    for entry in node.fields.entries() {
        let name = entry.key.as_str();
        let given = matches!(
            forms.reading(kind, name),
            Reading::Declared(form)
                if matches!(form.writer(), Some(FieldWriter::Computed | FieldWriter::Verdict))
        );
        if given || fields.contains_key(name) {
            continue;
        }
        if let Some(value) = resolved(graph, forms, node, name) {
            fields.insert(name.to_string(), value.as_value().clone());
        }
    }
    let value = json!({
        "id": node.id.raw.as_str(),
        "kind": kind,
        "title": node.title,
        "fields": fields,
        "obligations": node.obligations.iter()
            .map(|o| json!({"kind": o.kind, "text": o.description}))
            .collect::<Vec<_>>(),
        "methods": node.methods.iter()
            .map(|m| MethodJson::of(m).signature())
            .collect::<Vec<_>>(),
    });
    serde_json::to_vec(&value).expect("a JSON value serializes")
}

/// The path a verdict held by `holder` cites with `text`: by the holder's scope chain, else the one entity a surface
/// spelling names (the resolution that accepted the cite, [`Graph::find`]), else the text as written. The ledger bytes and
/// the cited digest both read a cite through it.
pub fn cited(graph: &Graph, holder: Sym, text: &str) -> String {
    match graph.scopes().chain(Some(holder), text, &|_| true) {
        Found::One(path) | Found::Mistyped(path) => return path.to_string(),
        _ => {}
    }
    match graph.find(text) {
        Found::One(path) | Found::Mistyped(path) => path.to_string(),
        _ => text.to_string(),
    }
}

/// The bytes a verdict is recorded as in the ledger (ADR 0290): its rule and outcome, its judge, model and time,
/// what it cited (a bare item as the full path of the entity it names, a quoted one a file, `cited_tests` the
/// tests), its input digests, findings and values. Layout, comments and the spelling of a cited reference are not
/// in it, so a respelled `cited` records the same bytes.
pub fn verdict_bytes(graph: &Graph, node: &Node, verdict: &VerdictMember) -> Vec<u8> {
    let member = |key: &str| verdict.body.entry(key).map(|e| write(&e.value));
    let text = |key: &str| {
        member(key)
            .map(|v| v.as_value().clone())
            .unwrap_or(Value::Null)
    };
    let items = |key: &str, bare: Option<bool>| -> Vec<Value> {
        let Some(FieldValue::List(items)) = verdict.body.get(key) else {
            return Vec::new();
        };
        items
            .iter()
            .filter(|item| bare.is_none_or(|bare| (item.written == Written::Bare) == bare))
            .map(|item| Value::String(item.text.clone()))
            .collect()
    };
    let mut entities: Vec<String> = items("cited", Some(true))
        .into_iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .map(|text| cited(graph, node.id.raw, &text))
        .collect();
    entities.sort();
    let mut files: Vec<Value> = items("cited", Some(false));
    files.sort_by_key(|v| v.to_string());
    let mut tests = items("cited_tests", None);
    tests.sort_by_key(|v| v.to_string());
    let value = json!({
        "rule": verdict.rule,
        "outcome": verdict.outcome,
        "judge": text("judge"),
        "model": text("model"),
        "at": text("at"),
        "cited": {"entities": entities, "files": files, "tests": tests},
        "inputs": member("inputs").map_or_else(|| json!({}), |v| v.as_value().clone()),
        "findings": member("findings").map_or_else(|| json!([]), |v| v.as_value().clone()),
        "values": member("values").map_or_else(|| json!({}), |v| v.as_value().clone()),
    });
    serde_json::to_vec(&value).expect("a JSON value serializes")
}
