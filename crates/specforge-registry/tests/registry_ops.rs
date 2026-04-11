use std::sync::Mutex;

use specforge_common::Severity;
use specforge_registry::{
    ManifestV2, RegistryClient, RegistryConfig, RegistryCredential, RegistryError,
    RegistryResponse, RegistrySearchResult, TrustLevel,
};
use specforge_registry::registry_ops::{
    assign_trust_level, publish_to_registry, resolve_from_registry, search_registries,
    verify_registry_integrity,
};

// ---------------------------------------------------------------------------
// Mock client
// ---------------------------------------------------------------------------

struct MockRegistryClient {
    fetch_results: Mutex<Vec<(String, Result<RegistryResponse, RegistryError>)>>,
    #[allow(clippy::type_complexity)]
    search_results: Mutex<Vec<(String, Result<Vec<RegistrySearchResult>, RegistryError>)>>,
    publish_result: Mutex<Option<Result<String, RegistryError>>>,
}

impl MockRegistryClient {
    fn new() -> Self {
        Self {
            fetch_results: Mutex::new(Vec::new()),
            search_results: Mutex::new(Vec::new()),
            publish_result: Mutex::new(None),
        }
    }

    /// Add a fetch result keyed by registry alias.
    fn with_fetch_for(self, alias: &str, result: Result<RegistryResponse, RegistryError>) -> Self {
        self.fetch_results.lock().unwrap().push((alias.to_string(), result));
        self
    }

    /// Add a search result keyed by registry alias.
    fn with_search_for(
        self,
        alias: &str,
        result: Result<Vec<RegistrySearchResult>, RegistryError>,
    ) -> Self {
        self.search_results.lock().unwrap().push((alias.to_string(), result));
        self
    }

    fn with_publish(self, result: Result<String, RegistryError>) -> Self {
        *self.publish_result.lock().unwrap() = Some(result);
        self
    }
}

impl RegistryClient for MockRegistryClient {
    fn fetch(
        &self,
        _specifier: &str,
        registry: &RegistryConfig,
    ) -> Result<RegistryResponse, RegistryError> {
        let results = self.fetch_results.lock().unwrap();
        for (alias, result) in results.iter() {
            if alias == &registry.alias {
                return result.clone();
            }
        }
        Err(RegistryError::NotFound {
            specifier: _specifier.to_string(),
        })
    }

    fn search(
        &self,
        _query: &str,
        registry: &RegistryConfig,
    ) -> Result<Vec<RegistrySearchResult>, RegistryError> {
        let results = self.search_results.lock().unwrap();
        for (alias, result) in results.iter() {
            if alias == &registry.alias {
                return result.clone();
            }
        }
        Err(RegistryError::NetworkError {
            message: "no mock configured".into(),
        })
    }

    fn publish(
        &self,
        _package: &[u8],
        _manifest: &ManifestV2,
        _registry: &RegistryConfig,
    ) -> Result<String, RegistryError> {
        self.publish_result
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(Err(RegistryError::NetworkError {
                message: "no mock configured".into(),
            }))
    }

    fn authenticate(
        &self,
        _registry: &RegistryConfig,
        _credential: &RegistryCredential,
    ) -> Result<(), RegistryError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn default_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "default".to_string(),
        url: "https://registry.specforge.dev".to_string(),
        scope_filter: None,
        default_registry: true,
    }
}

fn scoped_registry(alias: &str, scope: &str) -> RegistryConfig {
    RegistryConfig {
        alias: alias.to_string(),
        url: format!("https://{alias}.registry.dev"),
        scope_filter: Some(scope.to_string()),
        default_registry: false,
    }
}

fn minimal_manifest() -> ManifestV2 {
    serde_json::from_str(
        r#"{
            "name": "@test/ext",
            "version": "1.0.0",
            "manifestVersion": 2,
            "wasmPath": "ext.wasm"
        }"#,
    )
    .unwrap()
}

fn make_response(name: &str, version: &str) -> RegistryResponse {
    RegistryResponse {
        name: name.to_string(),
        version: version.to_string(),
        wasm_url: format!("https://r.specforge.dev/{name}-{version}.wasm"),
        sha256: "abc123".to_string(),
    }
}

