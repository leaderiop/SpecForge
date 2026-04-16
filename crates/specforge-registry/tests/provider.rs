// Slice 13: Provider System Types & Registration Tests
//
// Tests behaviors through the public API:
// - B:load_provider_configurations
// - B:register_provider_schemes
// - B:validate_provider_refs
// - B:validate_ref_target_format
// - B:validate_provider_kinds
// - B:load_extension_manifests
// - B:register_extension_entity_types

use specforge_common::Severity;
use specforge_registry::{
    load_extension_manifests, load_provider_configurations, register_extension_entity_types,
    register_provider_schemes, validate_provider_ref, validate_ref_target_format,
    validate_provider_kinds, ExtensionContributions, ManifestV2, ProviderConfig,
    ProviderSchemeRegistry, SchemeRegistryEntry, KindRegistry,
};
use tempfile::TempDir;

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
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        surfaces: None,
    }
}

fn make_manifest(name: &str, providers: bool) -> ManifestV2 {
    ManifestV2 {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        wasm_path: "extension.wasm".to_string(),
        contributes: ExtensionContributions {
            providers,
            ..Default::default()
        },
        ..default_manifest()
    }
}

// ============================================================================
// B:load_provider_configurations — integration tests
// ============================================================================

// B:load_provider_configurations — verify integration "valid providers array parsed correctly"
#[test]
fn test_load_providers_valid_array() {
    let config = serde_json::json!({
        "providers": [
            {
                "alias": "github",
                "scheme": "gh",
                "baseUrl": "https://api.github.com",
                "apiKeyEnv": "GITHUB_TOKEN"
            },
            {
                "alias": "jira",
                "scheme": "jira",
                "baseUrl": "https://myorg.atlassian.net"
            }
        ]
    });

    let (providers, diags) = load_provider_configurations(&config);
    assert!(diags.is_empty());
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0].name, "github");
    assert_eq!(providers[0].scheme, "gh");
    assert_eq!(
        providers[0].base_url.as_deref(),
        Some("https://api.github.com")
    );
    assert_eq!(
        providers[0].api_key_env.as_deref(),
        Some("GITHUB_TOKEN")
    );
    assert_eq!(providers[1].name, "jira");
    assert_eq!(providers[1].scheme, "jira");
}

// B:load_provider_configurations — verify integration "missing providers key → empty vec"
#[test]
fn test_load_providers_missing_key() {
    let config = serde_json::json!({
        "name": "my-project",
        "version": "1.0.0"
    });

    let (providers, diags) = load_provider_configurations(&config);
    assert!(providers.is_empty());
    assert!(diags.is_empty());
}

// B:load_provider_configurations — verify integration "invalid provider entry → W032"
#[test]
fn test_load_providers_invalid_entry_warns() {
    let config = serde_json::json!({
        "providers": [
            { "scheme": "gh" },  // missing alias/name
            { "alias": "jira" }  // missing scheme
        ]
    });

    let (providers, diags) = load_provider_configurations(&config);
    assert!(providers.is_empty());
    assert_eq!(diags.len(), 2);
    assert!(diags.iter().all(|d| d.code == "W032"));
}

// B:load_provider_configurations — verify contract "requires config JSON, ensures provider configs"
#[test]
fn test_load_providers_contract() {
    // ensures: valid → parsed correctly
    let config = serde_json::json!({
        "providers": [{ "alias": "gh", "scheme": "gh" }]
    });
    let (providers, diags) = load_provider_configurations(&config);
    assert_eq!(providers.len(), 1);
    assert!(diags.is_empty());

    // ensures: empty config → empty result
    let (providers, diags) = load_provider_configurations(&serde_json::json!({}));
    assert!(providers.is_empty());
    assert!(diags.is_empty());

    // ensures: invalid entry → W032
    let config = serde_json::json!({ "providers": [{}] });
    let (_, diags) = load_provider_configurations(&config);
    assert!(diags.iter().any(|d| d.code == "W032"));
}

// ============================================================================
// B:register_provider_schemes — integration tests
// ============================================================================

// B:register_provider_schemes — verify integration "scheme registered from manifest"
#[test]
fn test_register_schemes_from_manifest() {
    let providers = vec![ProviderConfig {
        name: "github".to_string(),
        scheme: "gh".to_string(),
        base_url: None,
        api_key_env: None,
    }];

    let manifests = vec![("@specforge/github".to_string(), make_manifest("@specforge/github", true))];

    let (registry, diags) = register_provider_schemes(&providers, &manifests);
    assert!(diags.is_empty());
    assert_eq!(registry.entries.len(), 1);
    assert_eq!(registry.entries[0].scheme, "gh");
    assert_eq!(registry.entries[0].provider_name, "github");
    assert_eq!(registry.entries[0].extension_name, "@specforge/github");
}

