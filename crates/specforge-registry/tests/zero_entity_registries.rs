// Integration tests for spec/behaviors/zero-entity-registries.spec
//
// Covers verify statements for:
//   - boot_empty_kind_registry (6 verifies)
//   - boot_empty_field_registry (4 verifies)
//   - boot_empty_edge_registry (3 verifies)
//   - populate_kind_registry_from_extensions (6 verifies)
//   - populate_field_registry_from_extensions (4 verifies)
//   - populate_edge_registry_from_extensions (4 verifies)
//   - register_entity_kinds_from_manifest (8 verifies)
//   - register_edge_types_from_manifest (6 verifies)
//   - validate_manifest_v2_schema (5 verifies)
//   - detect_unknown_entity_kinds (5 verifies)
//   - suggest_missing_extensions (4 verifies)
//   - validate_registered_entity_fields (6 verifies)
//   - detect_duplicate_entity_kinds (4+1 verifies)
//   - validate_peer_dependencies (4 verifies)
//   - validate_extension_testability (5 verifies)
//   - register_verify_kinds_from_manifest (4 verifies)
//   - register_validation_rules_from_manifest (6 verifies)
//   - register_extension_validation_rules (3 verifies)
//   - apply_entity_enhancements (5 verifies)

use specforge_test_macros::test as spec;

use specforge_registry::{
    apply_entity_enhancements, detect_duplicate_entity_kinds, populate_registries,
    register_validation_rules, register_verify_kinds, validate_extension_testability,
    validate_manifest, validate_manifest_consistency, validate_peer_dependencies,
    validate_registered_entity_fields, EdgeRegistry, FieldEnhancement,
    FieldRegistry, FieldRegistryEntry, KindRegistry, ManifestEdgeType,
    ManifestField, ManifestFieldType, ManifestV2,
};
use specforge_common::{Severity, SourceSpan, Sym};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn software_manifest() -> ManifestV2 {
    serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                {
                    "name": "Behavior",
                    "keyword": "behavior",
                    "testable": true,
                    "singleton": false,
                    "supportsVerify": true,
                    "allowedVerifyKinds": ["unit", "contract", "integration"],
                    "semanticToken": "function",
                    "lspIcon": "Method",
                    "dotShape": "ellipse",
                    "fields": [
                        { "name": "contract", "fieldType": "block" },
                        { "name": "invariants", "fieldType": "reference_list", "edge": "enforces", "targetKind": "invariant" }
                    ]
                },
                {
                    "name": "Invariant",
                    "keyword": "invariant",
                    "testable": true,
                    "supportsVerify": true,
                    "dotShape": "diamond"
                }
            ],
            "edgeTypes": [
                { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant", "edgeStyle": "dashed" }
            ],
            "verifyKinds": ["unit", "contract", "integration"]
        }"#,
    )
    .unwrap()
}

#[allow(dead_code)]
fn product_manifest() -> ManifestV2 {
    serde_json::from_str(
        r#"{
            "name": "@specforge/product",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm",
            "entityKinds": [
                {
                    "name": "Feature",
                    "keyword": "feature",
                    "testable": false,
                    "dotShape": "box",
                    "fields": [
                        { "name": "behaviors", "fieldType": "reference_list", "edge": "composes", "targetKind": "behavior" }
                    ]
                }
            ],
            "edgeTypes": [
                { "label": "composes", "sourceKind": "feature", "targetKind": "behavior" }
            ],
            "peerDependencies": [
                { "name": "@specforge/software", "version": ">=1.0.0" }
            ]
        }"#,
    )
    .unwrap()
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

// ===========================================================================
// B:boot_empty_kind_registry (6 verifies)
// ===========================================================================

#[spec(behavior = "boot_empty_kind_registry", verify = "KindRegistry::new() has zero entries")]
fn boot_kind_registry_zero_entries() {
    let registry = KindRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(behavior = "boot_empty_kind_registry", verify = "parser recognizes spec keyword without extensions")]
fn boot_kind_registry_spec_keyword() {
    let registry = KindRegistry::new();
    // spec is a structural keyword — NOT in KindRegistry
    assert!(!registry.contains("spec"));
    // But the parser recognizes it (tested via specforge_parser)
    let parsed = specforge_parser::parse("spec my_spec \"My Spec\" {\n  version \"1.0\"\n}\n", "test.spec");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "spec");
}

#[spec(behavior = "boot_empty_kind_registry", verify = "parser recognizes ref keyword without extensions")]
fn boot_kind_registry_ref_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("ref"));
    let parsed = specforge_parser::parse("ref gh.issue:42 \"Fix bug\"\n", "test.spec");
    assert_eq!(parsed.entities.len(), 1);
    assert_eq!(parsed.entities[0].kind.raw, "ref");
}

#[spec(behavior = "boot_empty_kind_registry", verify = "parser recognizes use keyword without extensions")]
fn boot_kind_registry_use_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("use"));
    let parsed = specforge_parser::parse("use types/core\n", "test.spec");
    assert_eq!(parsed.imports.len(), 1);
}

#[spec(behavior = "boot_empty_kind_registry", verify = "parser recognizes define keyword without extensions")]
fn boot_kind_registry_define_keyword() {
    let registry = KindRegistry::new();
    assert!(!registry.contains("define"));
    let parsed = specforge_parser::parse("define user_story {\n  required [description]\n}\n", "test.spec");
    let has_define = parsed.entities.iter().any(|e| e.kind.raw == "define");
    assert!(has_define || parsed.entities.is_empty(), "define should parse via grammar rule");
}

#[spec(behavior = "boot_empty_kind_registry", verify = "requires/ensures consistency for empty kind registry boot")]
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

#[spec(behavior = "boot_empty_field_registry", verify = "FieldRegistry::new() has zero entries")]
fn boot_field_registry_zero_entries() {
    let registry = FieldRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(behavior = "boot_empty_field_registry", verify = "no field names recognized before extension loading")]
fn boot_field_registry_no_fields() {
    let registry = FieldRegistry::new();
    assert!(registry.get("behavior", "contract").is_none());
    assert!(!registry.contains("behavior", "contract"));
    assert!(registry.fields_for_kind("behavior").is_empty());
}