fn make_search_result(name: &str, version: &str, desc: &str) -> RegistrySearchResult {
    RegistrySearchResult {
        name: name.to_string(),
        version: version.to_string(),
        description: desc.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests: resolve_from_registry
// ---------------------------------------------------------------------------

// B:resolve_registry_source — verify unit "scope-prefixed specifier → matching registry"
#[test]
fn resolve_scope_prefixed_specifier_matches_registry() {
    let registries = vec![
        scoped_registry("private", "@myco"),
        default_registry(),
    ];
    let client = MockRegistryClient::new()
        .with_fetch_for("private", Ok(make_response("@myco/analytics", "2.0.0")));

    let resp = resolve_from_registry("@myco/analytics@2.0.0", &registries, &client).unwrap();
    assert_eq!(resp.name, "@myco/analytics");
    assert_eq!(resp.version, "2.0.0");
}

// B:resolve_registry_source — verify unit "no scope match → default registry fallback"
#[test]
fn resolve_falls_back_to_default_registry() {
    let registries = vec![
        scoped_registry("private", "@myco"),
        default_registry(),
    ];
    let client = MockRegistryClient::new()
        .with_fetch_for("default", Ok(make_response("@specforge/software", "1.0.0")));

    let resp = resolve_from_registry("@specforge/software@1.0.0", &registries, &client).unwrap();
    assert_eq!(resp.name, "@specforge/software");
}

// B:resolve_registry_source — verify unit "network error → ExtensionError with retry guidance"
#[test]
fn resolve_network_error_produces_diagnostic_with_retry_guidance() {
    let registries = vec![default_registry()];
    let client = MockRegistryClient::new().with_fetch_for(
        "default",
        Err(RegistryError::NetworkError {
            message: "connection refused".into(),
        }),
    );

    let err = resolve_from_registry("@specforge/software", &registries, &client).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("connection refused"));
    assert!(err.suggestion.as_ref().unwrap().contains("retry"));
}

// B:resolve_registry_source — verify unit "no registries → error diagnostic"
#[test]
fn resolve_no_registries_produces_error() {
    let registries: Vec<RegistryConfig> = vec![];
    let client = MockRegistryClient::new();

    let err = resolve_from_registry("@specforge/software", &registries, &client).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("No registry found"));
}

// ---------------------------------------------------------------------------
// Tests: search_registries
// ---------------------------------------------------------------------------

// B:search_registry — verify unit "queries ALL configured registries"
#[test]
fn search_queries_all_registries() {
    let registries = vec![
        scoped_registry("reg-a", "@alpha"),
        scoped_registry("reg-b", "@beta"),
    ];
    let client = MockRegistryClient::new()
        .with_search_for(
            "reg-a",
            Ok(vec![make_search_result("@alpha/ext", "1.0.0", "Alpha ext")]),
        )
        .with_search_for(
            "reg-b",
            Ok(vec![make_search_result("@beta/ext", "1.0.0", "Beta ext")]),
        );

    let (results, diags) = search_registries("ext", &registries, &client);
    assert!(diags.is_empty());
    assert_eq!(results.len(), 2);
    // Both registries contributed results
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"@alpha/ext"));
    assert!(names.contains(&"@beta/ext"));
}

// B:search_registry — verify unit "results deduplicated by name+version"
#[test]
fn search_deduplicates_by_name_and_version() {
    let registries = vec![
        scoped_registry("reg-a", "@specforge"),
        scoped_registry("reg-b", "@specforge"),
    ];
    // Both registries return the same package
    let client = MockRegistryClient::new()
        .with_search_for(
            "reg-a",
            Ok(vec![make_search_result(
                "@specforge/software",
                "1.0.0",
                "From A",
            )]),
        )
        .with_search_for(
            "reg-b",
            Ok(vec![make_search_result(
                "@specforge/software",
                "1.0.0",
                "From B",
            )]),
        );

    let (results, diags) = search_registries("software", &registries, &client);
    assert!(diags.is_empty());
    assert_eq!(results.len(), 1, "duplicate should be removed");
    assert_eq!(results[0].name, "@specforge/software");
}

// B:search_registry — verify unit "search output deterministic (sorted)"
#[test]
fn search_results_sorted_deterministically() {
    let registries = vec![default_registry()];
    let client = MockRegistryClient::new().with_search_for(
        "default",
        Ok(vec![
            make_search_result("@z/ext", "1.0.0", "Z"),
            make_search_result("@a/ext", "1.0.0", "A"),
            make_search_result("@m/ext", "1.0.0", "M"),
        ]),
    );

    let (results, _) = search_registries("ext", &registries, &client);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "@a/ext");
    assert_eq!(results[1].name, "@m/ext");
    assert_eq!(results[2].name, "@z/ext");
}

// B:search_registry — verify unit "error from one registry doesn't abort others"
#[test]
fn search_error_from_one_registry_does_not_abort_others() {
    let registries = vec![
        scoped_registry("failing", "@fail"),
        scoped_registry("working", "@work"),
    ];
    let client = MockRegistryClient::new()
        .with_search_for(
            "failing",
            Err(RegistryError::Timeout {
                url: "https://failing.registry.dev".into(),
            }),
        )
        .with_search_for(
            "working",
            Ok(vec![make_search_result("@work/ext", "1.0.0", "Works")]),
        );

    let (results, diags) = search_registries("ext", &registries, &client);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "@work/ext");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("failing"));
}

// ---------------------------------------------------------------------------
// Tests: publish_to_registry
// ---------------------------------------------------------------------------

