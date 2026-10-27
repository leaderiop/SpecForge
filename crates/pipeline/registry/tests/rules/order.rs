//! The rule set's order and ownership: the extensions' rules by code
//! (declaration order within a code), then the host's E006 rules by kind
//! and field; an extension's codes sit under its namespace.

use specforge_common::Severity;
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::CheckKind;
use specforge_registry::rules::Origin;

use super::{declaring_with_kinds, entity, rule, rules_of};
use crate::support::declare;

#[test]
fn rules_from_several_extensions_are_one_set_in_code_order() {
    let mut a = declaring_with_kinds(vec![
        rule("W300", "no_incoming_edges"),
        rule("W100", "no_incoming_edges"),
    ]);
    a.handshake.name = "@ext/a".to_string();
    let mut b = declaring_with_kinds(vec![rule("W200", "no_outgoing_edges")]);
    b.handshake.name = "@ext/b".to_string();

    let built = rules_of(vec![a, b]);

    assert!(built.diagnostics.is_empty(), "{:?}", built.diagnostics);
    let owned: Vec<(&str, &str)> = built
        .rules
        .iter()
        .map(|r| (r.code(), r.origin().name()))
        .collect();
    assert_eq!(
        owned,
        [
            ("ext.a/W100", "@ext/a"),
            ("ext.a/W300", "@ext/a"),
            ("ext.b/W200", "@ext/b")
        ]
    );
}

#[test]
fn a_code_declared_for_several_kinds_keeps_declaration_order() {
    let mut declared = Vec::new();
    for kind in ["type", "behavior", "event"] {
        let mut w004 = rule("W004", "no_verify_statements");
        w004.target_kind = Some(kind.to_string());
        declared.push(w004);
    }
    declared.insert(1, rule("E001", "no_edges"));
    let built = rules_of(vec![declaring_with_kinds(declared)]);
    let order: Vec<(&str, Option<&str>)> = built
        .rules
        .iter()
        .map(|r| (r.code(), r.target_kind()))
        .collect();
    assert_eq!(
        order,
        [
            ("test/E001", Some("behavior")),
            ("test/W004", Some("type")),
            ("test/W004", Some("behavior")),
            ("test/W004", Some("event")),
        ]
    );
}

#[test]
fn the_hosts_e006_rules_follow_the_extensions_by_kind_and_field() {
    let mut declaration = declare("@test", |c| {
        c.kind("behavior", |k| {
            k.keyword("behavior");
        });
        c.kind("zeta", |k| {
            k.description("z");
            k.field("b", |f| {
                f.field_type(FieldType::String).required();
            });
            k.field("a", |f| {
                f.field_type(FieldType::String).required();
            });
        });
        c.kind("alpha", |k| {
            k.description("a");
            k.field("name", |f| {
                f.field_type(FieldType::String).required();
            });
            k.field("note", |f| {
                f.field_type(FieldType::String);
            });
        });
    });
    declaration.validation_rules = vec![rule("Z999", "no_edges")];

    let built = rules_of(vec![declaration]);

    let order: Vec<(&str, Option<&str>, &Origin)> = built
        .rules
        .iter()
        .map(|r| (r.code(), r.target_kind(), r.origin()))
        .collect();
    let ext = Origin::Extension("@test".to_string());
    assert_eq!(
        order,
        [
            ("test/Z999", Some("behavior"), &ext),
            ("E006", Some("alpha"), &Origin::Host),
            ("E006", Some("zeta"), &Origin::Host),
            ("E006", Some("zeta"), &Origin::Host),
        ]
    );
    let e006 = built.rules.iter().nth(1).unwrap();
    assert_eq!(e006.check_kind(), CheckKind::MissingRequiredField);
    assert_eq!(e006.severity(), Severity::Error);
    assert_eq!(e006.describe()["field"], "name");
    let fields: Vec<serde_json::Value> = built
        .rules
        .iter()
        .skip(2)
        .map(|r| r.describe()["field"].clone())
        .collect();
    assert_eq!(fields, [serde_json::json!("a"), serde_json::json!("b")]);

    let diagnostics = super::check(
        &built,
        &[
            entity("a1", "alpha", 0, 0),
            entity("a2", "alpha", 0, 0).with_field("name", ""),
        ],
    );
    let messages: Vec<&str> = diagnostics
        .iter()
        .filter(|d| d.code == "E006")
        .map(|d| d.message.as_str())
        .collect();
    // Written, even empty, is present.
    assert_eq!(messages, ["alpha 'a1' is missing required field 'name'"]);
}