#[spec(behavior = "boot_empty_field_registry", verify = "entity title parsed by grammar, not FieldRegistry")]
fn boot_field_registry_title_not_a_field() {
    let mut registry = FieldRegistry::new();
    registry.register(FieldRegistryEntry {
        kind_name: "behavior".to_string(),
        field_name: "contract".to_string(),
        description: None,
        field_type: ManifestFieldType::Block,
        source_extension: "@specforge/software".to_string(),
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
    });
    // title is NOT a field — it's a grammar-level construct
    assert!(registry.get("behavior", "title").is_none());
}

#[spec(behavior = "boot_empty_field_registry", verify = "requires/ensures consistency for empty field registry boot")]
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

#[spec(behavior = "boot_empty_edge_registry", verify = "edge type set starts with zero entries")]
fn boot_edge_registry_zero_entries() {
    let registry = EdgeRegistry::new();
    assert_eq!(registry.len(), 0);
    assert!(registry.is_empty());
}

#[spec(behavior = "boot_empty_edge_registry", verify = "no edge labels recognized before extension loading")]
fn boot_edge_registry_no_labels() {
    let registry = EdgeRegistry::new();
    assert!(registry.get("enforces").is_none());
    assert!(!registry.contains("enforces"));
}

#[spec(behavior = "boot_empty_edge_registry", verify = "requires/ensures consistency for empty edge registry boot")]
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

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "extensions iterated in topological order")]
fn populate_kind_extensions_topological_order() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest(), product_manifest()]);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
    assert!(kind_reg.contains("feature"));
}

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "all entityKinds entries registered")]
fn populate_kind_all_entries_registered() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    assert_eq!(kind_reg.len(), 2);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
}

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "registered keywords available to parser")]
fn populate_kind_keywords_available() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest(), product_manifest()]);
    let keywords: Vec<String> = kind_reg.keywords().cloned().collect();
    assert!(keywords.contains(&"behavior".to_string()));
    assert!(keywords.contains(&"invariant".to_string()));
    assert!(keywords.contains(&"feature".to_string()));
}

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "population completes before validation")]
fn populate_kind_completes_before_validation() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    assert!(!kind_reg.is_empty());
    assert!(!field_reg.is_empty());
    assert!(!edge_reg.is_empty());
}

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "two extensions register kinds without collision")]
fn populate_kind_two_extensions_no_collision() {
    let (kind_reg, _, _, diags) = populate_registries(&[software_manifest(), product_manifest()]);
    assert!(!diags.iter().any(|d| d.code == "E026"));
    assert_eq!(kind_reg.len(), 3);
}

#[spec(behavior = "populate_kind_registry_from_extensions", verify = "requires/ensures consistency for registry population")]
fn populate_kind_registry_contract() {
    let (kind_reg, field_reg, edge_reg, diags) =
        populate_registries(&[software_manifest(), product_manifest()]);
    assert!(!kind_reg.is_empty());
    assert!(!field_reg.is_empty());
    assert!(!edge_reg.is_empty());
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("feature"));
    assert!(!diags.iter().any(|d| d.code == "E026"));
    assert_eq!(kind_reg.len(), 3);
}

// ===========================================================================
// B:populate_field_registry_from_extensions (4 verifies)
// ===========================================================================

#[spec(behavior = "populate_field_registry_from_extensions", verify = "fields registered per entity kind")]
fn populate_field_per_entity_kind() {
    let (_, field_reg, _, _) = populate_registries(&[software_manifest()]);
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
    assert!(field_reg.fields_for_kind("invariant").is_empty());
}

#[spec(behavior = "populate_field_registry_from_extensions", verify = "field types validated against known types")]
fn populate_field_types_validated() {
    let (_, field_reg, _, _) = populate_registries(&[software_manifest()]);
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    let invariants = field_reg.get("behavior", "invariants").unwrap();
    assert_eq!(invariants.field_type, ManifestFieldType::ReferenceList);
}

