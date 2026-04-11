// Slice 12: Sandbox Integration Tests
//
// Tests B:enforce_wasm_sandbox end-to-end through the public API.

use specforge_registry::SandboxPolicy;
use specforge_wasm::{
    configure_sandbox_policy, default_sandbox_policy, is_domain_allowed,
    is_output_extension_allowed, is_path_allowed, validate_total_memory,
};

fn default_test_manifest() -> specforge_registry::ManifestV2 {
    specforge_registry::ManifestV2 {
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

fn manifest_with_sandbox(name: &str, policy: SandboxPolicy) -> specforge_registry::ManifestV2 {
    let mut m = default_test_manifest();
    m.name = name.to_string();
    m.sandbox_policy = Some(policy);
    m
}

// ============================================================================
// B:enforce_wasm_sandbox — end-to-end integration tests
// ============================================================================

// B:enforce_wasm_sandbox — verify integration "path within allowed list → permitted"
#[test]
fn test_sandbox_path_within_allowed_list_permitted() {
    let policy = SandboxPolicy {
        file_system_access: Some(true),
        allowed_paths: vec!["/project/spec".into(), "/tmp".into()],
        ..Default::default()
    };

    assert!(is_path_allowed("/project/spec/file.spec", &policy));
    assert!(is_path_allowed("/tmp/output.json", &policy));
}

// B:enforce_wasm_sandbox — verify integration "path outside allowed list → denied"
#[test]
fn test_sandbox_path_outside_allowed_list_denied() {
    let policy = SandboxPolicy {
        file_system_access: Some(true),
        allowed_paths: vec!["/project/spec".into()],
        ..Default::default()
    };

    assert!(!is_path_allowed("/etc/passwd", &policy));
    assert!(!is_path_allowed("/home/user/.ssh/id_rsa", &policy));
}

// B:enforce_wasm_sandbox — verify integration "memory exceeding limit → denied"
#[test]
fn test_sandbox_memory_exceeding_ceiling_warns() {
    let p1 = default_sandbox_policy(); // 64MB
    let p2 = SandboxPolicy {
        max_memory_mb: Some(200),
        ..Default::default()
    };

    // Total: 264MB > 256MB ceiling
    let diags = validate_total_memory(&[("ext1", &p1), ("ext2", &p2)], 256);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W028");
    assert!(diags[0].message.contains("256MB ceiling"));

    // Under ceiling → no warning
    let small = SandboxPolicy {
        max_memory_mb: Some(32),
        ..Default::default()
    };
    let diags = validate_total_memory(&[("ext1", &p1), ("ext2", &small)], 256);
    assert!(diags.is_empty());
}

// B:enforce_wasm_sandbox — verify contract "requires policy + request, ensures enforcement"
#[test]
fn test_sandbox_enforcement_contract() {
    let policy = SandboxPolicy {
        max_memory_mb: Some(64),
        max_execution_ms: Some(30_000),
        file_system_access: Some(true),
        allowed_paths: vec!["/project".into()],
        network_access: Some(true),
        allowed_domains: vec!["api.example.com".into()],
        allowed_output_extensions: vec![".json".into(), ".csv".into()],
    };

    // ensures: filesystem enforced
    assert!(is_path_allowed("/project/file.spec", &policy));
    assert!(!is_path_allowed("/etc/secret", &policy));

    // ensures: network enforced
    assert!(is_domain_allowed("api.example.com", &policy));
    assert!(!is_domain_allowed("evil.com", &policy));

    // ensures: output extensions enforced
    assert!(is_output_extension_allowed(".json", &policy));
    assert!(is_output_extension_allowed(".csv", &policy));
    assert!(!is_output_extension_allowed(".rs", &policy));
    assert!(!is_output_extension_allowed(".exe", &policy));
}

// B:enforce_wasm_sandbox — verify integration "no filesystem access → all paths denied"
#[test]
fn test_sandbox_no_filesystem_access_denies_all() {
    let policy = SandboxPolicy {
        file_system_access: Some(false),
        allowed_paths: vec!["/project".into()],
        ..Default::default()
    };

    assert!(!is_path_allowed("/project/file.spec", &policy));
    assert!(!is_path_allowed("/any/path", &policy));
}

// B:enforce_wasm_sandbox — verify integration "no network access → all domains denied"
#[test]
fn test_sandbox_no_network_access_denies_all() {
    let policy = SandboxPolicy {
        network_access: Some(false),
        allowed_domains: vec!["api.github.com".into()],
        ..Default::default()
    };

    assert!(!is_domain_allowed("api.github.com", &policy));
    assert!(!is_domain_allowed("any.domain.com", &policy));
}

// B:configure_sandbox_policy — verify integration "three-layer merge: defaults < manifest < config"
#[test]
fn test_sandbox_three_layer_merge() {
    let manifest = manifest_with_sandbox(
        "@test/ext",
        SandboxPolicy {
            max_memory_mb: Some(48),
            max_execution_ms: Some(15_000),
            ..Default::default()
        },
    );
    let config_override = SandboxPolicy {
        max_memory_mb: Some(24),
        ..Default::default()
    };

    let (policy, diags) = configure_sandbox_policy(&manifest, Some(&config_override));
    assert!(diags.is_empty());
    // min(min(64, 48), 24) = 24
    assert_eq!(policy.max_memory_mb, Some(24));
    // min(30000, 15000) = 15000
    assert_eq!(policy.max_execution_ms, Some(15_000));
}

// B:configure_sandbox_policy — verify integration "code file extensions rejected with E030"
#[test]
fn test_sandbox_code_extensions_rejected() {
    let manifest = manifest_with_sandbox(
        "@bad/ext",
        SandboxPolicy {
            allowed_output_extensions: vec![".json".into(), ".rs".into()],
            ..Default::default()
        },
    );

    let (policy, diags) = configure_sandbox_policy(&manifest, None);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "E030");
    assert!(diags[0].message.contains(".rs"));
    // Code extension filtered out
    assert!(!policy.allowed_output_extensions.contains(&".rs".to_string()));
    assert!(policy.allowed_output_extensions.contains(&".json".to_string()));
}
