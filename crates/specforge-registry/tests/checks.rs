//! The checks a built graph runs over its entities, reading the registries
//! the build made: unknown kinds (E024) with the extension that would
//! provide them, unknown fields (W020), and the bundled keyword and field
//! indexes those suggestions come from.

use specforge_common::{SourceSpan, Sym};
use specforge_registry::entity::EntityRecord;
use specforge_registry::{KindRegistry, detect_unknown_entity_fields};
use specforge_test_macros::test as spec;

use crate::support::{build, software, span};

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "unregistered keyword produces E024"
)]
fn detect_unknown_kinds_e024() {
    let build = build([software()]);
    let entities = vec![EntityRecord::new("unknown_thing", "u1", span("test.spec"))];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &build.kinds, None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E024");
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "E024 includes keyword name and source span"
)]
fn detect_unknown_kinds_e024_includes_info() {
    let build = build([software()]);
    let s = SourceSpan {
        file: Sym::new("my/file.spec"),
        start_line: 42,
        start_col: 0,
        end_line: 42,
        end_col: 10,
    };
    let entities = vec![EntityRecord::new("unknown_thing", "u1", &s)];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &build.kinds, None);
    assert!(diags[0].message.contains("unknown_thing"));
    assert!(diags[0].message.contains("my/file.spec"));
    assert_eq!(diags[0].span.as_ref().unwrap().start_line, 42);
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "registered keyword does not produce E024"
)]
fn detect_unknown_kinds_registered_no_e024() {
    let build = build([software()]);
    let entities = vec![EntityRecord::new("behavior", "b1", span("test.spec"))];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &build.kinds, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "define-block keywords not checked against KindRegistry"
)]
fn detect_unknown_kinds_define_not_checked() {
    let kind_reg = KindRegistry::new();
    let entities = vec![EntityRecord::new("define", "my_define", span("test.spec"))];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "Detect Unknown Entity Kinds: unknown entity kind detection holds — registries_populated_fired, structural_parse_ready, unknown_kinds_diagnosed, registered_kinds_accepted"
)]
fn detect_unknown_kinds_contract() {
    let build = build([software()]);
    let unknown = vec![EntityRecord::new("xyzzy", "x1", span("t.spec"))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&unknown, &build.kinds, None);
    assert!(d1.iter().any(|d| d.code == "E024"));
    let known = vec![EntityRecord::new("behavior", "b1", span("t.spec"))];
    let d2 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&known, &build.kinds, None);
    assert!(d2.is_empty());
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "E024 for keyword in index suggests the providing extension"
)]
fn suggest_missing_ext_known_keyword() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let entities = vec![EntityRecord::new("behavior", "b1", span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &entities,
        &kind_reg,
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge add @specforge/software")
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "E024 for keyword not in index suggests specforge search"
)]
fn suggest_missing_ext_unknown_keyword() {
    let kind_reg = KindRegistry::new();
    let index = specforge_registry::compilation::KeywordExtensionIndex::new();
    let entities = vec![EntityRecord::new("xyzzy", "x1", span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &entities,
        &kind_reg,
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge search")
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "keyword-to-extension index is loaded from bundled data file"
)]
fn bundled_keyword_index_maps_every_builtin_keyword() {
    // The bundled file must say what the builtins' own declarations say
    // (their pinned wire answers).
    let declarations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations");
    let mut expected = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&declarations).unwrap() {
        let src = entry.unwrap().path();
        let Ok(entities) = std::fs::read_to_string(src.join("describe_entities.json")) else {
            continue;
        };
        let handshake: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(src.join("handshake.json")).unwrap())
                .unwrap();
        let name = handshake["name"].as_str().unwrap().to_string();
        // The SDK greet fixture is pinned beside the builtins.
        if !name.starts_with("@specforge/") {
            continue;
        }
        let entities: serde_json::Value = serde_json::from_str(&entities).unwrap();
        for item in entities["items"].as_array().unwrap() {
            expected.insert(item["keyword"].as_str().unwrap().to_string(), name.clone());
        }
    }
    assert!(!expected.is_empty());

    let bundled = specforge_registry::compilation::KeywordExtensionIndex::bundled();
    for (keyword, extension) in &expected {
        assert_eq!(
            bundled.lookup(keyword),
            Some(extension.as_str()),
            "{keyword}"
        );
    }
    assert_eq!(bundled.lookup("xyzzy"), None);

    // Without an index argument, E024 uses the bundled one.
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &[EntityRecord::new("feature", "f1", span("test.spec"))],
        &KindRegistry::new(),
        None,
    );
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge add @specforge/product"),
        "{diags:?}"
    );
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "an undeclared field a builtin enhancement adds suggests its extension"
)]
fn bundled_field_index_maps_every_builtin_enhancement_field() {
    // The bundled file must say what the builtins' enhancements say (their
    // pinned wire answers).
    let declarations = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-component/tests/declarations");
    let mut expected = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&declarations).unwrap() {
        let src = entry.unwrap().path();
        let Ok(enhancements) = std::fs::read_to_string(src.join("describe_enhancements.json"))
        else {
            continue;
        };
        let handshake: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(src.join("handshake.json")).unwrap())
                .unwrap();
        let name = handshake["name"].as_str().unwrap().to_string();
        // The SDK greet fixture is pinned beside the builtins.
        if !name.starts_with("@specforge/") {
            continue;
        }
        let enhancements: serde_json::Value = serde_json::from_str(&enhancements).unwrap();
        for item in enhancements["items"].as_array().unwrap() {
            let kind = item["target_kind"].as_str().unwrap();
            for field in item["fields"].as_array().into_iter().flatten() {
                let field = field["name"].as_str().unwrap();
                expected.insert(format!("{kind}.{field}"), name.clone());
            }
        }
    }
    assert!(expected.contains_key("invariant.expression"));
    let bundled_json: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../data/field-index.json")).unwrap();
    assert_eq!(bundled_json, expected);

    // W020 for such a field names the extension, as E024 does for a kind.
    let build = build([software()]);
    let diags = detect_unknown_entity_fields(
        &[EntityRecord::new("invariant", "i1", span("test.spec"))
            .with_fields(&["expression", "bogus"])],
        &build.kinds,
        &build.fields,
    );
    assert_eq!(diags.len(), 2, "{diags:?}");
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge add @specforge/formal"),
        "{diags:?}"
    );
    assert_eq!(diags[1].suggestion, None, "{diags:?}");
}

