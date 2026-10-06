//! The checks a built graph runs over its entities, reading the registries
//! the build made: unknown kinds (E024) with the extension that would
//! provide them, unknown fields (W020), and the bundled keyword and field
//! indexes those suggestions come from.

use specforge_common::{SourceSpan, Sym};
use specforge_registry::compilation::EntityView;
use specforge_registry::{KindRegistry, detect_unknown_entity_fields};
use specforge_test_macros::test as spec;

use crate::support::{build, software, span};

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "unregistered keyword produces E024"
)]
fn detect_unknown_kinds_e024() {
    let build = build([software()]);
    let entities = vec![EntityView::new("unknown_thing", "u1", span("test.spec"))];
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
    let entities = vec![EntityView::new("unknown_thing", "u1", &s)];
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
    let entities = vec![EntityView::new("behavior", "b1", span("test.spec"))];
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
    let entities = vec![EntityView::new("define", "my_define", span("test.spec"))];
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
    let unknown = vec![EntityView::new("xyzzy", "x1", span("t.spec"))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&unknown, &build.kinds, None);
    assert!(d1.iter().any(|d| d.code == "E024"));
    let known = vec![EntityView::new("behavior", "b1", span("t.spec"))];
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
    let entities = vec![EntityView::new("behavior", "b1", span("test.spec"))];
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
    let entities = vec![EntityView::new("xyzzy", "x1", span("test.spec"))];
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
        &[EntityView::new("feature", "f1", span("test.spec"))],
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
        &[EntityView::new("invariant", "i1", span("test.spec"))
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
        &[EntityView::new("feature", "f1", span("test.spec"))],
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
    let e1 = vec![EntityView::new("behavior", "b1", span("test.spec"))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
    assert!(
        d1[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("@specforge/software")
    );
    let e2 = vec![EntityView::new("xyzzy", "x1", span("test.spec"))];
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
