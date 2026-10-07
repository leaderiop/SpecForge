//! Coercing parsed field values to the types extensions declare.
//!
//! The grammar parses every value generically (`"12"` is a string, `x a`
//! an identifier); the field's declared type says what the value means.
//! Coercion runs on the graph before references resolve, so a coerced
//! reference list links like a written one and exports carry the declared
//! shape. It never diagnoses: values that can't be coerced are left as
//! parsed for the semantic check (E061) to report.

use crate::Graph;
use specforge_parser::{FieldValue, SpannedRef};
use std::collections::HashMap;

/// What a field's declared type asks of its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldCoercion {
    /// `string_list` / `reference_list`: a single value becomes the
    /// one-item list `field [value]` parses to.
    List,
    /// `integer`: a quoted integer becomes the integer.
    Integer,
    /// `bool`: a quoted or bare `true` / `false` becomes the boolean.
    Bool,
    /// `string` / `enum`: an integer or boolean becomes its text.
    Text,
}

impl Graph {
    /// Coerce every node's registered field values; `coercions` maps
    /// (entity kind, field name) to the declared type's coercion.
    /// Idempotent, so rebuilds may re-run it on already coerced nodes.
    pub(crate) fn coerce_field_values(
        &mut self,
        coercions: &HashMap<(String, String), FieldCoercion>,
    ) {
        if coercions.is_empty() {
            return;
        }
        for node in self.nodes_mut() {
            let kind = node.kind.raw.as_str().to_string();
            for entry in node.fields.entries_mut() {
                let Some(coercion) = coercions.get(&(kind.clone(), entry.key.as_str().to_string()))
                else {
                    continue;
                };
                if let Some(value) = coerce(&entry.value, *coercion, entry.value_span.as_ref()) {
                    entry.value = value;
                }
            }
        }
    }
}

/// The coerced value, or `None` when `value` stays as parsed.
fn coerce(
    value: &FieldValue,
    coercion: FieldCoercion,
    span: Option<&specforge_common::SourceSpan>,
) -> Option<FieldValue> {
    match (coercion, value) {
        (FieldCoercion::List, FieldValue::String(s)) => {
            Some(FieldValue::StringList(vec![s.clone()]))
        }
        (FieldCoercion::List, FieldValue::Identifier(id)) => {
            let span = span?.clone();
            Some(FieldValue::ReferenceList(vec![SpannedRef {
                id: id.clone(),
                span,
            }]))
        }
        (FieldCoercion::Integer, FieldValue::String(s)) => {
            s.trim().parse::<i64>().ok().map(FieldValue::Integer)
        }
        (FieldCoercion::Bool, FieldValue::String(s) | FieldValue::Identifier(s)) => {
            match s.as_str() {
                "true" => Some(FieldValue::Boolean(true)),
                "false" => Some(FieldValue::Boolean(false)),
                _ => None,
            }
        }
        (FieldCoercion::Text, FieldValue::Integer(n)) => Some(FieldValue::String(n.to_string())),
        (FieldCoercion::Text, FieldValue::Boolean(b)) => Some(FieldValue::String(b.to_string())),
        _ => None,
    }
}