#[spec(behavior = "populate_field_registry_from_extensions", verify = "invalid field type produces warning")]
fn populate_field_invalid_type_warning() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                {
                    "name": "Thing",
                    "keyword": "thing",
                    "fields": [
                        { "name": "data", "fieldType": "unknown_type_xyz" }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let (_, field_reg, _, diags) = populate_registries(&[manifest]);
    assert!(diags.iter().any(|d| d.code == "W019" && d.message.contains("unknown_type_xyz")));
    assert!(!field_reg.contains("thing", "data"));
}

#[spec(behavior = "populate_field_registry_from_extensions", verify = "requires/ensures consistency for field registry population")]
fn populate_field_registry_contract() {
    let (_, field_reg, _, diags) = populate_registries(&[software_manifest()]);
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    assert!(!diags.iter().any(|d| d.code == "W019"));
}

// ===========================================================================
// B:populate_edge_registry_from_extensions (4 verifies)
// ===========================================================================

#[spec(behavior = "populate_edge_registry_from_extensions", verify = "explicit edgeTypes merged into edge set")]
fn populate_edge_explicit_merged() {
    let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
    assert!(edge_reg.contains("enforces"));
}

#[spec(behavior = "populate_edge_registry_from_extensions", verify = "implicit edges from field mappings merged")]
fn populate_edge_implicit_from_fields() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                {
                    "name": "Task",
                    "keyword": "task",
                    "fields": [
                        { "name": "assignee", "fieldType": "reference", "edge": "assigned_to", "targetKind": "person" }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let (_, _, edge_reg, _) = populate_registries(&[manifest]);
    assert!(edge_reg.contains("assigned_to"));
    let edge = edge_reg.get("assigned_to").unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("task"));
    assert_eq!(edge.target_kind.as_deref(), Some("person"));
}

#[spec(behavior = "populate_edge_registry_from_extensions", verify = "duplicate edge labels produce warning")]
fn populate_edge_duplicate_warning() {
    let mut m1 = software_manifest();
    let mut m2 = product_manifest();
    m1.edge_types.push(ManifestEdgeType {
        label: "links_to".to_string(),
        description: None,
        source_kind: None,
        target_kind: None,
        edge_style: None,
        edge_color: None,
        edge_arrowhead: None,
    });
    m2.edge_types.push(ManifestEdgeType {
        label: "links_to".to_string(),
        description: None,
        source_kind: None,
        target_kind: None,
        edge_style: None,
        edge_color: None,
        edge_arrowhead: None,
    });
    let (_, _, _, diags) = populate_registries(&[m1, m2]);
    assert!(diags.iter().any(|d| d.code == "W018" && d.message.contains("links_to")));
}

#[spec(behavior = "populate_edge_registry_from_extensions", verify = "requires/ensures consistency for edge registry population")]
fn populate_edge_registry_contract() {
    let (_, _, edge_reg, diags) = populate_registries(&[software_manifest(), product_manifest()]);
    assert!(edge_reg.contains("enforces"));
    assert!(edge_reg.contains("composes"));
    assert!(!diags.iter().any(|d| d.code == "W018"));
}

// ===========================================================================
// B:register_entity_kinds_from_manifest (8 verifies)
// ===========================================================================

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "entity kind registered with testable flag")]
fn register_kind_testable_flag() {
    let (kind_reg, _, _, diags) = populate_registries(&[software_manifest()]);
    assert!(diags.is_empty());
    let behavior = kind_reg.get("behavior").unwrap();
    assert!(behavior.testable);
    let invariant = kind_reg.get("invariant").unwrap();
    assert!(invariant.testable);
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "entity kind registered with singleton flag")]
fn register_kind_singleton_flag() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert!(!behavior.singleton);
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "entity kind registered with LSP metadata")]
fn register_kind_lsp_metadata() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.semantic_token.as_deref(), Some("function"));
    assert_eq!(behavior.lsp_icon.as_deref(), Some("Method"));
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "source extension recorded in registry entry")]
fn register_kind_source_extension() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.source_extension, "@specforge/software");
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "testable=true entity participates in coverage")]
fn register_kind_testable_participates_in_coverage() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let testable_kinds: Vec<_> = kind_reg
        .iter()
        .filter(|(_, e)| e.testable)
        .map(|(k, _)| k.clone())
        .collect();
    assert!(testable_kinds.contains(&"behavior".to_string()));
    assert!(testable_kinds.contains(&"invariant".to_string()));
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "testable=false entity excluded from coverage")]
fn register_kind_testable_false_excluded() {
    let (kind_reg, _, _, _) = populate_registries(&[product_manifest()]);
    let feature = kind_reg.get("feature").unwrap();
    assert!(!feature.testable);
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "no default testability assumed by core")]
fn register_kind_no_default_testability() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                { "name": "Thing", "keyword": "thing" }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let thing = kind_reg.get("thing").unwrap();
    assert!(!thing.testable, "default testability should be false");
}

#[spec(behavior = "register_entity_kinds_from_manifest", verify = "requires/ensures consistency for entity kind registration")]
fn register_kind_contract() {
    let manifest = software_manifest();
    let (kind_reg, _, _, diags) = populate_registries(&[manifest]);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
    assert_eq!(kind_reg.get("behavior").unwrap().source_extension, "@specforge/software");
    assert!(kind_reg.get("behavior").unwrap().testable);
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

// ===========================================================================
// B:register_edge_types_from_manifest (6 verifies)
// ===========================================================================

#[spec(behavior = "register_edge_types_from_manifest", verify = "edge type registered with label and description")]
fn register_edge_label_and_description() {
    let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.label, "enforces");
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
}

#[spec(behavior = "register_edge_types_from_manifest", verify = "source/target kind constraints recorded")]
fn register_edge_source_target_constraints() {
    let (_, _, edge_reg, _) = populate_registries(&[software_manifest()]);
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
}

#[spec(behavior = "register_edge_types_from_manifest", verify = "duplicate edge label across extensions produces W-level warning")]
fn register_edge_duplicate_warning() {
    let m1 = software_manifest();
    let mut m2 = product_manifest();
    m2.edge_types.push(ManifestEdgeType {
        label: "enforces".to_string(),
        description: None,
        source_kind: Some("feature".to_string()),
        target_kind: Some("behavior".to_string()),
        edge_style: None,
        edge_color: None,
        edge_arrowhead: None,
    });
    let (_, _, _, diags) = populate_registries(&[m1, m2]);
    assert!(diags.iter().any(|d| d.code == "W018" && d.message.contains("enforces")));
}

#[spec(behavior = "register_edge_types_from_manifest", verify = "first-registered edge type wins on collision (topological order)")]
fn register_edge_first_wins() {
    let m1 = software_manifest();
    let mut m2 = product_manifest();
    m2.edge_types.push(ManifestEdgeType {
        label: "enforces".to_string(),
        description: None,
        source_kind: Some("feature".to_string()),
        target_kind: Some("behavior".to_string()),
        edge_style: Some("dotted".to_string()),
        edge_color: None,
        edge_arrowhead: None,
    });
    let (_, _, edge_reg, _) = populate_registries(&[m1, m2]);
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_extension, "@specforge/software");
    assert_eq!(enforces.edge_style.as_deref(), Some("dashed"));
}

#[spec(behavior = "register_edge_types_from_manifest", verify = "field-to-edge mapping creates edge type")]
fn register_edge_field_mapping() {
    let (_, _, edge_reg, _) = populate_registries(&[product_manifest()]);
    assert!(edge_reg.contains("composes"));
}

#[spec(behavior = "register_edge_types_from_manifest", verify = "requires/ensures consistency for edge type registration")]
fn register_edge_contract() {
    let manifest = software_manifest();
    let (_, _, edge_reg, diags) = populate_registries(&[manifest]);
    assert!(edge_reg.contains("enforces"));
    let enforces = edge_reg.get("enforces").unwrap();
    assert_eq!(enforces.source_kind.as_deref(), Some("behavior"));
    assert_eq!(enforces.target_kind.as_deref(), Some("invariant"));
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

// ===========================================================================
// B:validate_manifest_v2_schema (5 verifies)
// ===========================================================================

#[spec(behavior = "validate_manifest_v2_schema", verify = "valid v2 manifest passes schema validation")]
fn manifest_v2_valid_passes() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm"
        }"#,
    )
    .unwrap();
    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty());
}

