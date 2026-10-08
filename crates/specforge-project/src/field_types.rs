//! The graph build's field inputs, from the types extensions declare.
//!
//! The grammar parses values generically; the FieldRegistry says what a
//! field's value must be. [`field_coercions`] tells the graph build how to
//! coerce values (a single string on a `string_list` field becomes a
//! one-item list); the values that still can't be the declared type are
//! `RegistryBuild::check`'s E061.

use specforge_graph::{DerivedFrom, DerivedReference, FieldCoercion};
use specforge_registry::{FieldRegistry, FieldType};
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
