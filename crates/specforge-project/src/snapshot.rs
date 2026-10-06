//! The entity snapshot (ADR 0019): every entity of one built graph as every
//! check after the build reads it. It owns the field-text rule, the one
//! string a field value is to a declarative rule, a custom validator and a
//! compiler pass, and builds the records the registry checks and the rules
//! read.

use std::collections::{BTreeMap, HashMap};

use specforge_graph::{Graph, Node};
use specforge_parser::FieldValue;
use specforge_registry::{FieldRegistry, KindRegistry};

pub use specforge_registry::entity::{
    Direction, EdgeCounts, EdgeRecord, EntityRecord, Exemption, FieldRecord, MethodRecord,
    ObligationRecord, ParamRecord, RuleInput,
};

/// A field value's text: the one rule every reader after the graph build
/// shares (ADR 0019). Scalars as written; lists of strings or references
/// and mixed lists joined by `", "`; variant lists and type unions by
/// `" | "`; expressions by `", "` in their display form; verify statements'
/// texts by `"; "`; a block's keys by `", "`. Empty values are `""`.
///
/// A joined list cannot be split back when an item itself contains the
/// joiner (`["a, b", "c"]` is `"a, b, c"`); [`field_items`] keeps them.
pub fn field_text(value: &FieldValue) -> String {
    let joined = |joiner: &str| field_items(value).unwrap_or_default().join(joiner);
    // No `_` arm: a new variant does not compile until it has a text.
    match value {
        FieldValue::String(s) | FieldValue::Identifier(s) | FieldValue::Date(s) => s.clone(),
        FieldValue::Integer(n) => n.to_string(),
        FieldValue::Boolean(b) => b.to_string(),
        FieldValue::StringList(_)
        | FieldValue::ReferenceList(_)
        | FieldValue::MixedList(_)
        | FieldValue::Expression(_)
        | FieldValue::Block(_) => joined(", "),
        FieldValue::VariantList(_) | FieldValue::TypeUnion(_) => joined(" | "),
        FieldValue::VerifyList(_) => joined("; "),
    }
}

/// A list-shaped value's items, unjoined, each as its text: the list's
/// strings or ids, a variant list's or type union's members, a mixed
/// list's items' texts, each expression's display form, each verify
/// statement's text, a block's keys. `None` for a scalar. [`field_text`]
/// is these joined.
pub fn field_items(value: &FieldValue) -> Option<Vec<String>> {
    match value {
        FieldValue::String(_)
        | FieldValue::Identifier(_)
        | FieldValue::Date(_)
        | FieldValue::Integer(_)
        | FieldValue::Boolean(_) => None,
        FieldValue::StringList(items)
        | FieldValue::VariantList(items)
        | FieldValue::TypeUnion(items) => Some(items.clone()),
        FieldValue::ReferenceList(refs) => Some(refs.iter().map(|r| r.id.clone()).collect()),
        FieldValue::MixedList(items) => Some(items.iter().map(field_text).collect()),
        FieldValue::Expression(exprs) => Some(exprs.iter().map(ToString::to_string).collect()),
        FieldValue::VerifyList(statements) => {
            Some(statements.iter().map(|s| s.description.clone()).collect())
        }
        FieldValue::Block(block) => {
            Some(block.entries().iter().map(|e| e.key.to_string()).collect())
        }
    }
}

/// Every entity of `graph` as the records the checks read, in id order:
/// what it writes (as field text), its references, obligations, edge
/// counts by peer kind, methods, and what exempts it
/// ([`crate::coverage::exemption`], read with `kinds` and `fields`).
pub fn entity_records(
    graph: &Graph,
    kinds: &KindRegistry,
    fields: &FieldRegistry,
) -> Vec<EntityRecord> {
    let kind_of: HashMap<&str, &str> = graph
        .nodes()
        .into_iter()
        .map(|n| (n.id.raw.as_str(), n.kind.raw.as_str()))
        .collect();
    graph
        .nodes()
        .into_iter()
        .map(|node| record(graph, &kind_of, node, kinds, fields))
        .collect()
}

/// The graph's edges, in graph order.
pub fn edge_records(graph: &Graph) -> Vec<EdgeRecord> {
    graph
        .edges()
        .iter()
        .map(|e| EdgeRecord {
            source: e.source.as_str().to_string(),
            target: e.target.as_str().to_string(),
            label: e.label.as_str().to_string(),
        })
        .collect()
}

