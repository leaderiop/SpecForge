//! What a field's checks look like in the schema (ADR 0336).

use serde_json::json;
use specforge_emitter::generate_schema;
use specforge_protocol_types::{
    CheckRef, CheckerDescriptor, EntityKindDescriptor, ExtensionDeclaration, FieldDescriptor,
};
use specforge_registry::build_registries;
use specforge_test::prelude::*;

const EXT: &str = "@pin/schema";

fn plain(name: &str, checks: Vec<CheckRef>) -> FieldDescriptor {
    FieldDescriptor {
        name: name.into(),
        field_type: "string".into(),
        checks,
        ..Default::default()
    }
}

fn checker(name: &str, args: &[&str]) -> CheckerDescriptor {
    CheckerDescriptor {
        name: name.into(),
        applies_to: vec!["string".into()],
        args: args.iter().map(|arg| plain(arg, Vec::new())).collect(),
        ..Default::default()
    }
}

#[specforge_test(
    behavior = "generate_schema_from_registries",
    verify = "a checked field lists its resolved checks, and a member its own"
)]
fn a_checked_field_lists_its_checks() {
    let mut declaration = ExtensionDeclaration::default();
    declaration.handshake.name = EXT.to_string();
    declaration.handshake.version = "1.0.0".to_string();
    declaration.checkers = vec![checker("url", &["status"]), checker("file", &[])];
    declaration.entities = vec![EntityKindDescriptor {
        name: "service".into(),
        fields: vec![
            plain(
                "home",
                vec![
                    CheckRef::with("url", json!({"status": "200"})),
                    CheckRef::AnyOf {
                        any_of: vec![CheckRef::named("url"), CheckRef::named("file")],
                    },
                ],
            ),
            plain("note", Vec::new()),
            FieldDescriptor {
                name: "links".into(),
                field_type: "block".into(),
                members: vec![plain("href", vec![CheckRef::named("file")])],
                ..Default::default()
            },
        ],
        ..Default::default()
    }];
    let schema = generate_schema(&build_registries(vec![declaration]));
    let field = |name: &str| {
        schema.entity_kinds[0]
            .fields
            .iter()
            .find(|f| f.name == name)
            .unwrap()
    };
    let written = serde_json::to_value(&field("home").checks).unwrap();
    assert_eq!(
        written,
        json!([
            {"checker": "@pin/schema:url", "args": {"status": "200"}},
            {"any_of": [{"checker": "@pin/schema:url"}, {"checker": "@pin/schema:file"}]},
        ])
    );
    assert_eq!(
        field("links").members[0].checks[0].checker.as_deref(),
        Some("@pin/schema:file")
    );
    assert!(field("note").checks.is_empty());
    assert!(
        !serde_json::to_string(field("note"))
            .unwrap()
            .contains("checks"),
        "an unchecked field's schema is as it was"
    );
}
