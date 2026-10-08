use std::path::PathBuf;

use specforge_common::Severity;
use specforge_registry_client::auth;
use specforge_registry_client::registry_client::{RegistryError, RetryPolicy};
use specforge_registry_client::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};
use specforge_registry_client::testing::{CallKind, MemoryClient};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "test".to_string(),
        url: "https://registry.specforge.dev".to_string(),
        scope_filter: None,
        default_registry: true,
    }
}

/// How many `authenticate` calls `client` answered.
fn auth_calls(client: &MemoryClient) -> usize {
    client
        .calls()
        .iter()
        .filter(|call| call.kind == CallKind::Authenticate)
        .count()
}

fn unauthorized(guidance: &str) -> RegistryError {
    RegistryError::Unauthorized {
        guidance: guidance.into(),
    }
}

fn test_credential_env(var: &str) -> RegistryCredential {
    RegistryCredential {
        alias: "test".to_string(),
        auth_method: AuthMethod::TokenEnvVar(var.to_string()),
    }
}

fn test_credential_file(path: PathBuf) -> RegistryCredential {
    RegistryCredential {
        alias: "test".to_string(),
        auth_method: AuthMethod::TokenFile(path),
    }
}

fn test_credential_bearer(token: &str) -> RegistryCredential {
    RegistryCredential {
        alias: "test".to_string(),
        auth_method: AuthMethod::Bearer(token.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// B:auth-token-resolution — verify unit "token resolved from env var"
#[test]
fn resolve_credential_from_env_var() {
    let var_name = "SPECFORGE_TEST_TOKEN_RESOLVE";
    // SAFETY: This test is single-threaded and the env var is unique to this test.
    unsafe { std::env::set_var(var_name, "secret-token-value") };
    let cred = test_credential_env(var_name);
    let token = auth::resolve_credential(&cred).unwrap();
    assert_eq!(token, "secret-token-value");
    // SAFETY: Cleanup of test-only env var.
    unsafe { std::env::remove_var(var_name) };
}

// B:auth-token-resolution — verify unit "token resolved from file"
#[test]
fn resolve_credential_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("token.txt");
    std::fs::write(&file_path, "  file-token-value  \n").unwrap();

    let cred = test_credential_file(file_path);
    let token = auth::resolve_credential(&cred).unwrap();
    assert_eq!(token, "file-token-value");
}

// B:auth-token-resolution — verify unit "missing env var produces error diagnostic"
#[test]
fn resolve_credential_missing_env_var() {
    let var_name = "SPECFORGE_NONEXISTENT_VAR_FOR_TEST";
    // SAFETY: Ensuring env var is absent for this test.
    unsafe { std::env::remove_var(var_name) };
    let cred = test_credential_env(var_name);
    let err = auth::resolve_credential(&cred).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains(var_name));
    assert!(err.suggestion.is_some());
}

// B:auth-retry — verify unit "401 triggers re-resolve and retry once"
#[test]
fn auth_retry_on_first_401() {
    // First call: 401, second call: success
    let client = MemoryClient::new().accepting("some-token");
    client.fail_next(CallKind::Authenticate, None, unauthorized("token expired"));

    let cred = test_credential_bearer("some-token");
    let result = auth::authenticate_with_retry(&client, &test_registry(), &cred);
    assert!(result.is_ok());
    assert_eq!(auth_calls(&client), 2);
}

// B:auth-retry — verify unit "double 401 produces E-level diagnostic with login guidance"
#[test]
fn auth_double_401_produces_error_diagnostic() {
    let client = MemoryClient::new().accepting("some-token");
    client.fail_next(CallKind::Authenticate, None, unauthorized("bad token"));
    client.fail_next(CallKind::Authenticate, None, unauthorized("still bad"));

    let cred = test_credential_bearer("some-token");
    let err = auth::authenticate_with_retry(&client, &test_registry(), &cred).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("Authentication failed after retry"));
    assert!(
        err.suggestion
            .as_ref()
            .unwrap()
            .contains("specforge registry login")
    );
    assert_eq!(auth_calls(&client), 2);
}

