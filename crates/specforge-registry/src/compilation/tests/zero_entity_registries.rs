// Integration tests for spec/behaviors/zero-entity-registries.spec
//
// Covers verify statements for:
//   - boot_empty_kind_registry (6 verifies)
//   - boot_empty_field_registry (4 verifies)
//   - boot_empty_edge_registry (3 verifies)
//   - populate_kind_registry_from_extensions (6 verifies)
//   - populate_field_registry_from_extensions (6 verifies)
//   - populate_edge_registry_from_extensions (4 verifies)
//   - register_entity_kinds_from_manifest (9 verifies)
//   - register_edge_types_from_manifest (6 verifies)
//   - validate_manifest_v2_schema (3 verifies; the load's two are in specforge-project)
//   - detect_unknown_entity_kinds (5 verifies)
//   - suggest_missing_extensions (4 verifies)
//   - validate_registered_entity_fields (0; proven in specforge-project)
//   - detect_duplicate_entity_kinds (4+1 verifies)
//   - validate_peer_dependencies (4 verifies)
//   - validate_extension_testability (5 verifies)
//   - register_validation_rules_from_manifest (6 verifies)
//   - register_extension_validation_rules (3 verifies)
//   - apply_entity_enhancements (5 verifies)

use specforge_test_macros::test as spec;

use specforge_common::{Severity, SourceSpan, Sym};
use specforge_extension_sdk::{ContributionsBuilder, EnhancementBuilder, ExtensionMeta, FieldType};
use specforge_protocol_types::{EntityEnhancementDescriptor, ExtensionDeclaration};
use specforge_registry::compilation::EntityView;
use specforge_registry::compilation::apply_entity_enhancements;
use specforge_registry::{
    EdgeRegistry, FieldRegistry, FieldRegistryEntry, KindRegistry, ManifestFieldType,
    detect_unknown_entity_fields,
};

use super::support::{declare, software};
use crate::compilation::declaration::shape;
use crate::compilation::populate::populate;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The enhancement `extension` declares on `target` (owned by itself),
/// with what `f` adds.
fn enhancement(
    extension: &str,
    target: &str,
    f: impl FnOnce(&mut EnhancementBuilder),
) -> (String, EntityEnhancementDescriptor) {
    let declaration = declare(extension, |c| {
        c.enhance(target, extension, f);
    });
    (extension.to_string(), declaration.enhancements[0].clone())
}

#[allow(dead_code)]
fn span(file: &str) -> SourceSpan {
    SourceSpan {
        file: Sym::new(file),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

/// A span that outlives the test's entity views.
fn pinned(span: SourceSpan) -> &'static SourceSpan {
    Box::leak(Box::new(span))
}

// ===========================================================================
// B:boot_empty_kind_registry (6 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "KindRegistry::new() has zero entries"
)]
fn boot_kind_registry_zero_entries() {
    let registry = KindRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes spec keyword without extensions"
)]
fn boot_kind_registry_spec_keyword() {
    let registry = KindRegistry::new();
    // spec is a structural keyword — NOT in KindRegistry
    assert!(!registry.contains("spec"));
    // But the parser recognizes it (tested via specforge_parser)
    let parsed = specforge_parser::parse(
        "spec my_spec \"My Spec\" {\n  version \"1.0\"\n}\n",
        "test.spec",
    );
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "spec");
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes ref keyword without extensions"
)]
fn boot_kind_registry_ref_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("ref"));
    let parsed = specforge_parser::parse("ref gh.issue:42 \"Fix bug\"\n", "test.spec");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "ref");
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "parser recognizes use keyword without extensions"
)]
fn boot_kind_registry_use_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("use"));
    let parsed = specforge_parser::parse("use \"types/core\"\n", "test.spec");
    assert_eq!(parsed.imports.len(), 1);
}

#[test]
fn boot_kind_registry_define_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("define"));
    let parsed = specforge_parser::parse(
        "define user_story {\n  required [description]\n}\n",
        "test.spec",
    );
    let has_define = parsed.entities.iter().any(|e| e.kind.raw == "define");
    assert!(
        has_define || parsed.entities.is_empty(),
        "define should parse via grammar rule"
    );
}

