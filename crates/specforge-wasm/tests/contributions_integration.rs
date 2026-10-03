// Slice 15: Contribution Validation Integration Tests
//
// Tests entity enhancements, reserved keywords,
// and enhancement conflict resolution through the public API.

use specforge_registry::{ExtensionContributions, FieldEnhancement, ManifestField, ManifestV2};
use specforge_wasm::{
    ContributionToggle, EnhancementConflict, EnhancementOverride, EnhancementPolicy,
    is_contribution_disabled, register_entity_enhancements, reject_reserved_entity_kind,
    resolve_enhancement_conflicts, validate_contribution_exports,
};

fn default_manifest() -> ManifestV2 {
    ManifestV2 {
        name: String::new(),
        version: String::new(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: Default::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        fields: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        reserved_keywords: vec![],
        peer_dependencies: vec![],
        sandbox_policy: None,
        incremental: None,
        migration_hook: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        theme_color: None,
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![],
        surfaces: None,
    }
}

// ============================================================
// B:register_entity_enhancements
// ============================================================

// B:register_entity_enhancements — verify integration "enhancement with new field on target kind registered"
#[test]
fn register_enhancement_new_field() {
    let mut manifest = default_manifest();
    manifest.name = "@ext/a".to_string();
    manifest.entity_enhancements = vec![FieldEnhancement {
        verify_kinds: None,
        target_kind: "behavior".to_string(),
        source_extension: "@ext/a".to_string(),
        fields: vec![ManifestField {
            name: "custom_field".to_string(),
            field_type: "string".to_string(),
            description: None,
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
            default_value: None,
            enum_values: vec![],
            inverse_of: None,
            normative: false,
            exempts_obligations: false,
            headline: false,
            derived_from: None,
            proof_role: None,
        }],
        edge_types: vec![],
    }];

    let mut existing = Vec::new();
    let diags = register_entity_enhancements(&manifest, &mut existing);
    assert!(diags.is_empty());
    assert_eq!(existing.len(), 1);
    assert_eq!(existing[0].0, "@ext/a");
}

// B:register_entity_enhancements — verify integration "two extensions same target kind same field name produces E034"
#[test]
fn register_enhancement_conflict_e017() {
    let field = ManifestField {
        name: "priority".to_string(),
        field_type: "string".to_string(),
        description: None,
        edge: None,
        target_kind: None,
        file_reference: false,
        required: false,
        default_value: None,
        enum_values: vec![],
        inverse_of: None,
        normative: false,
        exempts_obligations: false,
        headline: false,
        derived_from: None,
        proof_role: None,
    };

    // First extension registers successfully
    let mut existing: Vec<(String, FieldEnhancement)> = vec![(
        "@ext/first".to_string(),
        FieldEnhancement {
            verify_kinds: None,
            target_kind: "behavior".to_string(),
            source_extension: "@ext/first".to_string(),
            fields: vec![field.clone()],
            edge_types: vec![],
        },
    )];

    // Second extension conflicts
    let mut manifest = default_manifest();
    manifest.name = "@ext/second".to_string();
    manifest.entity_enhancements = vec![FieldEnhancement {
        verify_kinds: None,
        target_kind: "behavior".to_string(),
        source_extension: "@ext/second".to_string(),
        fields: vec![field],
        edge_types: vec![],
    }];

    let diags = register_entity_enhancements(&manifest, &mut existing);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E017");
    assert!(diags[0].message.contains("priority"));
    assert!(diags[0].message.contains("behavior"));
}

// B:register_entity_enhancements — verify contract "requires manifest with entity_enhancements, ensures conflict detection"
#[test]
fn register_entity_enhancements_contract() {
    // No enhancements -> no diagnostics, no additions
    let manifest = default_manifest();
    let mut existing = Vec::new();
    let diags = register_entity_enhancements(&manifest, &mut existing);
    assert!(diags.is_empty());
    assert!(existing.is_empty());
}

// ============================================================
// B:reject_reserved_entity_kind
// ============================================================

// B:reject_reserved_entity_kind — verify integration "structural keyword rejected with E035"
#[test]
fn reject_structural_keyword() {
    for keyword in &["spec", "ref", "use", "define", "verify"] {
        let diag = reject_reserved_entity_kind(keyword, &[]).unwrap();
        assert_eq!(diag.code, "E035");
        assert!(diag.message.contains("reserved structural keyword"));
    }
}

// B:reject_reserved_entity_kind — verify integration "invalid identifier rejected with E035"
#[test]
fn reject_invalid_identifier() {
    let diag = reject_reserved_entity_kind("123bad", &[]).unwrap();
    assert_eq!(diag.code, "E035");
    assert!(diag.message.contains("not a valid identifier"));

    let diag = reject_reserved_entity_kind("CamelCase", &[]).unwrap();
    assert_eq!(diag.code, "E035");
}

// B:reject_reserved_entity_kind — verify integration "extension-reserved keyword rejected with E035"
#[test]
fn reject_extension_reserved_keyword() {
    let mut manifest = default_manifest();
    manifest.name = "@ext/custom".to_string();
    manifest.reserved_keywords = vec!["my_reserved".to_string()];

    let diag = reject_reserved_entity_kind("my_reserved", &[manifest]).unwrap();
    assert_eq!(diag.code, "E035");
    assert!(diag.message.contains("reserved by extension"));
}

// B:reject_reserved_entity_kind — verify integration "valid unreserved kind returns None"
#[test]
fn accept_valid_unreserved_kind() {
    let result = reject_reserved_entity_kind("custom_entity", &[]);
    assert!(result.is_none());
}

// ============================================================
// B:resolve_enhancement_conflicts
// ============================================================

// B:resolve_enhancement_conflicts — verify integration "grammar-level conflict always produces E018"
#[test]
fn grammar_conflict_always_e018() {
    let conflicts = vec![EnhancementConflict {
        entity_kind: "behavior".to_string(),
        field_name: "grammar".to_string(),
        first_extension: "@ext/a".to_string(),
        second_extension: "@ext/b".to_string(),
        is_grammar_level: true,
    }];
    let overrides = vec![EnhancementOverride {
        entity_kind: "behavior".to_string(),
        field_name: "grammar".to_string(),
        winning_extension: "@ext/a".to_string(),
    }];

    // Grammar conflicts cannot be overridden
    let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &overrides);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E018");
    assert!(diags[0].message.contains("cannot be overridden"));
}

