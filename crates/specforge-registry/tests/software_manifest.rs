use specforge_registry::{
    populate_registries, validate_manifest, validate_manifest_consistency, ManifestV2,
};

fn load_software_manifest() -> ManifestV2 {
    let json = include_str!("../../../extensions/software/manifest.json");
    serde_json::from_str(json).expect("software manifest.json should deserialize")
}

// B:se_declare_manifest — verify unit "manifest name is @specforge/software"
#[test]
fn test_software_manifest_deserializes() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.name, "@specforge/software");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.manifest_version, 2);
}

// B:se_declare_manifest — verify unit "manifest version is 2"
#[test]
fn test_software_manifest_version_is_2() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.manifest_version, 2);
}

// B:se_declare_manifest — verify unit "manifest declares exactly 5 entity kinds"
#[test]
fn test_software_manifest_has_5_entity_kinds() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.entity_kinds.len(), 5);
    let keywords: Vec<&str> = manifest
        .entity_kinds
        .iter()
        .map(|k| k.keyword.as_str())
        .collect();
    assert!(keywords.contains(&"behavior"));
    assert!(keywords.contains(&"invariant"));
    assert!(keywords.contains(&"event"));
    assert!(keywords.contains(&"type"));
    assert!(keywords.contains(&"port"));
}

// B:se_declare_manifest — verify unit "manifest declares exactly 11 edge types"
#[test]
fn test_software_manifest_has_11_edge_types() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.edge_types.len(), 11);
    let labels: Vec<&str> = manifest
        .edge_types
        .iter()
        .map(|e| e.label.as_str())
        .collect();
    assert!(labels.contains(&"References"));
    assert!(labels.contains(&"Implements"));
    assert!(labels.contains(&"Produces"));
    assert!(labels.contains(&"Consumes"));
    assert!(labels.contains(&"UsesType"));
    assert!(labels.contains(&"UsesPort"));
    assert!(labels.contains(&"Enforces"));
    assert!(labels.contains(&"ExtendsType"));
    assert!(labels.contains(&"TestedBy"));
    assert!(labels.contains(&"ExternalRef"));
    assert!(labels.contains(&"MilestoneBehavior"));
}

// B:se_declare_manifest — verify unit "contributes declares entities and validators"
#[test]
fn test_software_manifest_contributes() {
    let manifest = load_software_manifest();
    assert!(manifest.contributes.entities);
    assert!(manifest.contributes.validators);
}

// B:se_declare_manifest — verify unit "peer_dependencies includes @specforge/product with recommended: true"
#[test]
fn test_software_manifest_peer_dependencies() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.peer_dependencies.len(), 1);
    assert_eq!(manifest.peer_dependencies[0].name, "@specforge/product");
    assert_eq!(manifest.peer_dependencies[0].version, "^1.0");
}

// B:se_declare_manifest — verify unit "entity_enhancements declares module ports and ports_defined"
#[test]
fn test_software_manifest_enhancement_module() {
    let manifest = load_software_manifest();
    let module_enh = manifest
        .entity_enhancements
        .iter()
        .find(|e| e.target_kind == "module")
        .expect("should have module enhancement");
    assert_eq!(module_enh.source_extension, "@specforge/product");
    let field_names: Vec<&str> = module_enh.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"ports"));
    assert!(field_names.contains(&"ports_defined"));
}

// B:se_declare_manifest — verify unit "entity_enhancements declares milestone behaviors"
#[test]
fn test_software_manifest_enhancement_milestone() {
    let manifest = load_software_manifest();
    let ms_enh = manifest
        .entity_enhancements
        .iter()
        .find(|e| e.target_kind == "milestone")
        .expect("should have milestone enhancement");
    assert_eq!(ms_enh.source_extension, "@specforge/product");
    let field_names: Vec<&str> = ms_enh.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"behaviors"));
    // behaviors field maps to MilestoneBehavior edge
    let beh_field = ms_enh.fields.iter().find(|f| f.name == "behaviors").unwrap();
    assert_eq!(beh_field.edge.as_deref(), Some("MilestoneBehavior"));
    assert_eq!(beh_field.target_kind.as_deref(), Some("behavior"));
}

