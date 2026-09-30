//! Field values against the types extensions declare for them.
//!
//! The grammar parses values generically; the FieldRegistry says what a
//! field's value must be. [`field_coercions`] tells the graph build how to
//! coerce values (a single string on a `string_list` field becomes a
//! one-item list), and [`check_field_value_types`] reports, as E061, the
//! values that still can't be the declared type.

use specforge_common::{Diagnostic, Severity, find_close_match};
use specforge_graph::{FieldCoercion, Graph};
use specforge_parser::FieldValue;
use specforge_registry::{FieldRegistry, KindRegistry, ManifestFieldType};
use std::collections::HashMap;

/// How each registered field's value is coerced, keyed by (kind, field),
/// for [`specforge_graph::GraphConfig::field_coercions`].
pub fn field_coercions(field_reg: &FieldRegistry) -> HashMap<(String, String), FieldCoercion> {
    field_reg
        .iter()
        .filter_map(|(kind, field, entry)| {
            let coercion = match entry.field_type {
                ManifestFieldType::StringList | ManifestFieldType::ReferenceList => {
                    FieldCoercion::List
                }
                ManifestFieldType::Integer => FieldCoercion::Integer,
                ManifestFieldType::Bool => FieldCoercion::Bool,
                ManifestFieldType::String | ManifestFieldType::Enum(_) => FieldCoercion::Text,
                ManifestFieldType::Reference | ManifestFieldType::Block => return None,
            };
            Some(((kind.to_string(), field.to_string()), coercion))
        })
        .collect()
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
            let Some(mismatch) = mismatch(&declared.field_type, &entry.value) else {
                continue;
            };
            diagnostics.push(Diagnostic {
                code: "E061".to_string(),
                severity: Severity::Error,
                message: format!(
                    "field '{}' of {} '{}' is declared {}, but was given {}",
                    entry.key,
                    kind,
                    node.id.raw,
                    type_name(&declared.field_type),
                    mismatch.given
                ),
                span: Some(
                    entry
                        .value_span
                        .clone()
                        .unwrap_or_else(|| node.source_span.clone()),
                ),
                suggestion: mismatch.suggestion,
            });
        }
    }
    diagnostics
}

struct Mismatch {
    given: String,
    suggestion: Option<String>,
}

fn mismatch(declared: &ManifestFieldType, value: &FieldValue) -> Option<Mismatch> {
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
    match declared {
        ManifestFieldType::Integer => match value {
            FieldValue::Integer(_) => None,
            _ => wrong(None),
        },
        ManifestFieldType::Bool => match value {
            FieldValue::Boolean(_) => None,
            _ => wrong(Some("use true or false".to_string())),
        },
        ManifestFieldType::Enum(values) if !values.is_empty() => match value {
            FieldValue::String(s) | FieldValue::Identifier(s) if values.contains(s) => None,
            FieldValue::String(s) | FieldValue::Identifier(s) => wrong(Some(
                match find_close_match(s, values.iter().map(String::as_str)) {
                    Some(close) => format!("did you mean '{close}'?"),
                    None => format!("use one of: {}", values.join(", ")),
                },
            )),
            _ => wrong(Some(format!("use one of: {}", values.join(", ")))),
        },
        ManifestFieldType::String | ManifestFieldType::Enum(_) | ManifestFieldType::Reference
            if is_list =>
        {
            wrong(Some("give a single value, not a list".to_string()))
        }
        _ => None,
    }
}

fn type_name(field_type: &ManifestFieldType) -> String {
    match field_type {
        ManifestFieldType::String => "string".to_string(),
        ManifestFieldType::Integer => "integer".to_string(),
        ManifestFieldType::Bool => "bool".to_string(),
        ManifestFieldType::Enum(values) if values.is_empty() => "enum".to_string(),
        ManifestFieldType::Enum(values) => format!("enum ({})", values.join(", ")),
        ManifestFieldType::StringList => "string_list".to_string(),
        ManifestFieldType::Reference => "reference".to_string(),
        ManifestFieldType::ReferenceList => "reference_list".to_string(),
        ManifestFieldType::Block => "block".to_string(),
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