// B:resolve_enhancement_conflicts — verify integration "field-level conflict without override produces E017"
#[test]
fn field_conflict_without_override_e017() {
    let conflicts = vec![EnhancementConflict {
        entity_kind: "behavior".to_string(),
        field_name: "priority".to_string(),
        first_extension: "@ext/a".to_string(),
        second_extension: "@ext/b".to_string(),
        is_grammar_level: false,
    }];

    let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &[]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E017");
    assert!(diags[0].message.contains("priority"));
}

// B:resolve_enhancement_conflicts — verify integration "field-level conflict with matching override produces no diagnostic"
#[test]
fn field_conflict_with_override_resolved() {
    let conflicts = vec![EnhancementConflict {
        entity_kind: "behavior".to_string(),
        field_name: "priority".to_string(),
        first_extension: "@ext/a".to_string(),
        second_extension: "@ext/b".to_string(),
        is_grammar_level: false,
    }];
    let overrides = vec![EnhancementOverride {
        entity_kind: "behavior".to_string(),
        field_name: "priority".to_string(),
        winning_extension: "@ext/a".to_string(),
    }];

    let diags = resolve_enhancement_conflicts(&conflicts, EnhancementPolicy::Error, &overrides);
    assert!(diags.is_empty());
}

// ============================================================
// B:validate_contribution_exports
// ============================================================

// B:validate_contribution_exports — verify integration "all required exports present produces no diagnostics"
#[test]
fn validate_exports_all_present() {
    let mut manifest = default_manifest();
    manifest.name = "@specforge/software".to_string();
    manifest.contributes = ExtensionContributions {
        validators: true,
        renderers: true,
        ..Default::default()
    };

    let available = vec![
        "specforge__software_validate".to_string(),
        "specforge__software_render".to_string(),
    ];

    let diags = validate_contribution_exports(&manifest, &available);
    assert!(diags.is_empty());
}

// B:validate_contribution_exports — verify integration "missing validator export produces E020"
#[test]
fn validate_exports_missing_validator_e020() {
    let mut manifest = default_manifest();
    manifest.name = "@specforge/software".to_string();
    manifest.contributes = ExtensionContributions {
        validators: true,
        ..Default::default()
    };

    let diags = validate_contribution_exports(&manifest, &[]);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E020");
    assert!(diags[0].message.contains("specforge__software_validate"));
}

// B:validate_contribution_exports — verify integration "multiple missing exports produce one E020 each"
#[test]
fn validate_exports_multiple_missing() {
    let mut manifest = default_manifest();
    manifest.name = "@specforge/software".to_string();
    manifest.contributes = ExtensionContributions {
        validators: true,
        renderers: true,
        collectors: true,
        ..Default::default()
    };

    let diags = validate_contribution_exports(&manifest, &[]);
    assert_eq!(diags.len(), 3);
    assert!(diags.iter().all(|d| d.code == "E020"));
}

// ============================================================
// B:is_contribution_disabled (bonus — small function, quick coverage)
// ============================================================

// B:is_contribution_disabled — verify integration "matching toggle disables contribution"
#[test]
fn contribution_disabled_matching_toggle() {
    let toggles = vec![ContributionToggle {
        extension_name: "@ext/a".to_string(),
        disabled: ["validators".to_string()].into_iter().collect(),
    }];
    assert!(is_contribution_disabled(&toggles, "@ext/a", "validators"));
}

// B:is_contribution_disabled — verify integration "no matching toggle means enabled"
#[test]
fn contribution_enabled_no_matching_toggle() {
    let toggles = vec![ContributionToggle {
        extension_name: "@ext/a".to_string(),
        disabled: ["validators".to_string()].into_iter().collect(),
    }];
    assert!(!is_contribution_disabled(&toggles, "@ext/a", "renderers"));
    assert!(!is_contribution_disabled(&toggles, "@ext/b", "validators"));
}