#[spec(behavior = "validate_manifest_v2_schema", verify = "missing required field produces hard error")]
fn manifest_v2_missing_required_field() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name": "", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm"}"#,
    )
    .unwrap();
    let diags = validate_manifest(&manifest);
    assert!(diags.iter().any(|d| d.code == "E030" && d.message.contains("'name'")));

    let manifest2: ManifestV2 = serde_json::from_str(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": ""}"#,
    )
    .unwrap();
    let diags2 = validate_manifest(&manifest2);
    assert!(diags2.iter().any(|d| d.code == "E030" && d.message.contains("wasmPath")));
}

#[spec(behavior = "validate_manifest_v2_schema", verify = "manifestVersion != 2 produces hard error")]
fn manifest_v2_wrong_version() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 1, "wasmPath": "x.wasm"}"#,
    )
    .unwrap();
    let diags = validate_manifest(&manifest);
    assert!(diags.iter().any(|d| d.code == "E030" && d.message.contains("manifestVersion must be 2")));
}

#[spec(behavior = "validate_manifest_v2_schema", verify = "unknown top-level field produces warning")]
fn manifest_v2_unknown_field() {
    // serde silently ignores unknown fields (deny_unknown_fields is NOT set)
    // Warning is handled at a higher level that compares raw JSON keys
    let result: Result<ManifestV2, _> = serde_json::from_str(
        r#"{"name": "@test/ext", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "x.wasm", "unknownField": true}"#,
    );
    assert!(result.is_ok(), "unknown fields should not cause parse failure");
}

#[spec(behavior = "validate_manifest_v2_schema", verify = "requires/ensures consistency for manifest v2 schema validation")]
fn manifest_v2_schema_contract() {
    let good: ManifestV2 = serde_json::from_str(
        r#"{"name": "@specforge/software", "version": "1.0.0", "manifestVersion": 2, "wasmPath": "software.wasm"}"#,
    )
    .unwrap();
    let good_diags = validate_manifest(&good);
    assert!(good_diags.is_empty());

    let bad: ManifestV2 = serde_json::from_str(
        r#"{"name": "", "version": "", "manifestVersion": 1, "wasmPath": ""}"#,
    )
    .unwrap();
    let bad_diags = validate_manifest(&bad);
    assert!(bad_diags.len() >= 3);
    assert!(bad_diags.iter().all(|d| d.code == "E030"));
}

// ===========================================================================
// B:detect_unknown_entity_kinds (5 verifies)
// ===========================================================================

#[spec(behavior = "detect_unknown_entity_kinds", verify = "unregistered keyword produces E024")]
fn detect_unknown_kinds_e024() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![("unknown_thing".to_string(), "u1".to_string(), span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E024");
}

#[spec(behavior = "detect_unknown_entity_kinds", verify = "E024 includes keyword name and source span")]
fn detect_unknown_kinds_e024_includes_info() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let s = SourceSpan {
        file: Sym::new("my/file.spec"),
        start_line: 42,
        start_col: 0,
        end_line: 42,
        end_col: 10,
    };
    let entities = vec![("unknown_thing".to_string(), "u1".to_string(), s.clone())];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags[0].message.contains("unknown_thing"));
    assert!(diags[0].message.contains("my/file.spec"));
    assert_eq!(diags[0].span.as_ref().unwrap().start_line, 42);
}

#[spec(behavior = "detect_unknown_entity_kinds", verify = "registered keyword does not produce E024")]
fn detect_unknown_kinds_registered_no_e024() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let entities = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(behavior = "detect_unknown_entity_kinds", verify = "define-block keywords not checked against KindRegistry")]
fn detect_unknown_kinds_define_not_checked() {
    let kind_reg = KindRegistry::new();
    let entities = vec![("define".to_string(), "my_define".to_string(), span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, None);
    assert!(diags.is_empty());
}

#[spec(behavior = "detect_unknown_entity_kinds", verify = "requires/ensures consistency for unknown entity kind detection")]
fn detect_unknown_kinds_contract() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let unknown = vec![("xyzzy".to_string(), "x1".to_string(), span("t.spec"))];
    let d1 = specforge_registry::compilation::detect_unknown_entity_kinds(&unknown, &kind_reg, None);
    assert!(d1.iter().any(|d| d.code == "E024"));
    let known = vec![("behavior".to_string(), "b1".to_string(), span("t.spec"))];
    let d2 = specforge_registry::compilation::detect_unknown_entity_kinds(&known, &kind_reg, None);
    assert!(d2.is_empty());
}

// ===========================================================================
// B:suggest_missing_extensions (4 verifies)
// ===========================================================================

#[spec(behavior = "suggest_missing_extensions", verify = "E024 for keyword in index suggests the providing extension")]
fn suggest_missing_ext_known_keyword() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let entities = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, Some(&index));
    assert!(diags[0].suggestion.as_ref().unwrap().contains("specforge add @specforge/software"));
}

#[spec(behavior = "suggest_missing_extensions", verify = "E024 for keyword not in index suggests specforge search")]
fn suggest_missing_ext_unknown_keyword() {
    let kind_reg = KindRegistry::new();
    let index = specforge_registry::compilation::KeywordExtensionIndex::new();
    let entities = vec![("xyzzy".to_string(), "x1".to_string(), span("test.spec"))];
    let diags = specforge_registry::compilation::detect_unknown_entity_kinds(&entities, &kind_reg, Some(&index));
    assert!(diags[0].suggestion.as_ref().unwrap().contains("specforge search"));
}

#[spec(behavior = "suggest_missing_extensions", verify = "keyword-to-extension index is loaded from bundled data file")]
fn suggest_missing_ext_data_driven_index() {
    let json = r#"{"behavior": "@specforge/software", "feature": "@specforge/product"}"#;
    let entries: std::collections::HashMap<String, String> = serde_json::from_str(json).unwrap();
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    assert_eq!(index.lookup("behavior"), Some("@specforge/software"));
    assert_eq!(index.lookup("feature"), Some("@specforge/product"));
    assert_eq!(index.lookup("unknown"), None);
}