#[spec(
    behavior = "boot_empty_kind_registry",
    verify = "Boot Empty Kind Registry: empty kind registry boot holds — compiler_initializing, kind_registry_empty, structural_keywords_ready"
)]
fn boot_kind_registry_contract() {
    let registry = KindRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("behavior").is_none());
    assert!(registry.get("").is_none());
    assert!(!registry.contains("behavior"));
    assert_eq!(registry.keywords().count(), 0);
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:boot_empty_field_registry (4 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "FieldRegistry::new() has zero entries"
)]
fn boot_field_registry_zero_entries() {
    let registry = FieldRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "no field names recognized before extension loading"
)]
fn boot_field_registry_no_fields() {
    let registry = FieldRegistry::new();
    assert!(registry.get("behavior", "contract").is_none());
    assert!(!registry.contains("behavior", "contract"));
    assert!(registry.fields_for_kind("behavior").is_empty());
}

#[test]
fn boot_field_registry_title_not_a_field() {
    let mut registry = FieldRegistry::new();
    registry.register(FieldRegistryEntry {
        kind_name: "behavior".to_string(),
        field_type: ManifestFieldType::Block,
        source_extension: "@specforge/software".to_string(),
        proof_role: None,
        declared: specforge_protocol_types::FieldDescriptor {
            name: "contract".to_string(),
            ..Default::default()
        },
    });
    // title is NOT a field — it's a grammar-level construct
    assert!(registry.get("behavior", "title").is_none());
}

#[spec(
    behavior = "boot_empty_field_registry",
    verify = "Boot Empty Field Registry: empty field registry boot holds — compiler_initializing, field_registry_empty, no_fields_recognized"
)]
fn boot_field_registry_contract() {
    let registry = FieldRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("behavior", "contract").is_none());
    assert!(registry.fields_for_kind("behavior").is_empty());
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:boot_empty_edge_registry (3 verifies)
// ===========================================================================

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "edge type set starts with zero entries"
)]
fn boot_edge_registry_zero_entries() {
    let registry = EdgeRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "no edge labels recognized before extension loading"
)]
fn boot_edge_registry_no_labels() {
    let registry = EdgeRegistry::new();
    assert!(registry.get("enforces").is_none());
    assert!(!registry.contains("enforces"));
}

#[spec(
    behavior = "boot_empty_edge_registry",
    verify = "Boot Empty Edge Registry: empty edge registry boot holds — compiler_initializing, edge_registry_empty, no_edges_recognized"
)]
fn boot_edge_registry_contract() {
    let registry = EdgeRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.get("enforces").is_none());
    assert_eq!(registry.labels().count(), 0);
    assert_eq!(registry.iter().count(), 0);
}

// ===========================================================================
// B:populate_kind_registry_from_extensions (6 verifies)
// ===========================================================================

// ===========================================================================
// B:populate_field_registry_from_extensions (4 verifies)
// ===========================================================================

// ===========================================================================
// B:populate_edge_registry_from_extensions (4 verifies)
// ===========================================================================

// ===========================================================================
// B:register_entity_kinds_from_manifest (8 verifies)
// ===========================================================================

// ===========================================================================
// B:register_edge_types_from_manifest (6 verifies)
// ===========================================================================

// ===========================================================================
// B:validate_manifest_v2_schema (5 verifies)
// ===========================================================================

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "a valid declaration passes validation"
)]
fn a_valid_declaration_passes_validation() {
    let diags = shape(&declare("@specforge/software", |_| {}));
    assert!(diags.is_empty());
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "missing required field produces hard error"
)]
fn a_declaration_without_name_or_version_is_e030() {
    let nameless = shape(&ContributionsBuilder::new(ExtensionMeta::new("", "1.0.0")).declaration());
    assert_eq!(nameless.len(), 1, "{nameless:?}");
    assert_eq!(nameless[0].code, "E030");
    assert_eq!(nameless[0].severity, Severity::Error);
    assert!(
        nameless[0].message.contains("name is empty"),
        "{nameless:?}"
    );

    let unversioned =
        shape(&ContributionsBuilder::new(ExtensionMeta::new("@test/ext", "")).declaration());
    assert_eq!(unversioned.len(), 1, "{unversioned:?}");
    assert_eq!(unversioned[0].code, "E030");
    assert!(
        unversioned[0]
            .message
            .contains("'@test/ext': its version is empty"),
        "{unversioned:?}"
    );
}

#[spec(
    behavior = "validate_manifest_v2_schema",
    verify = "Validate Extension Declaration: declaration validation holds — declaration_loaded, shape_validated, malformed_diagnosed"
)]
fn declaration_validation_contract() {
    let good_diags = shape(&declare("@specforge/software", |_| {}));
    assert!(good_diags.is_empty());

    // No name, no version, and a short name that names no CLI subcommand.
    let mut bad = ContributionsBuilder::new(ExtensionMeta::new("", ""));
    bad.meta.short = Some("Not Kebab".to_string());
    let bad_diags = shape(&bad.declaration());
    assert!(bad_diags.len() >= 3);
    assert!(bad_diags.iter().all(|d| d.code == "E030"));
}