// B:auth-forbidden — verify unit "403 produces E-level diagnostic with permission guidance"
#[test]
fn auth_403_produces_permission_error() {
    let client = MemoryClient::new().accepting("some-token");
    client.fail_next(
        CallKind::Authenticate,
        None,
        RegistryError::Forbidden {
            guidance: "insufficient scope".into(),
        },
    );

    let cred = test_credential_bearer("some-token");
    let err = auth::authenticate_with_retry(&client, &test_registry(), &cred).unwrap_err();
    assert_eq!(err.severity, Severity::Error);
    assert_eq!(err.code, "R002");
    assert!(err.message.contains("forbidden"));
}

// B:auth-sanitization — verify unit "raw tokens never appear in diagnostic messages"
#[test]
fn sanitize_token_masks_long_token() {
    let token = "ghp_abcdef123456";
    let sanitized = auth::sanitize_token(token);
    assert_eq!(sanitized, "ghp_****");
    assert!(!sanitized.contains("abcdef"));
}

// B:auth-sanitization — verify unit "short tokens fully masked"
#[test]
fn sanitize_token_masks_short_token() {
    assert_eq!(auth::sanitize_token("abc"), "****");
    assert_eq!(auth::sanitize_token("abcd"), "****");
    assert_eq!(auth::sanitize_token("abcde"), "abcd****");
}

// B:auth-sanitization — verify unit "error diagnostics never contain raw token"
#[test]
fn diagnostics_never_contain_raw_token() {
    let raw_token = "super-secret-token-value";
    let err = RegistryError::Unauthorized {
        guidance: "invalid credentials".into(),
    };
    let diag = err.to_diagnostic();
    assert!(!diag.message.contains(raw_token));
    assert!(
        !diag
            .suggestion
            .as_ref()
            .is_some_and(|s| s.contains(raw_token))
    );
}

// B:retry-policy — verify unit "exponential backoff with base 1s, max 30s, max 3 retries"
#[test]
fn retry_policy_exponential_backoff() {
    let policy = RetryPolicy::default();
    assert_eq!(policy.base_delay_ms, 1000);
    assert_eq!(policy.max_delay_ms, 30_000);
    assert_eq!(policy.max_retries, 3);

    // 1000 * 2^0 = 1000
    assert_eq!(policy.delay_for_attempt(0), 1000);
    // 1000 * 2^1 = 2000
    assert_eq!(policy.delay_for_attempt(1), 2000);
    // 1000 * 2^2 = 4000
    assert_eq!(policy.delay_for_attempt(2), 4000);
    // 1000 * 2^3 = 8000
    assert_eq!(policy.delay_for_attempt(3), 8000);
    // 1000 * 2^5 = 32000 -> capped at 30000
    assert_eq!(policy.delay_for_attempt(5), 30_000);
}

// B:registry-timeout — verify unit "timeout error produces diagnostic"
#[test]
fn timeout_error_produces_diagnostic() {
    let err = RegistryError::Timeout {
        url: "https://registry.specforge.dev/pkg".into(),
    };
    let diag = err.to_diagnostic();
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, "R004");
    assert!(diag.message.contains("timed out"));
    assert!(diag.message.contains("registry.specforge.dev"));
}

// B:validate-credentials — verify unit "valid credentials pass validation"
#[test]
fn validate_credentials_success() {
    let client = MemoryClient::new().accepting("valid-token");
    let cred = test_credential_bearer("valid-token");
    let result = auth::validate_credentials(&client, &test_registry(), &cred);
    assert!(result.is_ok());
}