#[test]
fn malformed_keyword_index_falls_back_to_search() {
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_json("{not json");
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(
        &[EntityRecord::new("feature", "f1", span("test.spec"))],
        &KindRegistry::new(),
        Some(&index),
    );
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .unwrap()
            .contains("specforge search feature"),
        "{diags:?}"
    );
}

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "Suggest Missing Extensions: missing extension suggestions holds — e024_diagnostic_emitted, suggestion_provided, lazy_loading_enforced"
)]
fn suggest_missing_ext_contract() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let e1 = vec![EntityRecord::new("behavior", "b1", span("test.spec"))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
    assert!(
        d1[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("@specforge/software")
    );
    let e2 = vec![EntityRecord::new("xyzzy", "x1", span("test.spec"))];
    let d2 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e2, &kind_reg, Some(&index));
    assert!(
        d2[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("specforge search")
    );
}

// -- detect_unknown_entity_fields over the build's registries (moved from
// src/compilation/tests/zero_entity_validation.rs with the rule engine's
// tests) --

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "unregistered field name produces W020"
)]
fn unregistered_field_name_produces_w020() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    let entities = vec![
        EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["unknown_field"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == "W020" && d.message.contains("unknown_field"))
    );
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "W020 includes field name, entity kind, and source span"
)]
fn w020_includes_field_name_entity_kind_and_source_span() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    let s = SourceSpan {
        file: Sym::new("my.spec"),
        start_line: 5,
        start_col: 3,
        end_line: 5,
        end_col: 20,
    };
    let entities = vec![EntityRecord::new("behavior", "b1", &s).with_fields(&["bogus_field"])];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    let w020: Vec<_> = diags.iter().filter(|d| d.code == "W020").collect();
    assert_eq!(w020.len(), 1);
    assert!(
        w020[0].message.contains("bogus_field"),
        "should contain field name"
    );
    assert!(
        w020[0].message.contains("behavior"),
        "should contain entity kind"
    );
    assert!(w020[0].span.is_some(), "should contain source span");
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "expression is checked like any other field (W020 where undeclared)"
)]
fn expression_is_checked_like_any_other_field() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    // software's invariant and behavior declare no `expression`; without an
    // extension that declares it (formal enhances invariant), it is W020.
    let entities = vec![
        EntityRecord::new("invariant", "i1", span("test.spec")).with_fields(&["expression"]),
        EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["expression"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    let flagged: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(diags.len(), 2, "{flagged:?}");
    assert!(
        diags
            .iter()
            .all(|d| d.code == "W020" && d.message.contains("'expression'")),
        "{flagged:?}"
    );

    // Declared on a kind, it is a field of that kind like any other.
    let mut field_reg = field_reg;
    field_reg.register(
        specforge_registry::FieldRegistryEntry::new(
            "invariant",
            "@specforge/formal",
            specforge_protocol_types::FieldDescriptor {
                name: "expression".to_string(),
                field_type: "string".to_string(),
                normative: true,
                proof_role: Some("claim".to_string()),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].message.contains("behavior"), "{diags:?}");
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "registered field name does not produce W020"
)]
fn registered_field_name_does_not_produce_w020() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    let entities =
        vec![EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["contract"])];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(diags.is_empty(), "registered field should not produce W020");
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "structural fields (title, verify) not checked against FieldRegistry"
)]
fn structural_fields_not_checked_against_field_registry() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    let entities = vec![
        EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["title", "verify"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(diags.is_empty(), "structural fields should be skipped");
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "verify on a kind no extension made testable produces W020"
)]
fn verify_on_non_testable_kind_produces_w020() {
    let specforge_registry::RegistryBuild {
        kinds: mut kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    kind_reg.get_mut("behavior").unwrap().supports_verify = false;
    let entities = vec![
        EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["title", "verify"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert_eq!(diags.len(), 1, "only verify is flagged: {diags:?}");
    assert_eq!(diags[0].code, "W020");
    assert!(diags[0].message.contains("'verify'"));
    assert!(
        diags[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("@specforge/testing"))
    );
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "field validation skipped when entity kind is unregistered"
)]
fn field_validation_skipped_when_entity_kind_is_unregistered() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    let entities = vec![
        EntityRecord::new("nonexistent_kind", "x1", span("test.spec")).with_fields(&["some_field"]),
    ];
    let diags = specforge_registry::compilation::detect_unknown_entity_fields(
        &entities, &kind_reg, &field_reg,
    );
    assert!(
        diags.is_empty(),
        "unregistered kind should skip field validation to avoid cascading diagnostics"
    );
}

#[spec(
    behavior = "detect_unknown_entity_fields",
    verify = "Detect Unknown Entity Fields: unknown field detection holds — registries_populated_fired, unknown_fields_diagnosed, cascading_avoided"
)]
fn detect_unknown_entity_fields_contract() {
    let specforge_registry::RegistryBuild {
        kinds: kind_reg,
        fields: field_reg,
        ..
    } = build([software()]);
    // ensures: unknown field → W020
    let e1 = vec![
        EntityRecord::new("behavior", "b1", span("test.spec")).with_fields(&["unknown_field"]),
    ];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e1, &kind_reg, &field_reg)
            .iter()
            .any(|d| d.code == "W020")
    );
    // ensures: registered field → no W020
    let e2 =
        vec![EntityRecord::new("behavior", "b2", span("test.spec")).with_fields(&["contract"])];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e2, &kind_reg, &field_reg)
            .is_empty()
    );
    // ensures: unregistered kind → skipped
    let e3 =
        vec![EntityRecord::new("unknown_kind", "x", span("test.spec")).with_fields(&["field"])];
    assert!(
        specforge_registry::compilation::detect_unknown_entity_fields(&e3, &kind_reg, &field_reg)
            .is_empty()
    );
}
