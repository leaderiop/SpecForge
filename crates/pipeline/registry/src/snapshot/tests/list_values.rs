//! A list field reaches a pass and a validator as its items (ADR 0294), not only as the text joined by `", "`.

use std::path::Path;

use serde_json::{Value, json};
use specforge_protocol_types::{EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor};

use crate::compilation::build_registries;
use crate::snapshot::EntitySnapshot;

fn field(name: &str, field_type: &str) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        field_type: field_type.into(),
        target_kind: (field_type == "reference_list").then(|| "node".into()),
        enum_values: if field_type == "enum_list" {
            vec!["red".into(), "blue".into()]
        } else {
            vec![]
        },
        ..Default::default()
    }
}

/// The registries of a kind `node` with the list fields `tags`, `parts` and `colors`, and a variant list `shapes`.
fn registries() -> crate::compilation::RegistryBuild {
    let mut declaration = ExtensionDeclaration {
        entities: vec![EntityKindDescriptor {
            name: "node".into(),
            keyword: Some("node".into()),
            fields: vec![
                field("tags", "string_list"),
                field("parts", "reference_list"),
                field("colors", "enum_list"),
                field("shapes", "variants"),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    declaration.handshake.name = "@specforge/testing".into();
    declaration.handshake.version = "1.0.0".into();
    build_registries(vec![declaration])
}

/// The snapshot of `source` over [`registries`].
fn snapshot(source: &str) -> EntitySnapshot {
    let registries = registries();
    EntitySnapshot::of(
        &super::graph_of(source, &registries),
        &registries,
        Path::new(""),
        Path::new(""),
    )
}

/// `a`'s `values` as a pass is sent them, and as a validator is.
fn values_of(snapshot: &EntitySnapshot, id: &str) -> (Value, Value) {
    let pass = snapshot
        .pass_entities()
        .into_iter()
        .find(|e| e.id == id)
        .expect("a pass entity");
    let validator = snapshot.validator_context(id).expect("an entity").entity;
    (json!(pass.values), json!(validator.values))
}

#[test]
fn a_list_item_holding_the_joiner_reaches_a_pass_whole() {
    let snapshot = snapshot("node a \"A\" {\n  tags [\"ships, fast\", \"works\"]\n}\n");
    let (pass, _) = values_of(&snapshot, "a");
    assert_eq!(pass["tags"], json!(["ships, fast", "works"]));
    // The joined text stays for a 1.10 guest.
    let entity = &snapshot.pass_entities()[0];
    assert_eq!(entity.fields["tags"], "ships, fast, works");
}

#[test]
fn a_written_empty_list_is_an_empty_array() {
    let snapshot = snapshot("node a \"A\" {\n  tags []\n}\n");
    let (pass, _) = values_of(&snapshot, "a");
    assert_eq!(pass["tags"], json!([]));
    // A field not written has no key.
    assert!(pass.get("parts").is_none(), "{pass}");
}

#[test]
fn a_reference_list_and_an_enum_list_are_items_too() {
    let snapshot = snapshot(
        "node a \"A\" {\n  parts [b, c]\n  colors [red, blue]\n}\nnode b \"B\" {}\nnode c \"C\" {}\n",
    );
    let (pass, _) = values_of(&snapshot, "a");
    assert_eq!(pass["parts"], json!(["b", "c"]));
    assert_eq!(pass["colors"], json!(["red", "blue"]));
}

#[test]
fn a_validator_gets_the_same_items() {
    let snapshot = snapshot("node a \"A\" {\n  tags [\"x, y\", \"z\"]\n  colors [red]\n}\n");
    let (pass, validator) = values_of(&snapshot, "a");
    assert_eq!(pass, validator);
    assert_eq!(validator["tags"], json!(["x, y", "z"]));
}

#[test]
fn a_variant_list_is_still_text_only() {
    let snapshot = snapshot("node a \"A\" {\n  shapes a | b\n}\n");
    let (pass, _) = values_of(&snapshot, "a");
    assert!(pass.get("shapes").is_none(), "{pass}");
}

#[test]
fn a_computed_list_and_an_authored_one_agree() {
    let registries = registries();
    let authored = snapshot("node a \"A\" {\n  tags [\"x, y\", \"z\"]\n}\nnode b \"B\" {}\n");
    let (_, form) = registries
        .fields()
        .get_path("node", "tags")
        .expect("the field");
    let mut computed = specforge_graph::Computed::default();
    computed.insert(
        "b",
        "tags",
        form.admit_json(&json!(["x, y", "z"])).expect("admitted"),
    );
    let merged = authored.with_computed(&computed, registries.fields());
    let (a, _) = values_of(&merged, "a");
    let (b, _) = values_of(&merged, "b");
    assert_eq!(a["tags"], b["tags"]);
}