// B:se_declare_manifest — verify unit "sandbox_policy declares no network access and read-only filesystem"
#[test]
fn test_software_manifest_sandbox_policy() {
    let manifest = load_software_manifest();
    let sandbox = manifest.sandbox_policy.as_ref().expect("should have sandbox policy");
    assert_eq!(sandbox.network_access, Some(false));
    assert_eq!(sandbox.file_system_access, Some(false));
    assert_eq!(sandbox.max_memory_mb, Some(256));
    assert_eq!(sandbox.max_execution_ms, Some(5000));
}

// B:se_declare_manifest — verify unit "host_api_version is 1.0.0"
#[test]
fn test_software_manifest_host_api_version() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.host_api_version.as_deref(), Some("1.0.0"));
}

// B:se_declare_manifest — verify unit "starter_template is templates/behavior.spec"
#[test]
fn test_software_manifest_starter_template() {
    let manifest = load_software_manifest();
    assert_eq!(
        manifest.starter_template.as_deref(),
        Some("templates/behavior.spec")
    );
}

// B:se_declare_manifest — passes schema validation
#[test]
fn test_software_manifest_passes_schema_validation() {
    let manifest = load_software_manifest();
    let diags = validate_manifest(&manifest);
    assert!(
        diags.is_empty(),
        "expected no schema validation errors, got: {:?}",
        diags
    );
}

// B:se_declare_manifest — passes internal consistency validation
#[test]
fn test_software_manifest_passes_consistency_validation() {
    let manifest = load_software_manifest();
    let diags = validate_manifest_consistency(&manifest);
    assert!(
        diags.is_empty(),
        "expected no consistency warnings, got: {:?}",
        diags
    );
}

// B:se_register_entity_kinds — verify unit "behavior registered with testable=true and supportsVerify=true"
#[test]
fn test_software_behavior_kind_metadata() {
    let manifest = load_software_manifest();
    let behavior = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "behavior")
        .unwrap();
    assert!(behavior.testable);
    assert!(behavior.supports_verify);
    assert_eq!(behavior.semantic_token.as_deref(), Some("function"));
    assert_eq!(behavior.lsp_icon.as_deref(), Some("Method"));
    assert_eq!(behavior.dot_shape.as_deref(), Some("box"));
}

// B:se_register_entity_kinds — verify unit "invariant registered with testable=true and supportsVerify=true"
#[test]
fn test_software_invariant_kind_metadata() {
    let manifest = load_software_manifest();
    let invariant = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "invariant")
        .unwrap();
    assert!(invariant.testable);
    assert!(invariant.supports_verify);
    assert_eq!(invariant.semantic_token.as_deref(), Some("property"));
    assert_eq!(invariant.lsp_icon.as_deref(), Some("Property"));
    assert_eq!(invariant.dot_shape.as_deref(), Some("diamond"));
}

// B:se_register_entity_kinds — verify unit "event registered with semanticToken=event"
#[test]
fn test_software_event_kind_metadata() {
    let manifest = load_software_manifest();
    let event = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "event")
        .unwrap();
    assert!(event.testable);
    assert!(event.supports_verify);
    assert_eq!(event.semantic_token.as_deref(), Some("event"));
    assert_eq!(event.lsp_icon.as_deref(), Some("Event"));
    assert_eq!(event.dot_shape.as_deref(), Some("ellipse"));
}

// B:se_register_entity_kinds — verify unit "type registered with testable=true, supportsVerify=true, dotShape=rectangle"
#[test]
fn test_software_type_kind_metadata() {
    let manifest = load_software_manifest();
    let type_kind = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "type")
        .unwrap();
    assert!(type_kind.testable);
    assert!(type_kind.supports_verify);
    assert_eq!(type_kind.semantic_token.as_deref(), Some("type"));
    assert_eq!(type_kind.lsp_icon.as_deref(), Some("Struct"));
    assert_eq!(type_kind.dot_shape.as_deref(), Some("rectangle"));
}

