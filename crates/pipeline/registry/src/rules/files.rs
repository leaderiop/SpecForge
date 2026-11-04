//! The paths an entity's field names (ADR 0293): what a pass's or command's `field` read hands to the host.

use specforge_common::SourceSpan;
use specforge_graph::ValueShape;

use crate::entity::{EntityRecord, FieldRecord};

/// The paths `record` names in `field`: its last occurrence's text when it is a scalar, or each
/// item when it is a list, never an empty one. A block, an expression group, a variant list and a
/// type union name none; a joined text is never split (ADR 0019). A member path names those of
/// each of its members.
fn named<'r>(record: &'r EntityRecord, field: &str) -> Vec<(&'r str, Option<SourceSpan>)> {
    let member = field.contains('.');
    let written: Vec<&FieldRecord> = if member {
        record
            .fields
            .iter()
            .flat_map(|f| &f.members)
            .filter(|m| m.declared == field)
            .collect()
    } else {
        record
            .fields
            .iter()
            .rev()
            .find(|f| f.key == field)
            .into_iter()
            .collect()
    };
    written
        .into_iter()
        .flat_map(|written| -> Vec<(&'r str, Option<SourceSpan>)> {
            let span = if member {
                written.value_span.clone()
            } else {
                None
            };
            let paths: Vec<&str> = match written.shape {
                ValueShape::String
                | ValueShape::Identifier
                | ValueShape::Date
                | ValueShape::Integer
                | ValueShape::Boolean => vec![written.text.as_str()],
                ValueShape::List => written.items.iter().flatten().map(String::as_str).collect(),
                ValueShape::Block
                | ValueShape::Blocks
                | ValueShape::Expression
                | ValueShape::Variants
                | ValueShape::TypeUnion => Vec::new(),
            };
            paths.into_iter().map(|p| (p, span.clone())).collect()
        })
        .filter(|(name, _)| !name.is_empty())
        .collect()
}

/// The paths `record` names in `field` (a declared path), as [`named`] reads them: what a pass's or command's `field` read hands
/// to the host (ADR 0293).
pub fn paths_named<'r>(record: &'r EntityRecord, field: &str) -> Vec<&'r str> {
    named(record, field)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}