// B:registry-logout — verify unit "logout removes credential, others untouched, no network call"
#[test]
fn logout_removes_credential_no_network() {
    let mut creds = vec![
        RegistryCredential {
            alias: "primary".into(),
            auth_method: AuthMethod::Bearer("tok1".into()),
        },
        RegistryCredential {
            alias: "secondary".into(),
            auth_method: AuthMethod::Bearer("tok2".into()),
        },
        RegistryCredential {
            alias: "tertiary".into(),
            auth_method: AuthMethod::Bearer("tok3".into()),
        },
    ];

    let removed = auth::logout_registry(&mut creds, "secondary");
    assert!(removed);
    assert_eq!(creds.len(), 2);
    assert!(creds.iter().all(|c| c.alias != "secondary"));
    // Remaining credentials are untouched
    assert_eq!(creds[0].alias, "primary");
    assert_eq!(creds[1].alias, "tertiary");
}

// B:registry-logout — verify unit "logout returns false when alias not found"
#[test]
fn logout_returns_false_when_not_found() {
    let mut creds = vec![RegistryCredential {
        alias: "primary".into(),
        auth_method: AuthMethod::Bearer("tok1".into()),
    }];

    let removed = auth::logout_registry(&mut creds, "nonexistent");
    assert!(!removed);
    assert_eq!(creds.len(), 1);
}

// B:auth-no-cache-fallback — verify unit "auth failure does not trigger cache fallback"
#[test]
fn auth_failure_does_not_trigger_cache_fallback() {
    // 401 and 403 should produce errors, never silently succeed via cache
    let client_401 = MemoryClient::new().accepting("tok");
    client_401.fail_next(CallKind::Authenticate, None, unauthorized("expired"));
    client_401.fail_next(CallKind::Authenticate, None, unauthorized("still expired"));

    let cred = test_credential_bearer("tok");
    let result = auth::authenticate_with_retry(&client_401, &test_registry(), &cred);
    assert!(result.is_err(), "401 must not silently succeed via cache");

    let client_403 = MemoryClient::new().accepting("tok");
    client_403.fail_next(
        CallKind::Authenticate,
        None,
        RegistryError::Forbidden {
            guidance: "no access".into(),
        },
    );

    let result = auth::authenticate_with_retry(&client_403, &test_registry(), &cred);
    assert!(result.is_err(), "403 must not silently succeed via cache");
}

// B:registry-error-to-diagnostic — verify unit "all error variants convert to diagnostics"
#[test]
fn all_registry_errors_convert_to_diagnostics() {
    let errors: Vec<RegistryError> = vec![
        RegistryError::Unauthorized {
            guidance: "g".into(),
        },
        RegistryError::Forbidden {
            guidance: "g".into(),
        },
        RegistryError::RateLimited {
            retry_after_ms: 5000,
        },
        RegistryError::Timeout {
            url: "https://x".into(),
        },
        RegistryError::NetworkError {
            message: "m".into(),
        },
        RegistryError::NotFound {
            specifier: "s".into(),
        },
        RegistryError::DuplicateVersion {
            name: "n".into(),
            version: "v".into(),
        },
    ];

    let codes = ["R001", "R002", "R003", "R004", "R005", "R006", "R007"];

    for (err, expected_code) in errors.into_iter().zip(codes.iter()) {
        let diag: specforge_common::Diagnostic = err.into();
        assert_eq!(&diag.code, expected_code, "wrong code for {expected_code}");
    }
}

#[specforge_test_macros::test(
    behavior = "resolve_registry_source",
    verify = "network error produces ExtensionError with retry guidance"
)]
fn a_network_error_suggests_a_retry() {
    let network = RegistryError::NetworkError {
        message: "connection refused".into(),
    }
    .to_diagnostic();
    assert_eq!(network.code, "R005");
    assert_eq!(network.severity, Severity::Error);
    assert!(network.message.contains("connection refused"));
    assert!(network.suggestion.as_ref().unwrap().contains("retry"));

    let timeout = RegistryError::Timeout {
        url: "http://registry.invalid/v1".into(),
    }
    .to_diagnostic();
    assert_eq!(timeout.code, "R004");
    assert_eq!(timeout.severity, Severity::Error);
    assert!(timeout.suggestion.as_ref().unwrap().contains("retry"));
}