// B:register_provider_schemes — verify integration "duplicate scheme → E033"
#[test]
fn test_register_schemes_duplicate_produces_e033() {
    let providers = vec![
        ProviderConfig {
            name: "gh-a".to_string(),
            scheme: "gh".to_string(),
            base_url: None,
            api_key_env: None,
        },
        ProviderConfig {
            name: "gh-b".to_string(),
            scheme: "gh".to_string(),
            base_url: None,
            api_key_env: None,
        },
    ];

    let manifests = vec![
        ("@ext/a".to_string(), make_manifest("@ext/a", true)),
        ("@ext/b".to_string(), make_manifest("@ext/b", true)),
    ];

    let (_, diags) = register_provider_schemes(&providers, &manifests);
    assert!(
        diags.iter().any(|d| d.code == "E033"),
        "expected E033 for duplicate scheme, got: {:?}",
        diags
    );
}

// B:register_provider_schemes — verify integration "provider without matching manifest → W033"
#[test]
fn test_register_schemes_no_manifest_warns() {
    let providers = vec![ProviderConfig {
        name: "github".to_string(),
        scheme: "gh".to_string(),
        base_url: None,
        api_key_env: None,
    }];

    // No manifests contribute providers
    let manifests: Vec<(String, ManifestV2)> = vec![];

    let (_, diags) = register_provider_schemes(&providers, &manifests);
    assert!(
        diags.iter().any(|d| d.code == "W033"),
        "expected W033, got: {:?}",
        diags
    );
}

// B:register_provider_schemes — verify contract "requires providers + manifests, ensures scheme registry"
#[test]
fn test_register_schemes_contract() {
    let providers = vec![ProviderConfig {
        name: "github".to_string(),
        scheme: "gh".to_string(),
        base_url: None,
        api_key_env: None,
    }];
    let manifests = vec![("@ext/gh".to_string(), make_manifest("@ext/gh", true))];

    // ensures: registered correctly
    let (registry, diags) = register_provider_schemes(&providers, &manifests);
    assert!(diags.is_empty());
    assert!(registry.find_by_scheme("gh").is_some());

    // ensures: non-contributing extension not matched
    let manifests_no_provider = vec![("@ext/other".to_string(), make_manifest("@ext/other", false))];
    let (registry, _) = register_provider_schemes(&providers, &manifests_no_provider);
    assert!(registry.entries.is_empty());
}

// ============================================================================
// B:validate_provider_refs — integration tests
// ============================================================================

// B:validate_provider_refs — verify integration "known scheme → no diagnostics"
#[test]
fn test_validate_provider_ref_known_scheme() {
    let registry = ProviderSchemeRegistry {
        entries: vec![SchemeRegistryEntry {
            scheme: "gh".to_string(),
            provider_name: "github".to_string(),
            extension_name: "@ext/github".to_string(),
        }],
    };

    let diags = validate_provider_ref("gh", "42", &registry);
    assert!(diags.is_empty());
}

// B:validate_provider_refs — verify integration "unknown scheme → E034"
#[test]
fn test_validate_provider_ref_unknown_scheme() {
    let registry = ProviderSchemeRegistry {
        entries: vec![],
    };

    let diags = validate_provider_ref("unknown", "42", &registry);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E034");
    assert!(diags[0].message.contains("unknown"));
}

// B:validate_provider_refs — verify contract "requires ref + registry, ensures validation"
#[test]
fn test_validate_provider_ref_contract() {
    let registry = ProviderSchemeRegistry {
        entries: vec![SchemeRegistryEntry {
            scheme: "gh".to_string(),
            provider_name: "github".to_string(),
            extension_name: "@ext/github".to_string(),
        }],
    };

    // ensures: known → empty
    assert!(validate_provider_ref("gh", "issue/42", &registry).is_empty());

    // ensures: unknown → E034
    let diags = validate_provider_ref("jira", "PROJ-123", &registry);
    assert_eq!(diags[0].code, "E034");
    assert_eq!(diags[0].severity, Severity::Error);
}

// ============================================================================
// B:validate_ref_target_format — integration tests
// ============================================================================