// ===========================================================================
// B:detect_unknown_entity_kinds (5 verifies)
// ===========================================================================

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "unregistered keyword produces E024"
)]
fn detect_unknown_kinds_e024() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let entities = vec![EntityView::new(
        "unknown_thing",
        "u1",
        pinned(span("test.spec")),
    )];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E024");
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "E024 includes keyword name and source span"
)]
fn detect_unknown_kinds_e024_includes_info() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let s = SourceSpan {
        file: Sym::new("my/file.spec"),
        start_line: 42,
        start_col: 0,
        end_line: 42,
        end_col: 10,
    };
    let entities = vec![EntityView::new("unknown_thing", "u1", &s)];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags[0].message.contains("unknown_thing"));
    assert!(diags[0].message.contains("my/file.spec"));
    assert_eq!(diags[0].span.as_ref().unwrap().start_line, 42);
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "registered keyword does not produce E024"
)]
fn detect_unknown_kinds_registered_no_e024() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let entities = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "define-block keywords not checked against KindRegistry"
)]
fn detect_unknown_kinds_define_not_checked() {
    let kind_reg = KindRegistry::new();
    let entities = vec![EntityView::new(
        "define",
        "my_define",
        pinned(span("test.spec")),
    )];
    let diags =
        specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(
    behavior = "detect_unknown_entity_kinds",
    verify = "Detect Unknown Entity Kinds: unknown entity kind detection holds — registries_populated_fired, structural_parse_ready, unknown_kinds_diagnosed, registered_kinds_accepted"
)]
fn detect_unknown_kinds_contract() {
    let (kind_reg, _, _, _) = populate(&[software()]);
    let unknown = vec![EntityView::new("xyzzy", "x1", pinned(span("t.spec")))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&unknown, &kind_reg, None);
    assert!(d1.iter().any(|d| d.code == "E024"));
    let known = vec![EntityView::new("behavior", "b1", pinned(span("t.spec")))];
    let d2 = specforge_registry::compilation::detect_unknown_entity_kinds(&known, &kind_reg, None);
    assert!(d2.is_empty());
}

// ===========================================================================
// B:suggest_missing_extensions (4 verifies)
// ===========================================================================

#[spec(
    behavior = "suggest_missing_extensions",
    verify = "E024 for keyword in index suggests the providing extension"
)]
fn suggest_missing_ext_known_keyword() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let entities = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
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
    let entities = vec![EntityView::new("xyzzy", "x1", pinned(span("test.spec")))];
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
        &[EntityView::new("feature", "f1", pinned(span("test.spec")))],
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
        serde_json::from_str(include_str!("../../../data/field-index.json")).unwrap();
    assert_eq!(bundled_json, expected);

    // W020 for such a field names the extension, as E024 does for a kind.
    let (kind_reg, field_reg, _, _) = populate(&[software()]);
    let diags = detect_unknown_entity_fields(
        &[
            EntityView::new("invariant", "i1", pinned(span("test.spec")))
                .with_fields(&["expression", "bogus"]),
        ],
        &kind_reg,
        &field_reg,
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
        &[EntityView::new("feature", "f1", pinned(span("test.spec")))],
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
    let e1 = vec![EntityView::new("behavior", "b1", pinned(span("test.spec")))];
    let d1 =
        specforge_registry::compilation::detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
    assert!(
        d1[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("@specforge/software")
    );
    let e2 = vec![EntityView::new("xyzzy", "x1", pinned(span("test.spec")))];
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

// ===========================================================================
// B:validate_registered_entity_fields: the load reports W021 for these
// (crates/specforge-project/tests/registered_fields.rs proves all 6
// verifies through Environment::load). An edge label a field maps to is
// still registered, as an implicit edge:
// ===========================================================================

// ===========================================================================
// B:detect_duplicate_entity_kinds (4 verifies)
// ===========================================================================

// ===========================================================================
// B:validate_peer_dependencies (4 verifies)
// ===========================================================================

// ===========================================================================
// B:validate_extension_testability (5 verifies)
// ===========================================================================

// ===========================================================================
// B:register_validation_rules_from_manifest (6 verifies)
// ===========================================================================

// ===========================================================================
// B:register_extension_validation_rules (3 verifies — from spec)
// ===========================================================================

// ===========================================================================
// B:register_entity_enhancements (5 verifies)
// ===========================================================================

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement fields registered in FieldRegistry"
)]
fn enhancements_merge_fields() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/coverage", "behavior", |e| {
        e.field("coverage_threshold", |f| {
            f.field_type(FieldType::String);
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "coverage_threshold"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "unknown target kind produces I004 info diagnostic"
)]
fn enhancements_unknown_kind_i004() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/ext", "nonexistent_kind", |e| {
        e.field("extra", |f| {
            f.field_type(FieldType::String);
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "I004");
    assert!(diags[0].message.contains("nonexistent_kind"));
    assert!(!field_reg.contains("nonexistent_kind", "extra"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement field does NOT overwrite existing kind-level field"
)]
fn enhancements_no_overwrite() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![enhancement("@test/ext", "behavior", |e| {
        e.field("contract", |f| {
            f.field_type(FieldType::String); // different type!
        });
    })];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    assert_eq!(contract.source_extension, "@specforge/software");
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "enhancement fields registered in FieldRegistry"
)]
fn enhancements_two_non_conflicting() {
    let (mut kind_reg, mut field_reg, _, _) = populate(&[software()]);
    let enhancements = vec![
        enhancement("@ext/a", "behavior", |e| {
            e.field("priority", |f| {
                f.field_type(FieldType::String);
            });
        }),
        enhancement("@ext/b", "behavior", |e| {
            e.field("category", |f| {
                f.field_type(FieldType::String);
            });
        }),
    ];
    let diags = apply_entity_enhancements(&enhancements, &[], &mut kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "priority"));
    assert!(field_reg.contains("behavior", "category"));
}

