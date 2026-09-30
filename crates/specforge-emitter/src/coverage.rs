//! The host's coverage vocabulary: which kinds are testable and what an
//! entity's obligations are.
//!
//! Every surface that asks "does this entity count toward coverage" or
//! "what does it promise to prove" (stats, plan validation, the context
//! exports, the MCP coverage, inspect, review and trace views) reads it
//! here, so they cannot disagree.

use serde_json::Value;
use specforge_graph::{FieldMap, FieldValue, Node};
use specforge_parser::VerifyStatement;
use specforge_registry::KindRegistry;
use std::collections::BTreeSet;

/// The kinds that count toward coverage: those an extension's manifest
/// declares `testable`. Nothing is testable by default, and accepting
/// `verify` statements (`supports_verify`) does not make a kind testable.
pub fn testable_kinds(reg: &KindRegistry) -> BTreeSet<&str> {
    reg.iter()
        .filter(|(_, kind)| kind.testable)
        .map(|(name, _)| name.as_str())
        .collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::KindRegistryEntry;
    use specforge_test_macros::test as specforge_test;

    fn kind(name: &str, testable: bool, supports_verify: bool) -> KindRegistryEntry {
        KindRegistryEntry {
            kind_name: name.into(),
            description: None,
            source_extension: "@test/ext".into(),
            testable,
            singleton: false,
            supports_verify,
            allowed_verify_kinds: Vec::new(),
            has_body_parser: false,
            semantic_token: None,
            lsp_icon: None,
            dot_shape: None,
            dot_color: None,
            dot_fillcolor: None,
            open_fields: false,
        }
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "no default testability assumed by core"
    )]
    fn no_kind_is_testable_unless_an_extension_says_so() {
        assert!(testable_kinds(&KindRegistry::new()).is_empty());

        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", false, false));
        assert!(testable_kinds(&reg).is_empty());
    }

    #[specforge_test(
        invariant = "testable_entity_classification",
        verify = "testable=false entity excluded from coverage"
    )]
    fn only_kinds_declared_testable_count() {
        let mut reg = KindRegistry::new();
        reg.register(kind("behavior", true, true));
        reg.register(kind("type", true, true));
        // Accepts verify statements but does not count toward coverage.
        reg.register(kind("property", false, true));
        reg.register(kind("feature", false, false));
        assert_eq!(
            testable_kinds(&reg).into_iter().collect::<Vec<_>>(),
            ["behavior", "type"]
        );
    }
}