// B:validate_ref_target_format — verify integration "valid identifier → no diagnostics"
#[test]
fn test_validate_ref_target_valid() {
    assert!(validate_ref_target_format("42").is_empty());
    assert!(validate_ref_target_format("PROJ-123").is_empty());
    assert!(validate_ref_target_format("my/resource/path").is_empty());
}

// B:validate_ref_target_format — verify integration "empty → W034"
#[test]
fn test_validate_ref_target_empty() {
    let diags = validate_ref_target_format("");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W034");
}

// B:validate_ref_target_format — verify contract "requires target string, ensures format check"
#[test]
fn test_validate_ref_target_contract() {
    // ensures: valid → empty
    assert!(validate_ref_target_format("valid-target").is_empty());

    // ensures: empty → W034
    let diags = validate_ref_target_format("");
    assert_eq!(diags[0].code, "W034");
    assert_eq!(diags[0].severity, Severity::Warning);
}

// ============================================================================
// B:validate_provider_kinds — integration tests
// ============================================================================

// B:validate_provider_kinds — verify integration "provider with known kinds → passes"
#[test]
fn test_validate_provider_kinds_passes() {
    let providers = vec![ProviderConfig {
        name: "github".to_string(),
        scheme: "gh".to_string(),
        base_url: None,
        api_key_env: None,
    }];
    let kind_reg = KindRegistry::new();

    let diags = validate_provider_kinds(&providers, &kind_reg);
    assert!(diags.is_empty());
}

// B:validate_provider_kinds — verify integration "empty providers → no diagnostics"
#[test]
fn test_validate_provider_kinds_empty() {
    let kind_reg = KindRegistry::new();
    let diags = validate_provider_kinds(&[], &kind_reg);
    assert!(diags.is_empty());
}

// B:validate_provider_kinds — verify contract "requires providers + kind registry, ensures validation"
#[test]
fn test_validate_provider_kinds_contract() {
    let providers = vec![ProviderConfig {
        name: "test".to_string(),
        scheme: "test".to_string(),
        base_url: None,
        api_key_env: None,
    }];
    let kind_reg = KindRegistry::new();

    let diags = validate_provider_kinds(&providers, &kind_reg);
    assert!(diags.is_empty());
}

// ============================================================================
// B:load_extension_manifests — integration tests
// ============================================================================

// B:load_extension_manifests — verify integration "directory with valid manifests → all loaded"
#[test]
fn test_load_manifests_valid_directory() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("software.json"),
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm"
        }"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("product.json"),
        r#"{
            "name": "@specforge/product",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "product.wasm"
        }"#,
    )
    .unwrap();

    let (manifests, diags) = load_extension_manifests(dir.path());
    assert!(diags.is_empty(), "expected no diagnostics, got: {:?}", diags);
    assert_eq!(manifests.len(), 2);
}

// B:load_extension_manifests — verify integration "invalid manifest → diagnostic, others still loaded"
#[test]
fn test_load_manifests_invalid_produces_diagnostic() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("good.json"),
        r#"{
            "name": "@test/good",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "good.wasm"
        }"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("bad.json"), "not valid json {{{").unwrap();

    let (manifests, diags) = load_extension_manifests(dir.path());
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].name, "@test/good");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E030");
}

// B:load_extension_manifests — verify integration "empty directory → empty result"
#[test]
fn test_load_manifests_empty_directory() {
    let dir = TempDir::new().unwrap();
    let (manifests, diags) = load_extension_manifests(dir.path());
    assert!(manifests.is_empty());
    assert!(diags.is_empty());
}

// B:load_extension_manifests — verify contract "requires path, ensures manifests + diagnostics"
#[test]
fn test_load_manifests_contract() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("ext.json"),
        r#"{"name":"@test/ext","version":"1.0.0","manifestVersion":2,"wasmPath":"ext.wasm"}"#,
    )
    .unwrap();

    // ensures: valid manifests loaded
    let (manifests, diags) = load_extension_manifests(dir.path());
    assert_eq!(manifests.len(), 1);
    assert!(diags.is_empty());

    // ensures: nonexistent directory → empty
    let (manifests, diags) = load_extension_manifests(std::path::Path::new("/no/such/dir"));
    assert!(manifests.is_empty());
    assert!(diags.is_empty());
}

// ============================================================================
// B:register_extension_entity_types — integration tests
// ============================================================================