#[spec(
    behavior = "register_entity_enhancements",
    verify = "Register Entity Enhancements: entity enhancement registration holds — manifests_validated, enhancement_registered_emitted, registration_before_resolve, registration_order_deterministic"
)]
fn enhancements_contract() {
    // Two extensions each add an `owner` field to behavior, with different types.
    let enhancer = |name: &str, field_type: FieldType| -> ExtensionDeclaration {
        declare(name, |c| {
            c.enhance("behavior", name, |e| {
                e.field("owner", |f| {
                    f.field_type(field_type);
                });
                e.field(&format!("{}_note", field_type.as_str()), |f| {
                    f.field_type(FieldType::String);
                });
            });
        })
    };
    let a = enhancer("@test/a", FieldType::String);
    let b = enhancer("@test/b", FieldType::Reference);

    // requires manifests_validated: every declaration passes its shape check.
    for d in [&software(), &a, &b] {
        assert!(shape(d).is_empty(), "{}", d.name());
    }

    let (kind_reg, field_reg, _, diags) = populate(&[software(), a.clone(), b.clone()]);
    assert!(diags.is_empty(), "{diags:?}");

    // enhancement_registered_emitted: each registered field records the
    // extension, target kind, field and type the event carries.
    let owner = field_reg.get("behavior", "owner").unwrap();
    assert_eq!(owner.kind_name, "behavior");
    assert_eq!(owner.source_extension, "@test/a");
    assert_eq!(owner.field_type, ManifestFieldType::String);
    let note = field_reg.get("behavior", "reference_note").unwrap();
    assert_eq!(note.source_extension, "@test/b");

    // registration_before_resolve: the registries populate hands
    // on already accept an enhanced field on a parsed entity.
    let unknown = detect_unknown_entity_fields(
        &[EntityView::new("behavior", "b1", pinned(span("main.spec")))
            .with_fields(&["owner", "string_note"])],
        &kind_reg,
        &field_reg,
    );
    assert!(unknown.is_empty(), "{unknown:?}");

    // registration_order_deterministic: the extensions array order decides
    // which `owner` wins, and the same order always gives the same result.
    let (_, swapped, _, _) = populate(&[software(), b.clone(), a.clone()]);
    let owner = swapped.get("behavior", "owner").unwrap();
    assert_eq!(owner.source_extension, "@test/b");
    assert_eq!(owner.field_type, ManifestFieldType::Reference);
    for _ in 0..3 {
        let (_, again, _, _) = populate(&[software(), a.clone(), b.clone()]);
        assert_eq!(
            again.get("behavior", "owner").unwrap().source_extension,
            "@test/a"
        );
    }
}

// ===========================================================================
// B:validate_extension_manifest_consistency (6 verifies)
// ===========================================================================