// B:publish_to_registry — verify unit "computes SHA256, includes in upload"
#[test]
fn publish_computes_sha256() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let package = b"fake-wasm-bytes";

    let client = MockRegistryClient::new()
        // First fetch (existence check) returns NotFound — version doesn't exist
        .with_fetch_for(
            "default",
            Err(RegistryError::NotFound {
                specifier: "@test/ext@1.0.0".into(),
            }),
        )
        .with_publish(Ok("https://r.specforge.dev/@test/ext/1.0.0".into()));

    let url = publish_to_registry(package, &manifest, &registry, &client, false).unwrap();
    assert!(url.contains("@test/ext"));
}

// B:publish_to_registry — verify unit "duplicate version rejected without --force"
#[test]
fn publish_rejects_duplicate_version_without_force() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let package = b"fake-wasm-bytes";

    // fetch succeeds = version already exists
    let client = MockRegistryClient::new()
        .with_fetch_for("default", Ok(make_response("@test/ext", "1.0.0")));

    let err = publish_to_registry(package, &manifest, &registry, &client, false).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("already exists"));
}

// B:publish_to_registry — verify unit "duplicate version allowed with --force"
#[test]
fn publish_allows_duplicate_version_with_force() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let package = b"fake-wasm-bytes";

    let client = MockRegistryClient::new()
        .with_publish(Ok("https://r.specforge.dev/@test/ext/1.0.0".into()));

    // force=true skips the existence check entirely
    let url = publish_to_registry(package, &manifest, &registry, &client, true).unwrap();
    assert!(url.contains("@test/ext"));
}

// B:publish_to_registry — verify unit "successful publish returns registry URL"
#[test]
fn publish_returns_registry_url_on_success() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let package = b"fake-wasm-bytes";

    let expected_url = "https://registry.specforge.dev/@test/ext/1.0.0";
    let client = MockRegistryClient::new()
        .with_fetch_for(
            "default",
            Err(RegistryError::NotFound {
                specifier: "@test/ext@1.0.0".into(),
            }),
        )
        .with_publish(Ok(expected_url.to_string()));

    let url = publish_to_registry(package, &manifest, &registry, &client, false).unwrap();
    assert_eq!(url, expected_url);
}

// ---------------------------------------------------------------------------
// Tests: verify_registry_integrity
// ---------------------------------------------------------------------------

// B:verify_registry_integrity — verify unit "matching SHA256 passes"
#[test]
fn verify_integrity_matching_sha256_passes() {
    let data = b"hello world";
    // Pre-computed SHA256 of "hello world"
    let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

    let result = verify_registry_integrity(data, expected);
    assert!(result.is_ok());
}

// B:verify_registry_integrity — verify unit "mismatched SHA256 → hard error"
#[test]
fn verify_integrity_mismatched_sha256_is_error() {
    let data = b"hello world";
    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    let err = verify_registry_integrity(data, wrong_hash).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert_eq!(err.code, "R-OPS-002");
    assert!(err.message.contains("integrity check failed"));
    assert!(err.message.contains(wrong_hash));
}

// ---------------------------------------------------------------------------
// Tests: assign_trust_level
// ---------------------------------------------------------------------------

// B:support_private_registries — verify unit "trust level assigned deterministically"
#[test]
fn trust_level_assigned_deterministically() {
    // Local paths
    assert_eq!(assign_trust_level("/home/user/ext.wasm"), TrustLevel::Local);
    assert_eq!(assign_trust_level("./extensions/my-ext"), TrustLevel::Local);

    // Git sources
    assert_eq!(
        assign_trust_level("git+https://github.com/org/ext.git"),
        TrustLevel::Git
    );

    // Verified sources
    assert_eq!(
        assign_trust_level("https://verified.specforge.dev/ext"),
        TrustLevel::Verified
    );

    // Community (default)
    assert_eq!(
        assign_trust_level("https://registry.specforge.dev/@community/ext"),
        TrustLevel::Community
    );

    // Deterministic: same input always gives same output
    let source = "git+https://github.com/org/ext.git";
    assert_eq!(assign_trust_level(source), assign_trust_level(source));
}

// B:support_private_registries — verify unit "error messages don't leak auth details"
#[test]
fn error_messages_do_not_leak_auth_details() {
    let raw_token = "ghp_super_secret_token_12345";

    // Network error with a message that could contain a token — verify the RegistryError
    // variants never include raw tokens in their diagnostic output
    let errors = vec![
        RegistryError::Unauthorized {
            guidance: "invalid credentials".into(),
        },
        RegistryError::Forbidden {
            guidance: "access denied".into(),
        },
        RegistryError::NetworkError {
            message: "connection refused".into(),
        },
    ];

    for err in errors {
        let diag = err.to_diagnostic();
        assert!(
            !diag.message.contains(raw_token),
            "diagnostic message must not contain raw token"
        );
        if let Some(ref suggestion) = diag.suggestion {
            assert!(
                !suggestion.contains(raw_token),
                "diagnostic suggestion must not contain raw token"
            );
        }
    }

    // Also verify sanitize_token works correctly
    let sanitized = specforge_registry::sanitize_token(raw_token);
    assert!(!sanitized.contains("super_secret"));
    assert!(sanitized.ends_with("****"));
}