// B:register_extension_entity_types — verify integration "manifests with entity kinds → KindRegistry populated"
#[test]
fn test_register_entity_types_populates_kind_registry() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                { "name": "Behavior", "keyword": "behavior", "testable": true },
                { "name": "Invariant", "keyword": "invariant", "testable": true }
            ]
        }"#,
    )
    .unwrap();

    let (kind_reg, _, _, diags) = register_extension_entity_types(&[manifest]);
    assert!(diags.is_empty());
    assert!(kind_reg.contains("behavior"));
    assert!(kind_reg.contains("invariant"));
}

// B:register_extension_entity_types — verify integration "manifests with fields → FieldRegistry populated"
#[test]
fn test_register_entity_types_populates_field_registry() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                {
                    "name": "Behavior",
                    "keyword": "behavior",
                    "fields": [
                        { "name": "contract", "fieldType": "block" }
                    ]
                }
            ]
        }"#,
    )
    .unwrap();

    let (_, field_reg, _, diags) = register_extension_entity_types(&[manifest]);
    assert!(diags.is_empty());
    assert!(field_reg.contains("behavior", "contract"));
}

// B:register_extension_entity_types — verify integration "manifests with edges → EdgeRegistry populated"
#[test]
fn test_register_entity_types_populates_edge_registry() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@specforge/software",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "software.wasm",
            "entityKinds": [
                { "name": "Behavior", "keyword": "behavior" },
                { "name": "Invariant", "keyword": "invariant" }
            ],
            "edgeTypes": [
                { "label": "enforces", "sourceKind": "behavior", "targetKind": "invariant" }
            ]
        }"#,
    )
    .unwrap();

    let (_, _, edge_reg, diags) = register_extension_entity_types(&[manifest]);
    assert!(diags.is_empty());
    assert!(edge_reg.contains("enforces"));
}

// B:register_extension_entity_types — verify contract "requires manifests, ensures registries populated"
#[test]
fn test_register_entity_types_contract() {
    let manifest: ManifestV2 = serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "ext.wasm",
            "entityKinds": [
                {
                    "name": "Task",
                    "keyword": "task",
                    "testable": true,
                    "fields": [
                        { "name": "priority", "fieldType": "string" }
                    ]
                }
            ],
            "edgeTypes": [
                { "label": "depends_on", "sourceKind": "task", "targetKind": "task" }
            ]
        }"#,
    )
    .unwrap();

    let (kind_reg, field_reg, edge_reg, diags) = register_extension_entity_types(&[manifest]);
    assert!(diags.is_empty());
    // ensures: kinds populated
    assert!(kind_reg.contains("task"));
    // ensures: fields populated
    assert!(field_reg.contains("task", "priority"));
    // ensures: edges populated
    assert!(edge_reg.contains("depends_on"));
}

// ============================================================================
// H11: Provider scheme isolation — each provider registered to its matching extension only
// ============================================================================

// H11 — verify "two extensions, two providers, each only registered to its owner"
#[test]
fn test_provider_scheme_isolation_each_registered_to_owner() {
    // Two extensions, each contributing providers
    let manifests = vec![
        ("@ext/github".to_string(), make_manifest("@ext/github", true)),
        ("@ext/jira".to_string(), make_manifest("@ext/jira", true)),
    ];

    // Two provider configs, each with a distinct scheme matching one extension
    let providers = vec![
        ProviderConfig {
            name: "github".to_string(),
            scheme: "gh".to_string(),
            base_url: None,
            api_key_env: None,
        },
        ProviderConfig {
            name: "jira".to_string(),
            scheme: "jira".to_string(),
            base_url: None,
            api_key_env: None,
        },
    ];

    let (registry, diags) = register_provider_schemes(&providers, &manifests);

    // No diagnostics — each provider maps to a different extension via name/scheme matching
    assert!(
        diags.is_empty(),
        "expected no diagnostics for isolated providers, got: {:?}",
        diags
    );

    // Both schemes registered
    assert_eq!(registry.entries.len(), 2, "both providers should be registered");

    // "gh" scheme should map to @ext/github (contains "gh")
    let gh_entry = registry.find_by_scheme("gh").expect("gh scheme should be registered");
    assert_eq!(
        gh_entry.extension_name, "@ext/github",
        "gh scheme should be registered to @ext/github, not cross-assigned"
    );

    // "jira" scheme should map to @ext/jira (contains "jira")
    let jira_entry = registry.find_by_scheme("jira").expect("jira scheme should be registered");
    assert_eq!(
        jira_entry.extension_name, "@ext/jira",
        "jira scheme should be registered to @ext/jira, not cross-assigned"
    );
}
