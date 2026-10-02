// Slice 13: Provider System Types & Registration Tests
//
// Tests behaviors through the public API:
// - B:load_provider_configurations
// - B:register_provider_schemes

use specforge_registry::{
    ExtensionContributions, ManifestV2, ProviderConfig, load_provider_configurations,
    register_provider_schemes,
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

/// A `providers` entry: `alias` serving `scheme`, implemented by `extension`.
fn provider(alias: &str, scheme: &str, extension: &str) -> ProviderConfig {
    ProviderConfig {
        scheme: scheme.to_string(),
        alias: alias.to_string(),
        extension: extension.to_string(),
        settings: Default::default(),
    }
}

// B:load_provider_configurations — the array of {scheme, alias, extension,
// settings} (ADR 0004 D3-c), in declaration order.
#[test]
fn test_load_providers_valid_array() {
    let config = serde_json::json!({
        "providers": [
            {
                "scheme": "gh",
                "alias": "github",
                "extension": "@acme/gh",
                "settings": {"baseUrl": "https://api.github.com", "apiKeyEnv": "GITHUB_TOKEN"}
            },
            {"scheme": "jira", "alias": "jira", "extension": "@acme/jira"}
        ]
    });

    let (providers, diags) = load_provider_configurations(&config);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(
        providers,
        vec![
            ProviderConfig {
                settings: serde_json::json!({
                    "baseUrl": "https://api.github.com", "apiKeyEnv": "GITHUB_TOKEN"
                })
                .as_object()
                .unwrap()
                .clone(),
                ..provider("github", "gh", "@acme/gh")
            },
            provider("jira", "jira", "@acme/jira"),
        ]
    );
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

// An entry missing scheme, alias or extension is W118, and so is the old
// object-map shape.
#[test]
fn test_load_providers_invalid_entry_warns() {
    let config = serde_json::json!({
        "providers": [
            { "scheme": "gh", "extension": "@acme/gh" },
            { "alias": "jira", "extension": "@acme/jira" },
            { "scheme": "x", "alias": "x" },
            { "scheme": "y", "alias": "y", "extension": "@acme/y", "settings": 3 }
        ]
    });

    let (providers, diags) = load_provider_configurations(&config);
    assert!(providers.is_empty());
    assert_eq!(diags.len(), 4, "{diags:?}");
    assert!(diags.iter().all(|d| d.code == "W118"));
    assert!(diags[2].message.contains("extension"), "{:?}", diags[2]);

    let map = serde_json::json!({ "providers": { "gh": { "package": "@acme/gh" } } });
    let (providers, diags) = load_provider_configurations(&map);
    assert!(providers.is_empty());
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("must be an array"),
        "{:?}",
        diags[0]
    );
}

// B:load_provider_configurations — verify contract "requires config JSON, ensures provider configs"
#[test]
fn test_load_providers_contract() {
    // ensures: valid → parsed correctly
    let config = serde_json::json!({
        "providers": [{ "scheme": "gh", "alias": "gh", "extension": "@acme/gh" }]
    });
    let (providers, diags) = load_provider_configurations(&config);
    assert_eq!(providers.len(), 1);
    assert!(diags.is_empty());

    // ensures: empty config → empty result
    let (providers, diags) = load_provider_configurations(&serde_json::json!({}));
    assert!(providers.is_empty());
    assert!(diags.is_empty());

    // ensures: invalid entry → W118
    let config = serde_json::json!({ "providers": [{}] });
    let (_, diags) = load_provider_configurations(&config);
    assert!(diags.iter().any(|d| d.code == "W118"));
}

// ============================================================================
// B:register_provider_schemes — integration tests
// ============================================================================

// B:register_provider_schemes — verify integration "scheme registered from manifest"
#[test]
fn test_register_schemes_from_manifest() {
    let providers = vec![provider("github", "gh", "@specforge/github")];

    let manifests = vec![(
        "@specforge/github".to_string(),
        make_manifest("@specforge/github", true),
    )];

    let (registry, diags) = register_provider_schemes(&providers, &manifests);
    assert!(diags.is_empty());
    assert_eq!(registry.entries.len(), 1);
    assert_eq!(registry.entries[0].scheme, "gh");
    assert_eq!(registry.entries[0].provider_name, "github");
    assert_eq!(registry.entries[0].extension_name, "@specforge/github");
}

// B:register_provider_schemes — verify integration "duplicate scheme → E033"
#[test]
fn test_register_schemes_duplicate_produces_e057() {
    let providers = vec![
        provider("gh-a", "gh", "@ext/a"),
        provider("gh-b", "gh", "@ext/b"),
    ];

    let manifests = vec![
        ("@ext/a".to_string(), make_manifest("@ext/a", true)),
        ("@ext/b".to_string(), make_manifest("@ext/b", true)),
    ];

    let (_, diags) = register_provider_schemes(&providers, &manifests);
    assert!(
        diags.iter().any(|d| d.code == "E057"),
        "expected E057 for duplicate scheme, got: {:?}",
        diags
    );
}

// B:register_provider_schemes — verify integration "provider without matching manifest → W033"
#[test]
fn test_register_schemes_no_manifest_warns() {
    let providers = vec![provider("github", "gh", "@specforge/github")];

    // No manifests contribute providers
    let manifests: Vec<(String, ManifestV2)> = vec![];

    let (_, diags) = register_provider_schemes(&providers, &manifests);
    assert!(
        diags.iter().any(|d| d.code == "W118"),
        "expected W118, got: {:?}",
        diags
    );
}

// B:register_provider_schemes — verify contract "requires providers + manifests, ensures scheme registry"
#[test]
fn test_register_schemes_contract() {
    let providers = vec![provider("github", "gh", "@ext/gh")];
    let manifests = vec![("@ext/gh".to_string(), make_manifest("@ext/gh", true))];

    // ensures: registered correctly
    let (registry, diags) = register_provider_schemes(&providers, &manifests);
    assert!(diags.is_empty());
    assert!(registry.find_by_scheme("gh").is_some());

    // ensures: non-contributing extension not matched
    let manifests_no_provider =
        vec![("@ext/other".to_string(), make_manifest("@ext/other", false))];
    let (registry, _) = register_provider_schemes(&providers, &manifests_no_provider);
    assert!(registry.entries.is_empty());
}

// ============================================================================
// H11: Provider scheme isolation — each provider registered to its matching extension only
// ============================================================================

// H11 — verify "two extensions, two providers, each only registered to its owner"
#[test]
fn test_provider_scheme_isolation_each_registered_to_owner() {
    // Two extensions, each contributing providers
    let manifests = vec![
        (
            "@ext/github".to_string(),
            make_manifest("@ext/github", true),
        ),
        ("@ext/jira".to_string(), make_manifest("@ext/jira", true)),
    ];

    // Two provider configs, each with a distinct scheme matching one extension
    let providers = vec![
        provider("github", "gh", "@ext/github"),
        provider("jira", "jira", "@ext/jira"),
    ];

    let (registry, diags) = register_provider_schemes(&providers, &manifests);

    // No diagnostics — each provider names its extension
    assert!(
        diags.is_empty(),
        "expected no diagnostics for isolated providers, got: {:?}",
        diags
    );

    // Both schemes registered
    assert_eq!(
        registry.entries.len(),
        2,
        "both providers should be registered"
    );

    // "gh" scheme should map to @ext/github (contains "gh")
    let gh_entry = registry
        .find_by_scheme("gh")
        .expect("gh scheme should be registered");
    assert_eq!(
        gh_entry.extension_name, "@ext/github",
        "gh scheme should be registered to @ext/github, not cross-assigned"
    );

    // "jira" scheme should map to @ext/jira (contains "jira")
    let jira_entry = registry
        .find_by_scheme("jira")
        .expect("jira scheme should be registered");
    assert_eq!(
        jira_entry.extension_name, "@ext/jira",
        "jira scheme should be registered to @ext/jira, not cross-assigned"
    );
}