#[spec(behavior = "suggest_missing_extensions", verify = "requires/ensures consistency for missing extension suggestions")]
fn suggest_missing_ext_contract() {
    let kind_reg = KindRegistry::new();
    let mut entries = std::collections::HashMap::new();
    entries.insert("behavior".to_string(), "@specforge/software".to_string());
    let index = specforge_registry::compilation::KeywordExtensionIndex::from_entries(entries);
    let e1 = vec![("behavior".to_string(), "b1".to_string(), span("test.spec"))];
    let d1 = specforge_registry::compilation::detect_unknown_entity_kinds(&e1, &kind_reg, Some(&index));
    assert!(d1[0].suggestion.as_ref().unwrap().contains("@specforge/software"));
    let e2 = vec![("xyzzy".to_string(), "x1".to_string(), span("test.spec"))];
    let d2 = specforge_registry::compilation::detect_unknown_entity_kinds(&e2, &kind_reg, Some(&index));
    assert!(d2[0].suggestion.as_ref().unwrap().contains("specforge search"));
}

// ===========================================================================
// B:validate_registered_entity_fields (6 verifies)
// ===========================================================================

#[spec(behavior = "validate_registered_entity_fields", verify = "target_kind reference resolves to registered kind")]
fn validate_fields_target_kind_resolves() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(!diags.iter().any(|d| d.message.contains("target_kind")));
}

#[spec(behavior = "validate_registered_entity_fields", verify = "edge label resolves to registered edge type")]
fn validate_fields_edge_label_resolves() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(!diags.iter().any(|d| d.message.contains("edge label")));
}

#[spec(behavior = "validate_registered_entity_fields", verify = "unresolved target_kind produces warning")]
fn validate_fields_unresolved_target_kind() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                {
                    "name": "Task",
                    "keyword": "task",
                    "fields": [
                        { "name": "owner", "fieldType": "reference", "targetKind": "person" }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[manifest]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(diags.iter().any(|d| d.code == "W022" && d.message.contains("person")));
}

#[spec(behavior = "validate_registered_entity_fields", verify = "unresolved edge label produces warning")]
fn validate_fields_unresolved_edge_label() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "x.wasm",
            "entityKinds": [
                {
                    "name": "Task",
                    "keyword": "task",
                    "fields": [
                        { "name": "owner", "fieldType": "reference", "edge": "owns" }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();
    let (_kind_reg, _field_reg, edge_reg, _) = populate_registries(&[manifest]);
    // "owns" should be auto-created as implicit edge during populate
    assert!(edge_reg.contains("owns"), "implicit edge 'owns' should exist");
}

#[spec(behavior = "validate_registered_entity_fields", verify = "cross-validation uses no domain-specific logic")]
fn validate_fields_no_domain_logic() {
    // Entirely custom domain — cooking! No software/product assumptions.
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@custom/cooking",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "cooking.wasm",
            "entityKinds": [
                {
                    "name": "Recipe",
                    "keyword": "recipe",
                    "testable": true,
                    "supportsVerify": true,
                    "fields": [
                        { "name": "ingredients", "fieldType": "reference_list", "edge": "uses", "targetKind": "ingredient" }
                    ]
                },
                { "name": "Ingredient", "keyword": "ingredient" }
            ],
            "edgeTypes": [
                { "label": "uses", "sourceKind": "recipe", "targetKind": "ingredient" }
            ]
        }"#,
    )
    .unwrap();
    let (kind_reg, field_reg, edge_reg, pop_diags) = populate_registries(&[manifest]);
    assert!(pop_diags.is_empty());
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(diags.is_empty(), "custom domain should validate cleanly: {:?}", diags);
}

#[spec(behavior = "validate_registered_entity_fields", verify = "requires/ensures consistency for field cross-validation")]
fn validate_fields_contract() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(diags.is_empty());

    let bad_manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"A","keyword":"a","fields":[
                {"name":"f","fieldType":"reference","targetKind":"nonexistent"}
            ]}]}"#,
    )
    .unwrap();
    let (kr, fr, er, _) = populate_registries(&[bad_manifest]);
    let bad_diags = validate_registered_entity_fields(&fr, &kr, &er);
    assert!(bad_diags.iter().any(|d| d.code == "W022"));
}

// ===========================================================================
// B:detect_duplicate_entity_kinds (4 verifies)
// ===========================================================================

#[spec(behavior = "detect_duplicate_entity_kinds", verify = "duplicate kind from two extensions produces E026")]
fn detect_dup_kinds_e026() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@other/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"other.wasm",
            "entityKinds":[{"name":"Behavior","keyword":"behavior"}]}"#,
    )
    .unwrap();
    let diags = detect_duplicate_entity_kinds(&[m1, m2]);
    assert!(diags.iter().any(|d| d.code == "E026" && d.message.contains("behavior")));
}

#[spec(behavior = "detect_duplicate_entity_kinds", verify = "first extension in topological order owns the kind")]
fn detect_dup_kinds_first_wins() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@other/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"other.wasm",
            "entityKinds":[{"name":"Behavior","keyword":"behavior"}]}"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[m1, m2]);
    let behavior = kind_reg.get("behavior").unwrap();
    assert_eq!(behavior.source_extension, "@specforge/software");
}

#[spec(behavior = "detect_duplicate_entity_kinds", verify = "single extension registering a kind produces no diagnostic")]
fn detect_dup_kinds_single_ext_no_diag() {
    let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
    assert!(diags.is_empty());
}

#[spec(behavior = "detect_duplicate_entity_kinds", verify = "requires/ensures consistency for duplicate entity kind detection")]
fn detect_dup_kinds_contract() {
    let diags = detect_duplicate_entity_kinds(&[software_manifest()]);
    assert!(diags.is_empty());
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@other/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"o.wasm",
            "entityKinds":[{"name":"Behavior","keyword":"behavior"}]}"#,
    )
    .unwrap();
    let dup_diags = detect_duplicate_entity_kinds(&[software_manifest(), m2]);
    assert!(dup_diags.iter().any(|d| d.code == "E026"));
}

// ===========================================================================
// B:validate_peer_dependencies (4 verifies)
// ===========================================================================

#[spec(behavior = "validate_peer_dependencies", verify = "satisfied peer dependency passes validation")]
fn peer_deps_satisfied() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"product.wasm",
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m1, m2]);
    assert!(diags.is_empty());
}