// B:se_register_entity_kinds — verify unit "port registered with testable=true, supportsVerify=true, lspIcon=Interface"
#[test]
fn test_software_port_kind_metadata() {
    let manifest = load_software_manifest();
    let port = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "port")
        .unwrap();
    assert!(port.testable);
    assert!(port.supports_verify);
    assert_eq!(port.semantic_token.as_deref(), Some("interface"));
    assert_eq!(port.lsp_icon.as_deref(), Some("Interface"));
    assert_eq!(port.dot_shape.as_deref(), Some("trapezium"));
}

// B:se_register_verify_kinds — verify unit "behavior allows unit, integration, property, load, e2e, mutation"
#[test]
fn test_software_behavior_verify_kinds() {
    let manifest = load_software_manifest();
    let behavior = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "behavior")
        .unwrap();
    assert_eq!(
        behavior.allowed_verify_kinds,
        vec!["unit", "integration", "property", "load", "e2e", "mutation"]
    );
}

// B:se_register_verify_kinds — verify unit "invariant allows property, unit, mutation, integration"
#[test]
fn test_software_invariant_verify_kinds() {
    let manifest = load_software_manifest();
    let invariant = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "invariant")
        .unwrap();
    assert_eq!(
        invariant.allowed_verify_kinds,
        vec!["property", "unit", "mutation", "integration"]
    );
}

// B:se_register_verify_kinds — verify unit "event allows integration, unit"
#[test]
fn test_software_event_verify_kinds() {
    let manifest = load_software_manifest();
    let event = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "event")
        .unwrap();
    assert_eq!(event.allowed_verify_kinds, vec!["integration", "unit"]);
}

// B:se_register_verify_kinds — verify unit "type allows property, unit, mutation"
#[test]
fn test_software_type_verify_kinds() {
    let manifest = load_software_manifest();
    let type_kind = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "type")
        .unwrap();
    assert_eq!(
        type_kind.allowed_verify_kinds,
        vec!["property", "unit", "mutation"]
    );
}

// B:se_register_verify_kinds — verify unit "port allows property, unit"
#[test]
fn test_software_port_verify_kinds() {
    let manifest = load_software_manifest();
    let port = manifest
        .entity_kinds
        .iter()
        .find(|k| k.keyword == "port")
        .unwrap();
    assert_eq!(port.allowed_verify_kinds, vec!["property", "unit"]);
}

// B:se_register_edge_types — verify unit "Implements edge has sourceKind=behavior and targetKind=feature"
#[test]
fn test_software_implements_edge() {
    let manifest = load_software_manifest();
    let edge = manifest
        .edge_types
        .iter()
        .find(|e| e.label == "Implements")
        .unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("behavior"));
    assert_eq!(edge.target_kind.as_deref(), Some("feature"));
}

// B:se_register_edge_types — verify unit "Produces edge has sourceKind=behavior and targetKind=event"
#[test]
fn test_software_produces_edge() {
    let manifest = load_software_manifest();
    let edge = manifest
        .edge_types
        .iter()
        .find(|e| e.label == "Produces")
        .unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("behavior"));
    assert_eq!(edge.target_kind.as_deref(), Some("event"));
}

// B:se_register_edge_types — verify unit "Enforces edge has sourceKind=behavior and targetKind=invariant"
#[test]
fn test_software_enforces_edge() {
    let manifest = load_software_manifest();
    let edge = manifest
        .edge_types
        .iter()
        .find(|e| e.label == "Enforces")
        .unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("behavior"));
    assert_eq!(edge.target_kind.as_deref(), Some("invariant"));
}

// B:se_register_edge_types — verify unit "ExtendsType edge has sourceKind=type and targetKind=type"
#[test]
fn test_software_extends_type_edge() {
    let manifest = load_software_manifest();
    let edge = manifest
        .edge_types
        .iter()
        .find(|e| e.label == "ExtendsType")
        .unwrap();
    assert_eq!(edge.source_kind.as_deref(), Some("type"));
    assert_eq!(edge.target_kind.as_deref(), Some("type"));
}

