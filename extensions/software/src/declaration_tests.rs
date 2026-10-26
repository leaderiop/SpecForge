use specforge_extension_sdk::FieldDescriptor;
use specforge_test::prelude::*;

/// The declared field `name` of `kind`.
fn declared(kind: &str, name: &str) -> FieldDescriptor {
    crate::specforge_extension_build()
        .declaration()
        .entities
        .into_iter()
        .find(|k| k.keyword.as_deref() == Some(kind))
        .unwrap_or_else(|| panic!("{kind} is not declared"))
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{kind}.{name} is not declared"))
}

/// A tag is one word from an open set (ADR 0147); `risk` and `direction` are closed sets.
#[specforge_test(
    behavior = "se_register_field_definitions",
    verify = "tag fields are identifiers, and risk and direction are enums of their levels"
)]
fn software_declares_its_tags_and_levels() {
    for (kind, field) in [
        ("behavior", "category"),
        ("behavior", "status"),
        ("behavior", "severity"),
        ("behavior", "diagnostic"),
        ("event", "channel_type"),
        ("event", "category"),
    ] {
        assert_eq!(
            declared(kind, field).field_type,
            "identifier",
            "{kind}.{field}"
        );
    }
    let direction = declared("port", "direction");
    assert_eq!(direction.field_type, "enum");
    assert_eq!(
        direction.enum_values,
        ["inbound", "outbound", "bidirectional"]
    );
    assert_eq!(direction.default_value, None);
    for (kind, field) in [("port", "category"), ("event", "channel")] {
        assert_eq!(declared(kind, field).field_type, "string", "{kind}.{field}");
    }
}

/// `invariant` is governance's (ADR 0280); a behavior's `invariants` reach it through an optional peer.
#[specforge_test(
    behavior = "se_declare_manifest",
    verify = "software declares behavior, event, type and port, and governance as an optional peer"
)]
fn software_declares_no_invariant() {
    let declaration = crate::specforge_extension_build().declaration();
    let kinds: Vec<&str> = declaration
        .entities
        .iter()
        .filter_map(|k| k.keyword.as_deref())
        .collect();
    assert_eq!(kinds, ["behavior", "event", "type", "port"]);
    assert!(declaration
        .validation_rules
        .iter()
        .all(|r| r.code != "W003"));
    let peers: Vec<(&str, bool)> = declaration
        .peers()
        .iter()
        .map(|p| (p.name.as_str(), p.optional))
        .collect();
    assert_eq!(
        peers,
        [
            ("@specforge/product", true),
            ("@specforge/governance", true)
        ]
    );
}
