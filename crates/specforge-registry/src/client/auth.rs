#![allow(clippy::result_large_err)]

use specforge_common::{Diagnostic, Severity};

use super::registry_client::{RegistryClient, RegistryError};
use super::registry_config::{AuthMethod, RegistryConfig, RegistryCredential};

/// Resolve a credential to its raw token string.
///
/// - `TokenEnvVar(name)` reads from the environment variable `name`.
/// - `TokenFile(path)` reads the file at `path` and trims whitespace.
/// - `Bearer(token)` returns the token directly.
pub fn resolve_credential(credential: &RegistryCredential) -> Result<String, Diagnostic> {
    match &credential.auth_method {
        AuthMethod::TokenEnvVar(var_name) => std::env::var(var_name).map_err(|_| Diagnostic {
            code: "R010".to_string(),
            severity: Severity::Error,
            message: format!(
                "Environment variable '{}' not set for registry '{}'.",
                var_name, credential.alias
            ),
            span: None,
            suggestion: Some(format!("Set the environment variable: export {var_name}=<token>")),
        }),
        AuthMethod::TokenFile(path) => {
            std::fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|e| Diagnostic {
                    code: "R011".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Cannot read token file '{}' for registry '{}': {}",
                        path.display(),
                        credential.alias,
                        e
                    ),
                    span: None,
                    suggestion: Some(format!(
                        "Ensure the file exists and is readable: {}",
                        path.display()
                    )),
                })
        }
        AuthMethod::Bearer(token) => Ok(token.clone()),
    }
}

/// Sanitize a token for safe display in diagnostic messages.
///
/// Shows the first 4 characters followed by `****` for tokens longer than 4 characters.
/// Tokens of 4 characters or fewer are fully masked as `****`.
pub fn sanitize_token(token: &str) -> String {
    if token.len() > 4 {
        format!("{}****", &token[..4])
    } else {
        "****".to_string()
    }
}

/// Validate credentials by calling `client.authenticate()`.
///
/// On failure, the error diagnostic never contains the raw token.
pub fn validate_credentials(
    client: &dyn RegistryClient,
    registry: &RegistryConfig,
    credential: &RegistryCredential,
) -> Result<(), Diagnostic> {
    client.authenticate(registry, credential).map_err(|e| {
        
        // Ensure the diagnostic never contains the raw token.
        // The RegistryError variants already produce safe messages,
        // but we wrap for consistency.
        e.to_diagnostic()
    })
}

/// Remove a credential for the given registry alias.
///
/// Returns `true` if a credential was removed, `false` if no matching alias was found.
/// This is a local-only operation; no network call is made.
pub fn logout_registry(credentials: &mut Vec<RegistryCredential>, alias: &str) -> bool {
    let before = credentials.len();
    credentials.retain(|c| c.alias != alias);
    credentials.len() < before
}

/// Attempt authentication with automatic retry on first 401.
///
/// On the first `Unauthorized` error, re-resolves the credential and retries once.
/// If the second attempt also fails with `Unauthorized`, returns an error diagnostic
/// with login guidance.
pub fn authenticate_with_retry(
    client: &dyn RegistryClient,
    registry: &RegistryConfig,
    credential: &RegistryCredential,
) -> Result<(), Diagnostic> {
    match client.authenticate(registry, credential) {
        Ok(()) => Ok(()),
        Err(RegistryError::Unauthorized { .. }) => {
            // Re-resolve credential and retry once
            let _token = resolve_credential(credential)?;
            match client.authenticate(registry, credential) {
                Ok(()) => Ok(()),
                Err(RegistryError::Unauthorized { guidance }) => Err(Diagnostic {
                    code: "R001".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Authentication failed after retry for registry '{}': {}",
                        registry.alias, guidance
                    ),
                    span: None,
                    suggestion: Some(
                        "Run `specforge registry login` to re-authenticate.".to_string(),
                    ),
                }),
                Err(other) => Err(other.to_diagnostic()),
            }
        }
        Err(other) => Err(other.to_diagnostic()),
    }
}