// Populates registries with all 5 kinds, fields, and edge types
#[test]
fn test_software_manifest_populates_registries() {
    let manifest = load_software_manifest();
    let (kind_reg, field_reg, edge_reg, diags) = populate_registries(&[manifest]);
    // Entity enhancements target module/milestone from @specforge/product which isn't
    // loaded here — graceful degradation produces I004 info diagnostics.
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == specforge_common::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "expected no error diagnostics, got: {:?}",
        errors
    );
    let infos: Vec<_> = diags
        .iter()
        .filter(|d| d.code == "I004")
        .collect();
    assert_eq!(
        infos.len(),
        2,
        "expected 2 I004 for module/milestone enhancements, got: {:?}",
        infos
    );

    // All 5 kinds registered
    assert_eq!(kind_reg.len(), 5);
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
    assert!(kind_reg.contains("event"));
    assert!(kind_reg.contains("type"));
    assert!(kind_reg.contains("port"));

    // Source extension
    assert_eq!(
        kind_reg.get("behavior").unwrap().source_extension,
        "@specforge/software"
    );

    // All 5 testable
    for keyword in &["behavior", "invariant", "event", "type", "port"] {
        assert!(
            kind_reg.get(keyword).unwrap().testable,
            "{} should be testable",
            keyword
        );
        assert!(
            kind_reg.get(keyword).unwrap().supports_verify,
            "{} should support verify",
            keyword
        );
    }

    // Behavior fields
    assert!(field_reg.contains("behavior", "contract"));
    assert!(field_reg.contains("behavior", "invariants"));
    assert!(field_reg.contains("behavior", "types"));
    assert!(field_reg.contains("behavior", "ports"));
    assert!(field_reg.contains("behavior", "produces"));
    assert!(field_reg.contains("behavior", "category"));
    assert!(field_reg.contains("behavior", "tests"));
    assert!(field_reg.contains("behavior", "gherkin"));
    assert!(field_reg.contains("behavior", "features"));

    // Invariant fields
    assert!(field_reg.contains("invariant", "guarantee"));
    assert!(field_reg.contains("invariant", "risk"));

    // Event fields
    assert!(field_reg.contains("event", "trigger"));
    assert!(field_reg.contains("event", "channel"));
    assert!(field_reg.contains("event", "payload"));

    // Type fields
    assert!(field_reg.contains("type", "kind"));
    assert!(field_reg.contains("type", "fieldType"));

    // Port fields
    assert!(field_reg.contains("port", "direction"));
    assert!(field_reg.contains("port", "methods"));

    // Edge types
    assert!(edge_reg.contains("References"));
    assert!(edge_reg.contains("Implements"));
    assert!(edge_reg.contains("Produces"));
    assert!(edge_reg.contains("Consumes"));
    assert!(edge_reg.contains("UsesType"));
    assert!(edge_reg.contains("UsesPort"));
    assert!(edge_reg.contains("Enforces"));
    assert!(edge_reg.contains("ExtendsType"));
    assert!(edge_reg.contains("TestedBy"));
    assert!(edge_reg.contains("ExternalRef"));
    assert!(edge_reg.contains("MilestoneBehavior"));
}

// Validation rules are parseable
#[test]
fn test_software_manifest_validation_rules_parse() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.validation_rules.len(), 17);

    let rules: Vec<_> = manifest
        .validation_rules
        .iter()
        .map(|r| specforge_registry::validation_engine::parse_rule_pattern(r, "@specforge/software"))
        .collect();

    let errors: Vec<_> = rules.iter().filter(|r| r.is_err()).collect();
    assert!(
        errors.is_empty(),
        "all validation rules should parse, errors: {:?}",
        errors
    );
}

// Verify kinds are the full software set
#[test]
fn test_software_manifest_verify_kinds() {
    let manifest = load_software_manifest();
    assert_eq!(
        manifest.verify_kinds,
        vec!["unit", "integration", "property", "load", "e2e", "mutation"]
    );
}
