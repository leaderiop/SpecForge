//! The value bytes a ledger digest, a history check and a pass's `previous` share (ADR 0287): a field's value as
//! every output writes it, except that each reference is the full path of its target (so `phase 03` and
//! `phase v1.0/03` are one value), serialized compactly with sorted keys.

use serde_json::{Map, Value};
use specforge_common::Sym;
use specforge_parser::FieldValue;

use crate::field_form::{FieldForm, FieldJson, write};
use crate::scope::Found;
use crate::{FieldForms, Graph, Node, Reading};
use specforge_protocol_types::FieldType;

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
    Some(FieldJson::of(walk(graph, node.id.raw, &entry.value, form)))
}

/// `value` as JSON under `form`, references resolved from `from`.
fn walk(graph: &Graph, from: Sym, value: &FieldValue, form: Option<&FieldForm>) -> Value {
    let target = |text: &str| match graph.scopes().resolve(Some(from), text, &|_| true) {
        Found::One(path) | Found::Mistyped(path) => path.to_string(),
        _ => text.to_string(),
    };
    let Some(form) = form else {
        return write(value).as_value().clone();
    };
    match (form.field_type(), value) {
        (FieldType::Reference, FieldValue::Identifier(text) | FieldValue::String(text)) => {
            Value::String(target(text))
        }
        (FieldType::ReferenceList, FieldValue::List(items)) => Value::Array(
            items
                .iter()
                .map(|i| Value::String(target(&i.text)))
                .collect(),
        ),
        (FieldType::Block, FieldValue::Block(block)) => walk_entries(graph, from, block, form),
        (FieldType::BlockList, FieldValue::Blocks(items)) => Value::Array(
            items
                .iter()
                .map(|item| walk_entries(graph, from, &item.fields, form))
                .collect(),
        ),
        _ => write(value).as_value().clone(),
    }
}

/// A block's entries, each under its member's form; the first of a key written twice.
fn walk_entries(
    graph: &Graph,
    from: Sym,
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
        out.insert(key, walk(graph, from, &entry.value, member));
    }
    Value::Object(out)
}

/// The bytes a digest is taken over: the JSON, compact, keys sorted.
pub fn bytes(value: &FieldJson) -> Vec<u8> {
    serde_json::to_vec(value.as_value()).expect("a JSON value serializes")
}
