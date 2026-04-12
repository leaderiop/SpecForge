use specforge_registry::{
    populate_registries, validate_manifest, validate_manifest_consistency, ManifestV2,
};

fn load_product_manifest() -> ManifestV2 {
    let json = include_str!("../../../extensions/product/manifest.json");
    serde_json::from_str(json).expect("product manifest.json should deserialize")
}

#[test]
fn test_product_manifest_deserializes() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.name, "@specforge/product");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.manifest_version, 2);
}

#[test]
fn test_product_manifest_has_9_entity_kinds() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.entity_kinds.len(), 9);
    let keywords: Vec<&str> = manifest.entity_kinds.iter().map(|k| k.keyword.as_str()).collect();
    assert!(keywords.contains(&"feature"));
    assert!(keywords.contains(&"journey"));
    assert!(keywords.contains(&"deliverable"));
    assert!(keywords.contains(&"milestone"));
    assert!(keywords.contains(&"module"));
    assert!(keywords.contains(&"term"));
    assert!(keywords.contains(&"persona"));
    assert!(keywords.contains(&"channel"));
    assert!(keywords.contains(&"release"));
}

#[test]
fn test_product_manifest_has_20_edge_types() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.edge_types.len(), 20);
    let labels: Vec<&str> = manifest.edge_types.iter().map(|e| e.label.as_str()).collect();
    assert!(labels.contains(&"FeatureDependsOn"));
    assert!(labels.contains(&"FeatureRelatesTo"));
    assert!(labels.contains(&"JourneyExercisesFeature"));
    assert!(labels.contains(&"JourneyTargetsPersona"));
    assert!(labels.contains(&"JourneyUsesChannel"));
    assert!(labels.contains(&"DeliverableSupportsJourney"));
    assert!(labels.contains(&"DeliverableContainsModule"));
    assert!(labels.contains(&"DeliverableTrackedByMilestone"));
    assert!(labels.contains(&"DeliverableDependsOn"));
    assert!(labels.contains(&"MilestoneDeliversFeature"));
    assert!(labels.contains(&"MilestoneScopesModule"));
    assert!(labels.contains(&"MilestoneDependsOn"));
    assert!(labels.contains(&"ModuleContainsFeature"));
    assert!(labels.contains(&"ModuleDependsOn"));
    assert!(labels.contains(&"TermReferencesRelatedTerm"));
    assert!(labels.contains(&"TermBelongsToModule"));
    assert!(labels.contains(&"ReleaseIncludesDeliverable"));
    assert!(labels.contains(&"ReleaseCompletesMilestone"));
    assert!(labels.contains(&"ReleaseDependsOn"));
    assert!(labels.contains(&"PersonaPrioritizesFeature"));
}

#[test]
fn test_product_manifest_passes_schema_validation() {
    let manifest = load_product_manifest();
    let diags = validate_manifest(&manifest);
    assert!(diags.is_empty(), "expected no schema validation errors, got: {:?}", diags);
}

#[test]
fn test_product_manifest_passes_consistency_validation() {
    let manifest = load_product_manifest();
    let diags = validate_manifest_consistency(&manifest);
    assert!(diags.is_empty(), "expected no consistency warnings, got: {:?}", diags);
}

#[test]
fn test_product_manifest_populates_registries() {
    let manifest = load_product_manifest();
    let (kind_reg, field_reg, edge_reg, diags) = populate_registries(&[manifest]);
    assert!(diags.is_empty(), "expected no population diagnostics, got: {:?}", diags);

    // All 9 kinds registered
    assert_eq!(kind_reg.len(), 9);
    assert!(kind_reg.contains("feature"));
    assert!(kind_reg.contains("journey"));
    assert!(kind_reg.contains("deliverable"));
    assert!(kind_reg.contains("milestone"));
    assert!(kind_reg.contains("module"));
    assert!(kind_reg.contains("term"));
    assert!(kind_reg.contains("persona"));
    assert!(kind_reg.contains("channel"));
    assert!(kind_reg.contains("release"));

    // Verify source extension
    assert_eq!(kind_reg.get("feature").unwrap().source_extension, "@specforge/product");

    // Feature has supportsVerify but not testable
    let feature = kind_reg.get("feature").unwrap();
    assert!(!feature.testable);
    assert!(feature.supports_verify);

    // Shared 'tags' field on all 9 kinds
    for keyword in &["feature", "journey", "deliverable", "milestone", "module", "term", "persona", "channel", "release"] {
        assert!(field_reg.contains(keyword, "tags"), "expected 'tags' field on {}", keyword);
    }

    // Kind-specific fields
    assert!(field_reg.contains("feature", "status"));
    assert!(field_reg.contains("feature", "priority"));
    assert!(field_reg.contains("feature", "depends_on"));
    assert!(field_reg.contains("journey", "persona"));
    assert!(field_reg.contains("journey", "features"));
    assert!(field_reg.contains("deliverable", "artifact_type"));
    assert!(field_reg.contains("milestone", "exit_criteria"));
    assert!(field_reg.contains("module", "family"));
    assert!(field_reg.contains("term", "definition"));
    assert!(field_reg.contains("term", "module"));
    assert!(field_reg.contains("persona", "technical_level"));
    assert!(field_reg.contains("channel", "interaction_model"));
    assert!(field_reg.contains("release", "version"));

    // Term module field has correct edge and target_kind
    let term_module = field_reg.get("term", "module").unwrap();
    assert_eq!(term_module.edge.as_deref(), Some("TermBelongsToModule"));
    assert_eq!(term_module.target_kind.as_deref(), Some("module"));

    // Edge types registered including TermBelongsToModule
    assert!(edge_reg.contains("FeatureDependsOn"));
    assert!(edge_reg.contains("JourneyExercisesFeature"));
    assert!(edge_reg.contains("ModuleDependsOn"));
    assert!(edge_reg.contains("ReleaseIncludesDeliverable"));
    assert!(edge_reg.contains("TermBelongsToModule"));
    assert!(edge_reg.contains("FeatureRelatesTo"));
    assert!(edge_reg.contains("PersonaPrioritizesFeature"));

    // Persona key_features field
    let persona_features = field_reg.get("persona", "key_features").unwrap();
    assert_eq!(persona_features.edge.as_deref(), Some("PersonaPrioritizesFeature"));
    assert_eq!(persona_features.target_kind.as_deref(), Some("feature"));
}

#[test]
fn test_product_manifest_verify_kinds() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.verify_kinds, vec!["acceptance"]);
}

#[test]
fn test_product_manifest_validation_rules_count() {
    let manifest = load_product_manifest();
    // Should have validation rules for enum checks, orphan detection, and cycle detection
    assert!(manifest.validation_rules.len() >= 17, "expected at least 17 validation rules, got {}", manifest.validation_rules.len());
}

#[test]
fn test_product_manifest_validation_rules_parse() {
    let manifest = load_product_manifest();
    let rules: Vec<_> = manifest.validation_rules.iter().map(|r| {
        specforge_registry::validation_engine::parse_rule_pattern(r, "@specforge/product")
    }).collect();

    let errors: Vec<_> = rules.iter().filter(|r| r.is_err()).collect();
    assert!(errors.is_empty(), "all validation rules should parse, but got errors: {:?}", errors);
}
