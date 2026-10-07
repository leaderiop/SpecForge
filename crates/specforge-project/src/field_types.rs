//! Field values against the types extensions declare for them.
//!
//! The grammar parses values generically; the FieldRegistry says what a
//! field's value must be. [`field_coercions`] tells the graph build how to
//! coerce values (a single string on a `string_list` field becomes a
//! one-item list), and [`check_field_value_types`] reports, as E061, the
//! values that still can't be the declared type.

use specforge_common::{Diagnostic, codes, find_close_match};
use specforge_graph::{DerivedFrom, DerivedReference, FieldCoercion, Graph};
use specforge_parser::FieldValue;
use specforge_registry::{FieldRegistry, FieldRegistryEntry, FieldType, KindRegistry};
use std::collections::HashMap;

/// How each registered field's value is coerced, keyed by (kind, field),
/// for [`specforge_graph::GraphConfig::field_coercions`].
pub fn field_coercions(field_reg: &FieldRegistry) -> HashMap<(String, String), FieldCoercion> {
    field_reg
        .iter()
        .filter_map(|(kind, field, entry)| {
            let coercion = match entry.field_type() {
                t if t.is_list() => FieldCoercion::List,
                FieldType::Integer => FieldCoercion::Integer,
                FieldType::Bool => FieldCoercion::Bool,
                FieldType::String | FieldType::Enum => FieldCoercion::Text,
                // A single reference or a block is read as written.
                _ => return None,
            };
            Some(((kind.to_string(), field.to_string()), coercion))
        })
        .collect()
}

/// The registered reference fields whose edges the host derives
/// (`derived_from`), for [`specforge_graph::GraphConfig::derived_references`].
/// A field that names an unknown source or no target kind derives nothing
/// (the manifest check reports it as W021). Sorted, so the build is
/// deterministic.
pub fn derived_references(field_reg: &FieldRegistry) -> Vec<DerivedReference> {
    let mut derived: Vec<DerivedReference> = field_reg
        .iter()
        .filter(|(_, _, entry)| entry.field_type().is_reference())
        .filter_map(|(kind, field, entry)| {
            Some(DerivedReference {
                source_kind: kind.to_string(),
                field: field.to_string(),
                target_kind: entry.declared().target_kind.clone()?,
                from: DerivedFrom::parse(entry.declared().derived_from.as_deref()?)?,
            })
        })
        .collect();
    derived.sort_by(|a, b| (&a.source_kind, &a.field).cmp(&(&b.source_kind, &b.field)));
    derived
}

/// E061 for every registered field whose (coerced) value can't be its
/// declared type: not an integer, not true/false, not a declared enum
/// value, or a list where a single value is declared. Unregistered kinds
/// and fields are left to E024 / W020.
pub fn check_field_value_types(
    graph: &Graph,
    kind_reg: &KindRegistry,
    field_reg: &FieldRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for node in graph.nodes() {
        let kind = node.kind.raw.as_str();
        if !kind_reg.contains(kind) {
            continue;
        }
        for entry in node.fields.entries() {
            let Some(declared) = field_reg.get(kind, entry.key.as_str()) else {
                continue;
            };
            let Some(mismatch) = mismatch(declared, &entry.value) else {
                continue;
            };
            let mut diagnostic = Diagnostic::new(
                codes::E061,
                format!(
                    "field '{}' of {} '{}' is declared {}, but was given {}",
                    entry.key,
                    kind,
                    node.id.raw,
                    declared.type_label(),
                    mismatch.given
                ),
            )
            .with_span(
                entry
                    .value_span
                    .clone()
                    .unwrap_or_else(|| node.source_span.clone()),
            );
            diagnostic.suggestion = mismatch.suggestion;
            diagnostics.push(diagnostic);
        }
    }
    diagnostics
}

struct Mismatch {
    given: String,
    suggestion: Option<String>,
}

fn mismatch(declared: &FieldRegistryEntry, value: &FieldValue) -> Option<Mismatch> {
    let wrong = |suggestion: Option<String>| {
        Some(Mismatch {
            given: describe(value),
            suggestion,
        })
    };
    let is_list = matches!(
        value,
        FieldValue::StringList(_)
            | FieldValue::ReferenceList(_)
            | FieldValue::MixedList(_)
            | FieldValue::VariantList(_)
    );
    match declared.field_type() {
        FieldType::Integer => match value {
            FieldValue::Integer(_) => None,
            _ => wrong(None),
        },
        FieldType::Bool => match value {
            FieldValue::Boolean(_) => None,
            _ => wrong(Some("use true or false".to_string())),
        },
        FieldType::Enum if !declared.enum_values().is_empty() => {
            let values = declared.enum_values();
            match value {
                FieldValue::String(s) | FieldValue::Identifier(s) if values.contains(s) => None,
                FieldValue::String(s) | FieldValue::Identifier(s) => wrong(Some(
                    match find_close_match(s, values.iter().map(String::as_str)) {
                        Some(close) => format!("did you mean '{close}'?"),
                        None => format!("use one of: {}", values.join(", ")),
                    },
                )),
                _ => wrong(Some(format!("use one of: {}", values.join(", ")))),
            }
        }
        FieldType::String | FieldType::Enum | FieldType::Reference if is_list => {
            wrong(Some("give a single value, not a list".to_string()))
        }
        _ => None,
    }
}

/// The value as the message quotes it.
fn describe(value: &FieldValue) -> String {
    match value {
        FieldValue::String(s) => format!("\"{s}\""),
        FieldValue::Identifier(s) | FieldValue::Date(s) => s.clone(),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::StringList(_)
        | FieldValue::ReferenceList(_)
        | FieldValue::MixedList(_)
        | FieldValue::VariantList(_) => "a list".to_string(),
        FieldValue::Block(_) => "a block".to_string(),
        FieldValue::VerifyList(_) => "verify statements".to_string(),
        FieldValue::Expression(_) => "an expression".to_string(),
        FieldValue::TypeUnion(types) => types.join(" | "),
    }
}