fn record(
    graph: &Graph,
    kind_of: &HashMap<&str, &str>,
    node: &Node,
    kinds: &KindRegistry,
    fields: &FieldRegistry,
) -> EntityRecord {
    let id = node.id.raw.as_str();
    // Edges in one direction, by the kind of the entity at the far end.
    let counts = |peers: Vec<&str>| {
        let mut by_peer_kind = BTreeMap::new();
        for kind in peers.iter().filter_map(|peer| kind_of.get(peer)) {
            *by_peer_kind.entry(kind.to_string()).or_insert(0) += 1;
        }
        EdgeCounts {
            total: peers.len(),
            by_peer_kind,
        }
    };
    let incoming = counts(
        graph
            .edges_to(id)
            .iter()
            .map(|e| e.source.as_str())
            .collect(),
    );
    let outgoing = counts(
        graph
            .edges_from(id)
            .iter()
            .map(|e| e.target.as_str())
            .collect(),
    );
    let entries = node.fields.entries();
    EntityRecord {
        id: id.to_string(),
        kind: node.kind.raw.to_string(),
        span: node.source_span.clone(),
        fields: entries
            .iter()
            .map(|entry| FieldRecord {
                key: entry.key.to_string(),
                text: field_text(&entry.value),
                annotations: entry
                    .annotations
                    .iter()
                    .map(|a| a.name.to_string())
                    .collect(),
                items: field_items(&entry.value),
            })
            .collect(),
        references: entries
            .iter()
            .filter_map(|entry| match &entry.value {
                FieldValue::ReferenceList(refs) => Some((
                    entry.key.to_string(),
                    refs.iter().map(|r| r.id.clone()).collect(),
                )),
                _ => None,
            })
            .collect(),
        obligations: specforge_graph::obligations(node)
            .iter()
            .map(|statement| ObligationRecord {
                kind: statement.kind.clone(),
                text: statement.description.clone(),
            })
            .collect(),
        incoming,
        outgoing,
        exemption: crate::coverage::exemption(node, kinds, fields),
        methods: node
            .methods
            .iter()
            .map(|m| MethodRecord {
                name: m.name.clone(),
                params: m
                    .params
                    .iter()
                    .map(|p| ParamRecord {
                        name: p.name.clone(),
                        ty: p.ty.clone(),
                    })
                    .collect(),
                returns: m.returns.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    /// Every field of `source`'s first entity: key, variant name, text.
    fn texts(source: &str) -> Vec<(String, &'static str, String)> {
        let parsed = specforge_parser::parse(source, "t.spec");
        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
        parsed.entities[0]
            .fields
            .entries()
            .iter()
            .map(|e| {
                let variant = match &e.value {
                    FieldValue::String(_) => "String",
                    FieldValue::Identifier(_) => "Identifier",
                    FieldValue::Date(_) => "Date",
                    FieldValue::Integer(_) => "Integer",
                    FieldValue::Boolean(_) => "Boolean",
                    FieldValue::StringList(_) => "StringList",
                    FieldValue::ReferenceList(_) => "ReferenceList",
                    FieldValue::VariantList(_) => "VariantList",
                    FieldValue::TypeUnion(_) => "TypeUnion",
                    FieldValue::MixedList(_) => "MixedList",
                    FieldValue::Expression(_) => "Expression",
                    FieldValue::VerifyList(_) => "VerifyList",
                    FieldValue::Block(_) => "Block",
                };
                // The text is the items joined, for every list-shaped value.
                if let Some(items) = field_items(&e.value) {
                    let joiner = match variant {
                        "VariantList" | "TypeUnion" => " | ",
                        "VerifyList" => "; ",
                        _ => ", ",
                    };
                    assert_eq!(items.join(joiner), field_text(&e.value), "{}", e.key);
                }
                (e.key.to_string(), variant, field_text(&e.value))
            })
            .collect()
    }

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "a variant list or type union is its members joined by ' | ', a mixed list or expression group its items joined by ', '"
    )]
    fn field_text_of_every_variant() {
        let source = r#"item x "X" {
  title "a title"
  owner alice
  due 2026-10-06
  count 42
  active true
  labels ["a, b", "c"]
  needs [y, z]
  values [low, high]
  shape string | string[]
  mix [1, true, two]
  metric expr { latency < 10ms, load > 5 }
  ensures {
    done "it is done"
    kept "it is kept"
  }
  verify unit "x works"
  verify "x holds"
}
"#;
        let expected = [
            ("title", "String", "a title"),
            ("owner", "Identifier", "alice"),
            ("due", "Date", "2026-10-06"),
            ("count", "Integer", "42"),
            ("active", "Boolean", "true"),
            ("labels", "StringList", "a, b, c"),
            ("needs", "ReferenceList", "y, z"),
            ("values", "VariantList", "low | high"),
            ("shape", "TypeUnion", "string | string[]"),
            ("mix", "MixedList", "1, true, two"),
            ("metric", "Expression", "latency < 10ms, load > 5"),
            ("ensures", "Block", "done, kept"),
            ("verify", "VerifyList", "x works; x holds"),
        ];
        let actual = texts(source);
        assert_eq!(
            actual,
            expected
                .iter()
                .map(|(k, v, t)| (k.to_string(), *v, t.to_string()))
                .collect::<Vec<_>>()
        );

        // A list keeps its items unjoined: the joined text of `labels`
        // cannot tell "a, b" from "a" and "b"; its items can.
        let parsed = specforge_parser::parse(source, "t.spec");
        let items: Vec<(String, Option<Vec<String>>)> = parsed.entities[0]
            .fields
            .entries()
            .iter()
            .map(|e| (e.key.to_string(), field_items(&e.value)))
            .collect();
        let of = |key: &str| {
            items
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, items)| items.clone())
        };
        let owned = |list: &[&str]| Some(list.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(of("labels"), owned(&["a, b", "c"]));
        assert_eq!(of("values"), owned(&["low", "high"]));
        assert_eq!(of("shape"), owned(&["string", "string[]"]));
        assert_eq!(of("mix"), owned(&["1", "true", "two"]));
        assert_eq!(of("metric"), owned(&["latency < 10ms", "load > 5"]));
        assert_eq!(of("ensures"), owned(&["done", "kept"]));
        assert_eq!(of("verify"), owned(&["x works", "x holds"]));
        for scalar in ["title", "owner", "due", "count", "active"] {
            assert_eq!(of(scalar), None, "{scalar}");
        }
    }

    #[specforge_test(
        behavior = "snapshot_entities_once",
        verify = "an empty list or block is written, with empty text, never left out or null"
    )]
    fn an_empty_value_has_empty_text() {
        let actual = texts("item x \"X\" {\n  values []\n  tags []\n  requires {\n  }\n}\n");
        assert_eq!(
            actual,
            [
                ("values".to_string(), "VariantList", String::new()),
                ("tags".to_string(), "ReferenceList", String::new()),
                ("requires".to_string(), "Block", String::new()),
            ]
        );
    }
}