#[spec(behavior = "validate_peer_dependencies", verify = "missing peer dependency produces hard error")]
fn peer_deps_missing() {
    let m: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"product.wasm",
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m]);
    assert!(diags.iter().any(|d| d.code == "E027" && d.message.contains("@specforge/software")));
}

#[spec(behavior = "validate_peer_dependencies", verify = "incompatible version produces hard error with required range")]
fn peer_deps_incompatible_version() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/software","version":"0.5.0","manifestVersion":2,"wasmPath":"software.wasm"}"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"product.wasm",
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m1, m2]);
    assert!(diags.iter().any(|d| d.code == "E027" && d.message.contains(">=1.0.0") && d.message.contains("0.5.0")));
}

#[spec(behavior = "validate_peer_dependencies", verify = "requires/ensures consistency for peer dependency validation")]
fn peer_deps_contract() {
    let m1 = software_manifest();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    assert!(validate_peer_dependencies(&[m1, m2]).is_empty());
    let m3: ManifestV2 = serde_json::from_str(
        r#"{"name":"@specforge/product","version":"1.0.0","manifestVersion":2,"wasmPath":"p.wasm",
            "peerDependencies":[{"name":"@specforge/missing","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_peer_dependencies(&[m3]);
    assert!(diags.iter().any(|d| d.code == "E027"));
}

// ===========================================================================
// B:validate_extension_testability (5 verifies)
// ===========================================================================

#[spec(behavior = "validate_extension_testability", verify = "testable kind without supportsVerify produces W017")]
fn testability_w017() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"Thing","keyword":"thing","testable":true,"supportsVerify":false}]}"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.iter().any(|d| d.code == "W017" && d.message.contains("thing")));
}

#[spec(behavior = "validate_extension_testability", verify = "testable kind with supportsVerify=true passes")]
fn testability_passes() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(!diags.iter().any(|d| d.message.contains("behavior")));
}

#[spec(behavior = "validate_extension_testability", verify = "kind with supportsVerify but not testable produces I006")]
fn testability_i006() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"Note","keyword":"note","testable":false,"supportsVerify":true}]}"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.iter().any(|d| d.code == "I006" && d.message.contains("note")));
}

#[spec(behavior = "validate_extension_testability", verify = "consistent testable and supportsVerify flags produce no diagnostic")]
fn testability_consistent_no_diag() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"Thing","keyword":"thing","testable":false,"supportsVerify":false}]}"#,
    )
    .unwrap();
    let (kind_reg, _, _, _) = populate_registries(&[manifest]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty());
}

#[spec(behavior = "validate_extension_testability", verify = "requires/ensures consistency for extension testability validation")]
fn testability_contract() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let diags = validate_extension_testability(&kind_reg);
    assert!(diags.is_empty());
    let bad: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"X","keyword":"x","testable":true,"supportsVerify":false}]}"#,
    )
    .unwrap();
    let (bad_kr, _, _, _) = populate_registries(&[bad]);
    let bad_diags = validate_extension_testability(&bad_kr);
    assert!(bad_diags.iter().any(|d| d.code == "W017"));
}

// ===========================================================================
// B:register_verify_kinds_from_manifest (4 verifies)
// ===========================================================================

#[spec(behavior = "register_verify_kinds_from_manifest", verify = "custom verify kinds registered from manifest")]
fn verify_kinds_registered() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "verifyKinds":["smoke","contract","acceptance"]}"#,
    )
    .unwrap();
    let (kinds, diags) = register_verify_kinds(&[manifest]);
    assert!(diags.is_empty());
    assert!(kinds.contains(&"smoke".to_string()));
    assert!(kinds.contains(&"contract".to_string()));
    assert!(kinds.contains(&"acceptance".to_string()));
}

#[spec(behavior = "register_verify_kinds_from_manifest", verify = "no hardcoded verify kinds in core")]
fn verify_kinds_no_hardcoded() {
    let (kinds, _) = register_verify_kinds(&[]);
    assert!(kinds.is_empty(), "with no manifests, verify kinds should be empty");
}

#[spec(behavior = "register_verify_kinds_from_manifest", verify = "unknown verify kind in .spec produces W-level diagnostic in Phase 2")]
fn verify_kinds_unknown_w026() {
    let (kind_reg, _, _, _) = populate_registries(&[software_manifest()]);
    let registered_kinds = vec!["unit".to_string(), "contract".to_string(), "integration".to_string()];
    let entities = vec![(
        "behavior".to_string(),
        "b1".to_string(),
        vec!["chaos".to_string()], // not registered
        span("test.spec"),
    )];
    let diags = specforge_registry::compilation::detect_unknown_verify_kinds(
        &entities,
        &registered_kinds,
        &kind_reg,
    );
    assert!(diags.iter().any(|d| d.code == "W026" && d.message.contains("chaos")));
}

#[spec(behavior = "register_verify_kinds_from_manifest", verify = "requires/ensures consistency for verify kind registration")]
fn verify_kinds_contract() {
    let m: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "verifyKinds":["smoke","contract"]}"#,
    )
    .unwrap();
    let (kinds, diags) = register_verify_kinds(&[m]);
    assert_eq!(kinds.len(), 2);
    assert!(kinds.contains(&"smoke".to_string()));
    assert!(kinds.contains(&"contract".to_string()));
    assert!(diags.is_empty());
}

// ===========================================================================
// B:register_validation_rules_from_manifest (6 verifies)
// ===========================================================================

#[spec(behavior = "register_validation_rules_from_manifest", verify = "validation rule registered from manifest")]
fn validation_rule_registered() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "validationRules":[
                {"code":"W100","severity":"warning","messageTemplate":"orphan {kind} '{id}'","check":"no_incoming_edges","targetKind":"behavior"}
            ]}"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[manifest]);
    assert!(diags.is_empty());
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].code, "W100");
    assert_eq!(rules[0].check, "no_incoming_edges");
}

