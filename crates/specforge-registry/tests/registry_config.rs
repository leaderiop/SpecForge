use specforge_common::Severity;
use specforge_registry::registry_config::{
    find_registry_for_specifier, parse_registries_from_config, RegistryConfig,
};

// B:configure_registries — verify unit "RegistryConfig deserializes from JSON"
#[test]
fn registry_config_deserializes_from_json() {
    let json = r#"{
        "alias": "main",
        "url": "https://registry.example.com",
        "scope_filter": "@specforge",
        "default_registry": true
    }"#;
    let config: RegistryConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.alias, "main");
    assert_eq!(config.url, "https://registry.example.com");
    assert_eq!(config.scope_filter, Some("@specforge".to_string()));
    assert!(config.default_registry);
}

// B:configure_registries — verify unit "RegistryConfig deserializes with defaults for optional fields"
#[test]
fn registry_config_deserializes_with_defaults() {
    let json = r#"{
        "alias": "minimal",
        "url": "https://minimal.example.com"
    }"#;
    let config: RegistryConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.alias, "minimal");
    assert_eq!(config.url, "https://minimal.example.com");
    assert_eq!(config.scope_filter, None);
    assert!(!config.default_registry);
}

// B:configure_registries — verify unit "parse_registries_from_config extracts registries array"
#[test]
fn parse_registries_extracts_array() {
    let json = r#"{
        "registries": [
            {
                "alias": "primary",
                "url": "https://primary.example.com",
                "default_registry": true
            },
            {
                "alias": "secondary",
                "url": "https://secondary.example.com"
            }
        ]
    }"#;
    let (registries, diagnostics) = parse_registries_from_config(json);
    assert_eq!(registries.len(), 2);
    assert_eq!(registries[0].alias, "primary");
    assert_eq!(registries[1].alias, "secondary");
    // No error diagnostics expected (there is a default)
    assert!(
        diagnostics.iter().all(|d| d.severity != Severity::Error),
        "unexpected error diagnostics: {diagnostics:?}"
    );
}

// B:configure_registries — verify unit "empty registries with no default produces I003 info diagnostic"
#[test]
fn empty_registries_no_default_produces_i003() {
    let json = r#"{ "registries": [] }"#;
    let (registries, diagnostics) = parse_registries_from_config(json);
    assert!(registries.is_empty());
    let i003 = diagnostics
        .iter()
        .find(|d| d.code == "I003")
        .expect("expected I003 diagnostic");
    assert_eq!(i003.severity, Severity::Info);
    assert!(i003.message.contains("No registries configured"));
}

// B:configure_registries — verify unit "registries present but no default_registry true produces I003"
#[test]
fn registries_without_default_produces_i003() {
    let json = r#"{
        "registries": [
            { "alias": "a", "url": "https://a.example.com" }
        ]
    }"#;
    let (registries, diagnostics) = parse_registries_from_config(json);
    assert_eq!(registries.len(), 1);
    let i003 = diagnostics
        .iter()
        .find(|d| d.code == "I003")
        .expect("expected I003 diagnostic");
    assert_eq!(i003.severity, Severity::Info);
}

// B:configure_registries — verify unit "find_registry_for_specifier routes @scope/ prefix to matching registry"
#[test]
fn find_registry_routes_scope_prefix() {
    let registries = vec![
        RegistryConfig {
            alias: "specforge".to_string(),
            url: "https://specforge.example.com".to_string(),
            scope_filter: Some("@specforge".to_string()),
            default_registry: false,
        },
        RegistryConfig {
            alias: "default".to_string(),
            url: "https://default.example.com".to_string(),
            scope_filter: None,
            default_registry: true,
        },
    ];

    let result = find_registry_for_specifier("@specforge/software", &registries);
    assert_eq!(result.unwrap().alias, "specforge");
}

// B:configure_registries — verify unit "no scope match falls back to default registry"
#[test]
fn find_registry_falls_back_to_default() {
    let registries = vec![
        RegistryConfig {
            alias: "specforge".to_string(),
            url: "https://specforge.example.com".to_string(),
            scope_filter: Some("@specforge".to_string()),
            default_registry: false,
        },
        RegistryConfig {
            alias: "default".to_string(),
            url: "https://default.example.com".to_string(),
            scope_filter: None,
            default_registry: true,
        },
    ];

    let result = find_registry_for_specifier("@other/extension", &registries);
    assert_eq!(result.unwrap().alias, "default");
}

// B:configure_registries — verify unit "no scope match and no default returns None"
#[test]
fn find_registry_no_match_no_default_returns_none() {
    let registries = vec![RegistryConfig {
        alias: "specforge".to_string(),
        url: "https://specforge.example.com".to_string(),
        scope_filter: Some("@specforge".to_string()),
        default_registry: false,
    }];

    let result = find_registry_for_specifier("@other/extension", &registries);
    assert!(result.is_none());
}

// B:configure_registries — verify unit "duplicate alias produces W-level diagnostic"
#[test]
fn duplicate_alias_produces_warning() {
    let json = r#"{
        "registries": [
            { "alias": "dup", "url": "https://first.example.com", "default_registry": true },
            { "alias": "dup", "url": "https://second.example.com" }
        ]
    }"#;
    let (registries, diagnostics) = parse_registries_from_config(json);
    assert_eq!(registries.len(), 2);
    let warning = diagnostics
        .iter()
        .find(|d| d.code == "W-REG-001")
        .expect("expected W-REG-001 diagnostic");
    assert_eq!(warning.severity, Severity::Warning);
    assert!(warning.message.contains("Duplicate registry alias"));
    assert!(warning.message.contains("dup"));
}

// B:configure_registries — verify unit "no hardcoded URLs - all from config"
#[test]
fn no_hardcoded_urls_all_from_config() {
    // Parse with custom URLs only - verify the returned registries use exactly those URLs
    let json = r#"{
        "registries": [
            {
                "alias": "custom",
                "url": "https://my-private-registry.example.com/v1",
                "default_registry": true
            }
        ]
    }"#;
    let (registries, _) = parse_registries_from_config(json);
    assert_eq!(registries.len(), 1);
    assert_eq!(
        registries[0].url,
        "https://my-private-registry.example.com/v1"
    );

    // Parse empty config - no URLs should appear
    let empty_json = r#"{ "registries": [] }"#;
    let (empty_registries, _) = parse_registries_from_config(empty_json);
    assert!(empty_registries.is_empty());

    // No missing "registries" key - still no hardcoded fallback
    let no_key_json = r#"{}"#;
    let (no_key_registries, _) = parse_registries_from_config(no_key_json);
    assert!(no_key_registries.is_empty());
}

// B:configure_registries — verify unit "invalid JSON produces error diagnostic"
#[test]
fn invalid_json_produces_error() {
    let bad_json = "not valid json";
    let (registries, diagnostics) = parse_registries_from_config(bad_json);
    assert!(registries.is_empty());
    let error = diagnostics
        .iter()
        .find(|d| d.code == "E-REG-001")
        .expect("expected E-REG-001 diagnostic");
    assert_eq!(error.severity, Severity::Error);
}

// B:configure_registries — verify unit "specifier without scope falls back to default"
#[test]
fn specifier_without_scope_falls_back_to_default() {
    let registries = vec![RegistryConfig {
        alias: "default".to_string(),
        url: "https://default.example.com".to_string(),
        scope_filter: None,
        default_registry: true,
    }];

    let result = find_registry_for_specifier("plain-extension", &registries);
    assert_eq!(result.unwrap().alias, "default");
}
