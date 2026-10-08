use specforge_common::Severity;
use specforge_protocol_types::package::Version;
use specforge_protocol_types::{ExtensionDeclaration, PackageName};
use specforge_registry_client::registry_ops::{
    publish_to_registry, search_registries, verify_registry_integrity,
};
use specforge_registry_client::testing::{Call, CallKind, MemoryClient, package};
use specforge_registry_client::{
    AuthMethod, RegistryClient, RegistryConfig, RegistryCredential, RegistryError, SigningKey,
};
use specforge_registry_wire::{PackageMetadata, path};

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

fn minimal_manifest() -> ExtensionDeclaration {
    serde_json::from_value(serde_json::json!({
        "handshake": {
            "protocol_version": "1.0.0",
            "name": "@test/ext",
            "version": "1.0.0",
            "contribution_flags": {},
            "peer_dependencies": [],
            "sandbox_policy": null
        }
    }))
    .unwrap()
}

/// A token the registry accepts, and the credential that sends it.
const TOKEN: &str = "publisher-token";

fn accepted() -> RegistryCredential {
    RegistryCredential {
        alias: "default".to_string(),
        auth_method: AuthMethod::Bearer(TOKEN.to_string()),
    }
}

fn client() -> MemoryClient {
    MemoryClient::new().accepting(TOKEN)
}

/// `registry` holds `name@version`, found by `desc` in a search.
fn store(client: &MemoryClient, registry: &RegistryConfig, name: &str, version: &str, desc: &str) {
    client.store(
        registry,
        PackageMetadata {
            name: name.to_string(),
            version: version.to_string(),
            description: desc.to_string(),
            ..Default::default()
        },
        Vec::new(),
    );
}

fn publishes(client: &MemoryClient) -> Vec<Call> {
    client
        .calls()
        .into_iter()
        .filter(|call| call.kind == CallKind::Publish)
        .collect()
}

// ---------------------------------------------------------------------------
// Tests: search_registries
// ---------------------------------------------------------------------------

// B:search_registry — verify unit "queries ALL configured registries"
#[test]
fn search_queries_all_registries() {
    let (a, b) = (
        scoped_registry("reg-a", "@alpha"),
        scoped_registry("reg-b", "@beta"),
    );
    let client = MemoryClient::new();
    store(&client, &a, "@alpha/ext", "1.0.0", "Alpha ext");
    store(&client, &b, "@beta/ext", "1.0.0", "Beta ext");

    let (results, diags) = search_registries("ext", &[a, b], &client);
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
    let (a, b) = (
        scoped_registry("reg-a", "@specforge"),
        scoped_registry("reg-b", "@specforge"),
    );
    // Both registries return the same package
    let client = MemoryClient::new();
    store(&client, &a, "@specforge/software", "1.0.0", "From A");
    store(&client, &b, "@specforge/software", "1.0.0", "From B");

    let (results, diags) = search_registries("software", &[a, b], &client);
    assert!(diags.is_empty());
    assert_eq!(results.len(), 1, "duplicate should be removed");
    assert_eq!(results[0].name, "@specforge/software");
}

// B:search_registry — verify unit "search output deterministic (sorted)"
#[test]
fn search_results_sorted_deterministically() {
    let registry = default_registry();
    let client = MemoryClient::new();
    for name in ["@z/ext", "@a/ext", "@m/ext"] {
        store(&client, &registry, name, "1.0.0", "ext");
    }

    let (results, _) = search_registries("ext", &[registry], &client);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].name, "@a/ext");
    assert_eq!(results[1].name, "@m/ext");
    assert_eq!(results[2].name, "@z/ext");
}

// B:search_registry — verify unit "error from one registry doesn't abort others"
#[test]
fn search_error_from_one_registry_does_not_abort_others() {
    let (failing, working) = (
        scoped_registry("failing", "@fail"),
        scoped_registry("working", "@work"),
    );
    let client = MemoryClient::new();
    client.fail_next(
        CallKind::Search,
        Some(&failing),
        RegistryError::Timeout {
            url: "https://failing.registry.dev".into(),
        },
    );
    store(&client, &working, "@work/ext", "1.0.0", "Works");

    let (results, diags) = search_registries("ext", &[failing, working], &client);
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
    let package_bytes = b"fake-wasm-bytes";
    let client = client();

    let url = publish_to_registry(
        package_bytes,
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        false,
        None,
    )
    .unwrap();
    assert!(url.contains("@test"));

    let stored = client
        .metadata(
            &PackageName::parse("@test/ext").unwrap(),
            &Version::new(1, 0, 0),
            &registry,
        )
        .unwrap();
    assert_eq!(
        stored.sha256,
        package("@test/ext", "1.0.0", package_bytes, "", None).sha256
    );
    assert_eq!(stored.size_bytes, package_bytes.len() as u64);
}