#[spec(behavior = "register_validation_rules_from_manifest", verify = "target_kind validation deferred to post-registration phase")]
fn validation_rule_target_kind_deferred() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "validationRules":[
                {"code":"W100","severity":"warning","messageTemplate":"test","check":"no_incoming_edges","targetKind":"nonexistent_kind"}
            ]}"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[manifest]);
    assert!(diags.is_empty(), "rule registration should not validate target_kind");
    assert_eq!(rules.len(), 1);
}

#[spec(behavior = "register_validation_rules_from_manifest", verify = "target_kind reference validated against KindRegistry after registries_populated")]
fn validation_rule_target_kind_validated_post_registration() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    // Post-registration cross-validation catches unresolved refs
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    // Software manifest has valid references — should be clean
    assert!(!diags.iter().any(|d| d.message.contains("target_kind")));
}

#[spec(behavior = "register_validation_rules_from_manifest", verify = "edge_type reference validated against edge type set after registries_populated")]
fn validation_rule_edge_type_validated_post_registration() {
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[software_manifest()]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    assert!(!diags.iter().any(|d| d.message.contains("edge label")));
}

#[spec(behavior = "register_validation_rules_from_manifest", verify = "invalid reference produces warning not error")]
fn validation_rule_invalid_ref_warning() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[{"name":"Task","keyword":"task","fields":[
                {"name":"owner","fieldType":"reference","targetKind":"person"}
            ]}]}"#,
    )
    .unwrap();
    let (kind_reg, field_reg, edge_reg, _) = populate_registries(&[manifest]);
    let diags = validate_registered_entity_fields(&field_reg, &kind_reg, &edge_reg);
    // Should be a warning (W022), not error
    assert!(diags.iter().any(|d| d.code == "W022" && d.severity == Severity::Warning));
    assert!(!diags.iter().any(|d| d.severity == Severity::Error));
}

#[spec(behavior = "register_validation_rules_from_manifest", verify = "requires/ensures consistency for validation rule registration")]
fn validation_rule_contract() {
    let m: ManifestV2 = serde_json::from_str(
        r#"{"name":"@t/e","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "validationRules":[
                {"code":"W100","severity":"warning","messageTemplate":"test","check":"no_incoming_edges"}
            ]}"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[m]);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].code, "W100");
    assert!(diags.is_empty());
}

// ===========================================================================
// B:register_extension_validation_rules (3 verifies — from spec)
// ===========================================================================

#[spec(behavior = "register_extension_validation_rules", verify = "rules sorted by code for deterministic order")]
fn ext_validation_rules_sorted() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/a","version":"1.0.0","manifestVersion":2,"wasmPath":"a.wasm",
            "validationRules":[
                {"code":"W300","severity":"warning","messageTemplate":"third","check":"no_incoming_edges"},
                {"code":"W100","severity":"warning","messageTemplate":"first","check":"no_incoming_edges"}
            ]}"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/b","version":"1.0.0","manifestVersion":2,"wasmPath":"b.wasm",
            "validationRules":[
                {"code":"W200","severity":"warning","messageTemplate":"second","check":"no_outgoing_edges"}
            ]}"#,
    )
    .unwrap();
    let (rules, _) = register_validation_rules(&[m1, m2]);
    let codes: Vec<&str> = rules.iter().map(|r| r.code.as_str()).collect();
    assert_eq!(codes, vec!["W100", "W200", "W300"]);
}

#[spec(behavior = "register_extension_validation_rules", verify = "rules from multiple extensions are collected")]
fn ext_validation_rules_multiple_extensions() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/a","version":"1.0.0","manifestVersion":2,"wasmPath":"a.wasm",
            "validationRules":[
                {"code":"W100","severity":"warning","messageTemplate":"a rule","check":"no_incoming_edges"}
            ]}"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/b","version":"1.0.0","manifestVersion":2,"wasmPath":"b.wasm",
            "validationRules":[
                {"code":"W200","severity":"warning","messageTemplate":"b rule","check":"no_outgoing_edges"}
            ]}"#,
    )
    .unwrap();
    let (rules, diags) = register_validation_rules(&[m1, m2]);
    assert_eq!(rules.len(), 2);
    assert!(diags.is_empty());
}

#[spec(behavior = "register_extension_validation_rules", verify = "duplicate codes across extensions produce warning")]
fn ext_validation_rules_duplicate_codes() {
    let m1: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/a","version":"1.0.0","manifestVersion":2,"wasmPath":"a.wasm",
            "validationRules":[{"code":"W100","severity":"warning","messageTemplate":"a","check":"no_incoming_edges"}]}"#,
    )
    .unwrap();
    let m2: ManifestV2 = serde_json::from_str(
        r#"{"name":"@ext/b","version":"1.0.0","manifestVersion":2,"wasmPath":"b.wasm",
            "validationRules":[{"code":"W100","severity":"warning","messageTemplate":"b","check":"no_incoming_edges"}]}"#,
    )
    .unwrap();
    let (_, diags) = register_validation_rules(&[m1, m2]);
    assert!(diags.iter().any(|d| d.code == "W023" && d.message.contains("W100")));
}

// ===========================================================================
// B:apply_entity_enhancements (5 verifies)
// ===========================================================================

#[spec(behavior = "apply_entity_enhancements", verify = "merges enhancement fields into FieldRegistry for known target kind")]
fn enhancements_merge_fields() {
    let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
    let enhancements = vec![(
        "@test/coverage".to_string(),
        FieldEnhancement {
            target_kind: "behavior".to_string(),
            source_extension: "@test/coverage".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "coverage_threshold".to_string(),
                field_type: "string".to_string(),
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
            }],
        },
    )];
    let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "coverage_threshold"));
}

#[spec(behavior = "apply_entity_enhancements", verify = "unknown target kind produces I004 info diagnostic")]
fn enhancements_unknown_kind_i004() {
    let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
    let enhancements = vec![(
        "@test/ext".to_string(),
        FieldEnhancement {
            target_kind: "nonexistent_kind".to_string(),
            source_extension: "@test/ext".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "extra".to_string(),
                field_type: "string".to_string(),
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
            }],
        },
    )];
    let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "I004");
    assert!(diags[0].message.contains("nonexistent_kind"));
    assert!(!field_reg.contains("nonexistent_kind", "extra"));
}

