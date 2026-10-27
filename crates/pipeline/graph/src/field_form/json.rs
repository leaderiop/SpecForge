//! The JSON every output writes for a field value (ADR 0064): [`write`] and its schema [`FieldJson`].

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value, json};
use specforge_common::shape::{Definitions, Shape};
use specforge_parser::{FieldMap, FieldValue};

/// A field value as every output writes it: the Graph Protocol's field
/// value. Made only by [`write`]. Its shape (named `FieldJson`) is the schema
/// of every value `write` can produce, declared or not: a string, an
/// integer, a boolean, an array of strings, integers, booleans and objects, or an object whose values are `FieldJson`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FieldJson(Value);

impl FieldJson {
    /// A value a form admitted ([`FieldForm::admit_json`](super::FieldForm::admit_json)).
    pub(crate) fn of(value: Value) -> Self {
        Self(value)
    }

    /// The JSON.
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// Set the value at `path` below this field's own (see [`set_path`]).
    pub fn set_path(&mut self, path: &str, value: &FieldJson) {
        set_path(&mut self.0, path, &value.0);
    }
}

impl From<FieldJson> for Value {
    fn from(written: FieldJson) -> Self {
        written.0
    }
}

impl Shape for FieldJson {
    const NAME: Option<&'static str> = Some("FieldJson");

    fn body(defs: &mut Definitions) -> Value {
        let this = defs.reference::<FieldJson>("FieldJson");
        json!({"anyOf": [
            {"type": "string"},
            {"type": "integer"},
            {"type": "boolean"},
            {"type": "array", "items": {"anyOf": [
                {"type": "string"},
                {"type": "integer"},
                {"type": "boolean"},
                {"type": "object", "additionalProperties": this},
            ]}},
            {"type": "object", "additionalProperties": this},
        ]})
    }
}

/// `value` as every output writes it, whatever its field declares: text (a
/// string, an identifier, a date) as a string; a type union as its members
/// joined by `" | "`; an expression group as its conjuncts, each in its
/// display form, joined by `"\n"` (the string form the prove pass reads, one
/// comparison per line, ADR 0009); an integer and a boolean as JSON scalars;
/// a list or variant list as an array of its items' text; a block
/// as an object of its entries, each written by this function. Never the
/// parser's tree, never a position. The field text of the same value
/// (`specforge_registry::snapshot::field_text`) joins an expression group with `", "`.
pub fn write(value: &FieldValue) -> FieldJson {
    FieldJson(match value {
        FieldValue::String(s) | FieldValue::Identifier(s) | FieldValue::Date(s) => {
            Value::String(s.clone())
        }
        FieldValue::TypeUnion(types) => Value::String(
            types
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
        ),
        FieldValue::Expression(conjuncts) => Value::String(
            conjuncts
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        FieldValue::Integer(n) => Value::Number((*n).into()),
        FieldValue::Boolean(b) => Value::Bool(*b),
        FieldValue::List(items) => Value::Array(
            items
                .iter()
                .map(|item| Value::String(item.text.clone()))
                .collect(),
        ),
        FieldValue::VariantList(items) => Value::Array(
            items
                .iter()
                .map(|m| Value::String(m.text.clone()))
                .collect(),
        ),
        FieldValue::Blocks(items) => Value::Array(
            items
                .iter()
                .map(|item| {
                    Value::Object(
                        write_fields(&item.fields)
                            .into_iter()
                            .map(|(key, written)| (key, written.0))
                            .collect::<Map<_, _>>(),
                    )
                })
                .collect(),
        ),
        FieldValue::Block(inner) => Value::Object(
            write_fields(inner)
                .into_iter()
                .map(|(key, written)| (key, written.0))
                .collect::<Map<_, _>>(),
        ),
    })
}

/// Every field of `fields` written by [`write`], keyed by name: a key written twice is its first
/// entry, the parse rule (ADR 0215 D5).
pub(crate) fn write_fields(fields: &FieldMap) -> BTreeMap<String, FieldJson> {
    let mut out = BTreeMap::new();
    for entry in fields.entries() {
        out.entry(entry.key.to_string())
            .or_insert_with(|| write(&entry.value));
    }
    out
}

/// Set the value at `path` (`[0].status`, `.phase`: below the field's own name) inside `into`, when the item it
/// names is there (ADR 0287).
pub fn set_path(into: &mut Value, path: &str, value: &Value) {
    let (item, key, rest) = match path.strip_prefix('[') {
        Some(tail) => match tail.split_once(']') {
            Some((index, rest)) => (Some(index.parse().unwrap_or(usize::MAX)), "", rest),
            None => return,
        },
        None => {
            let tail = path.strip_prefix('.').unwrap_or(path);
            let end = tail.find(['.', '[']).unwrap_or(tail.len());
            (None, &tail[..end], &tail[end..])
        }
    };
    let next = match (item, &mut *into) {
        (Some(i), Value::Array(items)) => items.get_mut(i),
        (None, Value::Object(entries)) if rest.is_empty() => {
            entries.insert(key.to_string(), value.clone());
            return;
        }
        (None, Value::Object(entries)) => entries.get_mut(key),
        _ => None,
    };
    if let Some(next) = next {
        if rest.is_empty() {
            *next = value.clone();
        } else {
            set_path(next, rest, value);
        }
    }
}
