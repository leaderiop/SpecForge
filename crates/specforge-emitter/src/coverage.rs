//! The host's coverage vocabulary: what an entity's obligations are.
//!
//! Every surface that asks "what does this entity promise to prove" (stats,
//! plan validation, the context exports, the MCP coverage, inspect, review
//! and trace views) reads it here, so they cannot disagree.

use serde_json::Value;
use specforge_graph::{FieldMap, FieldValue, Node};
use specforge_parser::VerifyStatement;

/// An entity's obligations: its `verify` statements, in declaration order.
///
/// They are found wherever they sit among the entity's fields. A type may
/// declare a struct member named `verify` (`verify string @optional`); that
/// member is a field, not an obligation, and must not hide the statements,
/// which a first-match lookup of the `verify` key would do.
pub fn obligations(node: &Node) -> &[VerifyStatement] {
    obligations_in(&node.fields)
}

/// [`obligations`] over a bare field map.
pub fn obligations_in(fields: &FieldMap) -> &[VerifyStatement] {
    fields
        .entries()
        .iter()
        .find_map(|entry| match &entry.value {
            FieldValue::VerifyList(stmts) => Some(stmts.as_slice()),
            _ => None,
        })
        .unwrap_or(&[])
}

/// The obligations as the exports write them (`[{kind, description}]`), or
/// `None` when the entity declares none.
pub(crate) fn obligations_json(node: &Node) -> Option<Value> {
    let stmts = obligations(node);
    (!stmts.is_empty()).then(|| {
        Value::Array(
            stmts
                .iter()
                .map(|s| serde_json::json!({"kind": s.kind, "description": s.description}))
                .collect(),
        )
    })
}