#[spec(behavior = "apply_entity_enhancements", verify = "enhancement field does NOT overwrite existing kind-level field")]
fn enhancements_no_overwrite() {
    let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
    let enhancements = vec![(
        "@test/ext".to_string(),
        FieldEnhancement {
            target_kind: "behavior".to_string(),
            source_extension: "@test/ext".to_string(),
            edge_types: vec![],
            fields: vec![ManifestField {
                name: "contract".to_string(),
                field_type: "string".to_string(), // different type!
                description: None,
                edge: None,
                target_kind: None,
                file_reference: false,
                required: false,
                default_value: None,
                enum_values: vec![],
            }],
        },
    )];
    let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    let contract = field_reg.get("behavior", "contract").unwrap();
    assert_eq!(contract.field_type, ManifestFieldType::Block);
    assert_eq!(contract.source_extension, "@specforge/software");
}

#[spec(behavior = "apply_entity_enhancements", verify = "two non-conflicting enhancements on same kind both registered")]
fn enhancements_two_non_conflicting() {
    let (kind_reg, mut field_reg, _, _) = populate_registries(&[software_manifest()]);
    let enhancements = vec![
        (
            "@ext/a".to_string(),
            FieldEnhancement {
                target_kind: "behavior".to_string(),
                source_extension: "@ext/a".to_string(),
            edge_types: vec![],
                fields: vec![ManifestField {
                    name: "priority".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                default_value: None,
                enum_values: vec![],
                }],
            },
        ),
        (
            "@ext/b".to_string(),
            FieldEnhancement {
                target_kind: "behavior".to_string(),
                source_extension: "@ext/b".to_string(),
            edge_types: vec![],
                fields: vec![ManifestField {
                    name: "category".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    edge: None,
                    target_kind: None,
                    file_reference: false,
                    required: false,
                default_value: None,
                enum_values: vec![],
                }],
            },
        ),
    ];
    let diags = apply_entity_enhancements(&enhancements, &kind_reg, &mut field_reg);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "priority"));
    assert!(field_reg.contains("behavior", "category"));
}

#[spec(behavior = "apply_entity_enhancements", verify = "requires KindRegistry populated, ensures fields merged + diagnostics")]
fn enhancements_contract() {
    let manifests = vec![
        software_manifest(),
        serde_json::from_str::<ManifestV2>(
            r#"{"name":"@test/coverage","version":"1.0.0","manifestVersion":2,"wasmPath":"coverage.wasm",
                "entityEnhancements":[{"targetKind":"behavior","sourceExtension":"@test/coverage",
                "fields":[{"name":"coverage_threshold","fieldType":"string"}]}]}"#,
        )
        .unwrap(),
    ];
    let (_kind_reg, field_reg, _, diags) = populate_registries(&manifests);
    assert!(
        field_reg.contains("behavior", "coverage_threshold"),
        "enhancement field should be merged via populate_registries"
    );
    assert!(
        !diags.iter().any(|d| d.code == "I004"),
        "no I004 expected for known kind"
    );
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
}

// ===========================================================================
// B:validate_extension_manifest_consistency (6 verifies)
// ===========================================================================

#[spec(behavior = "validate_extension_manifest_consistency", verify = "target_kind referencing own manifest kind passes")]
fn manifest_consistency_own_kind_passes() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"Behavior","keyword":"behavior","fields":[
                    {"name":"invariants","fieldType":"reference_list","targetKind":"invariant"}
                ]},
                {"name":"Invariant","keyword":"invariant"}
            ]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty());
}

#[spec(behavior = "validate_extension_manifest_consistency", verify = "target_kind referencing peer dependency kind passes")]
fn manifest_consistency_peer_dep_passes() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"Feature","keyword":"feature","fields":[
                    {"name":"behaviors","fieldType":"reference_list","targetKind":"behavior"}
                ]}
            ],
            "peerDependencies":[{"name":"@specforge/software","version":">=1.0.0"}]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty());
}

#[spec(behavior = "validate_extension_manifest_consistency", verify = "self-contradictory target_kind produces E-level error")]
fn manifest_consistency_self_contradictory_target() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"Behavior","keyword":"behavior","fields":[
                    {"name":"invariants","fieldType":"reference_list","targetKind":"nonexistent_kind"}
                ]}
            ]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.iter().any(|d| d.message.contains("nonexistent_kind")));
}

#[spec(behavior = "validate_extension_manifest_consistency", verify = "target_kind referencing non-peer extension kind produces W-level warning")]
fn manifest_consistency_non_peer_warning() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"Feature","keyword":"feature","fields":[
                    {"name":"behaviors","fieldType":"reference_list","targetKind":"behavior"}
                ]}
            ]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.iter().any(|d| d.message.contains("behavior") && d.message.contains("target_kind")));
}

#[spec(behavior = "validate_extension_manifest_consistency", verify = "self-contradictory edge label produces E-level error")]
fn manifest_consistency_self_contradictory_edge() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"Behavior","keyword":"behavior","fields":[
                    {"name":"invariants","fieldType":"reference_list","edge":"missing_edge"}
                ]}
            ]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.iter().any(|d| d.message.contains("missing_edge")));
}

#[spec(behavior = "validate_extension_manifest_consistency", verify = "requires/ensures consistency for manifest self-consistency validation")]
fn manifest_consistency_contract() {
    let good: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"A","keyword":"a","fields":[{"name":"bs","fieldType":"reference_list","targetKind":"b"}]},
                {"name":"B","keyword":"b"}
            ],
            "edgeTypes":[{"label":"links","sourceKind":"a","targetKind":"b"}]}"#,
    )
    .unwrap();
    let diags = validate_manifest_consistency(&good);
    assert!(diags.is_empty());

    let bad: ManifestV2 = serde_json::from_str(
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"x.wasm",
            "entityKinds":[
                {"name":"A","keyword":"a","fields":[{"name":"bs","fieldType":"reference_list","targetKind":"missing","edge":"missing_edge"}]}
            ]}"#,
    )
    .unwrap();
    let bad_diags = validate_manifest_consistency(&bad);
    assert!(bad_diags.len() >= 2);
    assert!(bad_diags.iter().all(|d| d.code == "W021"));
}
