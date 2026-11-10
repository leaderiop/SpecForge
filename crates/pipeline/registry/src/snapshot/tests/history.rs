//! A reference the harness wrote once is history (ADR 0352 D2): the snapshot marks an unheld reference of a field
//! declared `writer: harness` and `immutable`, and no other.

use specforge_protocol_types::{EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor};

use crate::compilation::{RegistryBuild, build_registries};
use crate::snapshot::EntitySnapshot;

/// A reference list to `item`, the collected kind of the `js` scheme, written by `writer` and `immutable` as given.
fn cites(name: &str, writer: Option<&str>, immutable: bool) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        field_type: "reference_list".into(),
        target_kind: Some("item".into()),
        writer: writer.map(Into::into),
        immutable,
        ..Default::default()
    }
}

/// `record` with four reference lists to `item`: harness and immutable, harness only, immutable only, and authored.
fn registries() -> RegistryBuild {
    let mut declaration = ExtensionDeclaration {
        entities: vec![
            EntityKindDescriptor {
                name: "item".into(),
                writer: Some("collector".into()),
                schemes: vec!["js".into()],
                ..Default::default()
            },
            EntityKindDescriptor {
                name: "record".into(),
                keyword: Some("record".into()),
                fields: vec![
                    cites("once", Some("harness"), true),
                    cites("rewritten", Some("harness"), false),
                    cites("kept", None, true),
                    cites("authored", None, false),
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    declaration.handshake.name = "@t/history".into();
    declaration.handshake.version = "1.0.0".into();
    let build = build_registries(vec![declaration]);
    assert!(
        build.diagnostics().next().is_none(),
        "{:?}",
        build.diagnostics().collect::<Vec<_>>()
    );
    build
}

#[specforge_test_macros::test(
    behavior = "snapshot_entities_once",
    verify = "an unheld reference of a field the harness writes once is history"
)]
fn a_harness_immutable_reference_is_history() {
    let registries = registries();
    let source = "record r \"R\" {\n  once [\"js:a.js::f\"]\n  rewritten [\"js:a.js::g\"]\n  \
                  kept [\"js:a.js::h\"]\n  authored [\"js:a.js::i\"]\n}\n";
    let snapshot = EntitySnapshot::of(&super::graph_of(source, &registries), &registries);
    let entity = snapshot
        .pass_entities(&[])
        .into_iter()
        .find(|e| e.id == "r")
        .expect("the record");
    let marked: Vec<(&str, bool)> = entity
        .unheld
        .iter()
        .map(|u| (u.field.as_str(), u.history))
        .collect();
    assert_eq!(
        marked,
        [
            ("once", true),
            ("rewritten", false),
            ("kept", false),
            ("authored", false)
        ]
    );
    // On the wire only when set.
    let wire = serde_json::to_value(&entity.unheld).unwrap();
    assert_eq!(wire[0]["history"], true);
    assert!(wire[1].get("history").is_none(), "{wire}");
}
