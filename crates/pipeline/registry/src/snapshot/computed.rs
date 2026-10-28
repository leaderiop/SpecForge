//! The entity snapshot with the values compute passes gave (ADR 0287): the snapshot every rule, validator and check
//! pass reads. The graph keeps authored values only.

use serde_json::Value;
use specforge_graph::field_form::{declared_path, json::set_path};
use specforge_graph::{Computed, ValueShape};
use specforge_protocol_types::{FieldType, FieldWriter};

use super::EntitySnapshot;
use crate::FieldRegistry;
use crate::entity::{FieldRecord, Origin};

impl EntitySnapshot {
    /// This snapshot with `computed` merged in: a computed field replaces the authored record of the same field
    /// (which stays [`Origin::Overwritten`], at its written value's span, for E082) or is added; a computed member
    /// does the same inside its field's record. A value for a field or an item the entity does not hold is left out.
    pub fn with_computed(&self, computed: &Computed, fields: &FieldRegistry) -> EntitySnapshot {
        let mut merged = self.clone();
        for record in &mut merged.records {
            for (path, value) in computed.of(&record.id) {
                let declared = declared_path(path);
                let form = fields
                    .get_path(&record.kind, &declared)
                    .map(|(_, form)| form);
                let date = form.is_some_and(|f| f.field_type() == FieldType::Date);
                let origin = if form.is_some_and(|f| f.writer() == Some(FieldWriter::Verdict)) {
                    Origin::Verdict
                } else {
                    Origin::Computed
                };
                let value = value.as_value();
                match path.find(['.', '[']) {
                    None => put(&mut record.fields, path, &declared, value, (date, origin)),
                    Some(at) => {
                        let Some(top) = record.fields.iter_mut().find(|f| f.key == path[..at])
                        else {
                            continue;
                        };
                        put(&mut top.members, path, &declared, value, (date, origin));
                        if let Some(written) = &mut top.written {
                            set_path(written, &path[at..], value);
                        }
                    }
                }
            }
        }
        merged
    }
}

/// `value` as the record of `key` in `records`, replacing the one there.
fn put(
    records: &mut Vec<FieldRecord>,
    key: &str,
    declared: &str,
    value: &Value,
    (date, origin): (bool, Origin),
) {
    let mut record = of_json(key, declared, value, date);
    record.origin = origin;
    match records.iter_mut().find(|r| r.key == key) {
        Some(old) => {
            record.value_span = old.value_span.clone();
            record.origin = Origin::Overwritten;
            record.members = std::mem::take(&mut old.members);
            *old = record;
        }
        None => records.push(record),
    }
}

/// A computed value as a record: its text and items as the field text rule writes them.
fn of_json(key: &str, declared: &str, value: &Value, date: bool) -> FieldRecord {
    let (shape, text, items) = match value {
        Value::String(s) if date => (ValueShape::Date, s.clone(), None),
        Value::String(s) => (ValueShape::String, s.clone(), None),
        Value::Number(n) => (ValueShape::Integer, n.to_string(), None),
        Value::Bool(b) => (ValueShape::Boolean, b.to_string(), None),
        Value::Array(entries) if entries.iter().all(Value::is_object) && !entries.is_empty() => {
            let items: Vec<String> = entries.iter().map(keys).collect();
            (ValueShape::Blocks, items.join(", "), Some(items))
        }
        Value::Array(entries) => {
            let items: Vec<String> = entries
                .iter()
                .map(|e| e.as_str().map_or_else(|| e.to_string(), str::to_string))
                .collect();
            (ValueShape::List, items.join(", "), Some(items))
        }
        Value::Object(entries) => {
            let items: Vec<String> = entries.keys().cloned().collect();
            (ValueShape::Block, items.join(", "), Some(items))
        }
        Value::Null => (ValueShape::String, String::new(), None),
    };
    FieldRecord {
        key: key.to_string(),
        declared: declared.to_string(),
        members: Vec::new(),
        missing: Vec::new(),
        unknown: Vec::new(),
        written: matches!(value, Value::Array(_) | Value::Object(_)).then(|| value.clone()),
        text,
        annotations: Vec::new(),
        items,
        shape,
        value_span: None,
        rewrites: Vec::new(),
        origin: Origin::Computed,
    }
}

/// An object's keys in braces, the text of a block list item.
fn keys(item: &Value) -> String {
    let keys: Vec<&str> = item
        .as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default();
    format!("{{{}}}", keys.join(", "))
}