// B:publish_to_registry — verify unit "duplicate version rejected without --force"
#[test]
fn publish_rejects_duplicate_version_without_force() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let client = client();
    // the version is already there
    store(&client, &registry, "@test/ext", "1.0.0", "");

    let err = publish_to_registry(
        b"fake-wasm-bytes",
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        false,
        None,
    )
    .unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("already exists"));
    assert!(publishes(&client).is_empty(), "nothing was uploaded");
}

// B:publish_to_registry — verify unit "duplicate version allowed with --force"
#[test]
fn publish_allows_duplicate_version_with_force() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let client = client();

    // force=true skips the existence check entirely
    let url = publish_to_registry(
        b"fake-wasm-bytes",
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        true,
        None,
    )
    .unwrap();
    assert!(url.contains("@test"));
    assert!(
        client
            .calls()
            .iter()
            .all(|call| call.kind != CallKind::Metadata),
        "no existence check was made"
    );
}

// B:publish_to_registry — verify unit "successful publish returns registry URL"
#[test]
fn publish_returns_registry_url_on_success() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let client = client();

    let expected = format!(
        "{}{}",
        registry.url,
        path::version(
            &PackageName::parse("@test/ext").unwrap(),
            &Version::new(1, 0, 0)
        )
    );
    let url = publish_to_registry(
        b"fake-wasm-bytes",
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        false,
        None,
    )
    .unwrap();
    assert_eq!(url, expected);
}

// B:publish_to_registry — verify unit "threads credential into client.publish"
#[test]
fn publish_threads_credential_to_client() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let credential = accepted();

    let client = client();
    publish_to_registry(
        b"fake-wasm-bytes",
        &manifest,
        &registry,
        Some(&credential),
        &client,
        true,
        None,
    )
    .unwrap();
    assert_eq!(
        publishes(&client)
            .into_iter()
            .map(|call| call.credential)
            .collect::<Vec<_>>(),
        vec![Some(credential.clone())]
    );

    // An anonymous publish carries no credential, and the registry refuses it.
    let anonymous = self::client();
    let err = publish_to_registry(
        b"fake-wasm-bytes",
        &manifest,
        &registry,
        None,
        &anonymous,
        true,
        None,
    )
    .unwrap_err();
    assert_eq!(err.code, "R001");
    assert_eq!(
        publishes(&anonymous)
            .into_iter()
            .map(|call| call.credential)
            .collect::<Vec<_>>(),
        vec![None]
    );
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
    let sanitized = specforge_registry_client::sanitize_token(raw_token);
    assert!(!sanitized.contains("super_secret"));
    assert!(sanitized.ends_with("****"));
}

// B:publish_to_registry — verify unit "signed publish carries verifiable signature"
#[test]
fn publish_signs_package_when_key_provided() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let key = SigningKey::generate();
    let client = client();

    publish_to_registry(
        b"wasm-bytes",
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        false,
        Some(&key),
    )
    .unwrap();

    let sigs: Vec<Option<String>> = publishes(&client)
        .into_iter()
        .map(|call| call.signature)
        .collect();
    assert_eq!(sigs.len(), 1);
    let sig: specforge_registry_client::PackageSignature =
        serde_json::from_str(sigs[0].as_deref().expect("signature present")).unwrap();
    assert_eq!(sig.key_id, key.key_id());
    assert_eq!(sig.public_key, key.public_key_hex());
}

// B:publish_to_registry — verify unit "unsigned publish sends no signature"
#[test]
fn publish_without_key_sends_no_signature() {
    let registry = default_registry();
    let manifest = minimal_manifest();
    let client = client();

    publish_to_registry(
        b"wasm-bytes",
        &manifest,
        &registry,
        Some(&accepted()),
        &client,
        false,
        None,
    )
    .unwrap();
    assert_eq!(
        publishes(&client)
            .into_iter()
            .map(|call| call.signature)
            .collect::<Vec<_>>(),
        vec![None]
    );
}
