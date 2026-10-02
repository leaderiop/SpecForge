//! One extension vocabulary: what an SDK-built extension declares is what
//! the host's registry build reads. Every field type the SDK can name
//! registers (no W019), and the rules `specforge new --extension` and the
//! greet fixture write parse (no W112) and fire.

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use specforge_common::{SourceSpan, Sym};
use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta, prelude::*};
use specforge_registry::build_registries;
use specforge_registry::validation_engine::{ValidationEntity, execute_pattern};
use specforge_wasm::protocol::{
    DescribeResponse, ExtensionDescriptions, HandshakeResponse, ProtocolExtension,
    protocol_extension_to_manifest,
};

fn extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new("@you/vocab", "0.1.0"));
    c.kind("thing", |k| {
        for t in FieldType::ALL {
            k.field(&format!("f_{t}"), |f| {
                f.field_type(*t);
                if *t == FieldType::Enum {
                    f.enum_values(&["warm", "formal"]);
                }
            });
        }
        k.field("description", |f| {
            f.field_type(FieldType::String);
        });
        k.field("style", |f| {
            f.field_type(FieldType::String);
        });
    });
    // The rule `specforge new --extension` scaffolds.
    c.rule("W900", |r| {
        r.check(CheckKind::MissingRequiredField);
        r.target_kind("thing");
        r.field("description");
        r.severity(ValidationSeverity::Warning);
        r.message_template("thing '{id}' is missing a description");
    });
    // The greet fixture's rule.
    c.rule("G101", |r| {
        r.check(CheckKind::FieldValueConstraint);
        r.target_kind("thing");
        r.field("style");
        r.constraint(|fc| {
            fc.kind("matches");
            fc.pattern("^(warm|formal)$");
        });
        r.severity(ValidationSeverity::Error);
        r.message_template("thing '{id}' has unknown style");
    });
    c
}

fn describe<T: DeserializeOwned>(c: &ContributionsBuilder, category: &str) -> Vec<T> {
    let body = c
        .describe_response_json(category)
        .expect("supported category");
    serde_json::from_str::<DescribeResponse>(&body)
        .unwrap()
        .parse_items()
        .unwrap()
}

fn loaded(c: &ContributionsBuilder) -> ProtocolExtension {
    let handshake: HandshakeResponse = serde_json::from_str(&c.handshake_json()).unwrap();
    ProtocolExtension {
        name: handshake.name.clone(),
        version: handshake.version.clone(),
        handshake,
        descriptions: ExtensionDescriptions {
            entity_kinds: describe(c, "entities"),
            validation_rules: describe(c, "validation_rules"),
            ..Default::default()
        },
    }
}

fn entity(id: &str, fields: &[(&str, &str)]) -> ValidationEntity {
    ValidationEntity {
        id: id.to_string(),
        kind: "thing".to_string(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        incoming_edge_count: 0,
        outgoing_edge_count: 0,
        span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 0,
            end_line: 1,
            end_col: 0,
        },
        verify_kinds: Vec::new(),
        verify_texts: Vec::new(),
        outgoing_kinds: Default::default(),
        incoming_kinds: Default::default(),
        obligation_exempt: false,
    }
}

#[test]
fn sdk_vocabulary_round_trips_through_the_registry_build() {
    let build = build_registries(vec![protocol_extension_to_manifest(&loaded(&extension()))]);

    let unread: Vec<_> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W019" || d.code == "W112")
        .collect();
    assert!(unread.is_empty(), "host could not read: {unread:?}");
    for t in FieldType::ALL {
        assert!(
            build.fields.contains("thing", &format!("f_{t}")),
            "field type {t} did not register"
        );
    }

    let rule = |code: &str| {
        &build
            .rules
            .iter()
            .find(|(p, _)| p.code == code)
            .unwrap_or_else(|| panic!("rule {code} not registered"))
            .0
    };
    let bare = [entity("bare", &[("style", "loud")])];
    let fine = [entity(
        "fine",
        &[("description", "a thing"), ("style", "warm")],
    )];

    let fired = execute_pattern(rule("W900"), &bare, None);
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert_eq!(fired[0].message, "thing 'bare' is missing a description");
    let fired = execute_pattern(rule("G101"), &bare, None);
    assert_eq!(fired.len(), 1, "{fired:?}");
    assert_eq!(fired[0].code, "G101");
    assert!(execute_pattern(rule("W900"), &fine, None).is_empty());
    assert!(execute_pattern(rule("G101"), &fine, None).is_empty());
}

/// Rules written by SDK releases before the vocabulary was shared still
/// load: the host reads their check names as aliases.
#[test]
fn older_sdk_check_names_still_load() {
    let mut ext = loaded(&extension());
    for (rule, old) in ext
        .descriptions
        .validation_rules
        .iter_mut()
        .zip(["missing_field", "field_constraint"])
    {
        rule.check = old.to_string();
    }
    ext.descriptions.entity_kinds[0].fields[0].field_type = "boolean".to_string();

    let build = build_registries(vec![protocol_extension_to_manifest(&ext)]);
    let unread: Vec<_> = build
        .registry_diagnostics
        .iter()
        .filter(|d| d.code == "W019" || d.code == "W112")
        .collect();
    assert!(unread.is_empty(), "host could not read: {unread:?}");
    assert!(build.rules.iter().any(|(p, _)| p.code == "W900"));
    assert!(build.rules.iter().any(|(p, _)| p.code == "G101"));
}
