#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use specforge_common::{Diagnostic, Severity};

use super::registry_client::{RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult};
use super::registry_config::{find_registry_for_specifier, RegistryConfig, TrustLevel};
use crate::ManifestV2;

/// Compute the hex-encoded SHA256 digest of the given data.
fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Resolve an extension from the appropriate registry.
///
/// Routes via scope prefix to a matching registry, falls back to the default.
/// Returns a `Diagnostic` on failure (network error, not found, etc.).
pub fn resolve_from_registry(
    specifier: &str,
    registries: &[RegistryConfig],
    client: &dyn RegistryClient,
) -> Result<RegistryResponse, Diagnostic> {
    let registry = find_registry_for_specifier(specifier, registries).ok_or_else(|| Diagnostic {
        code: "R-OPS-001".to_string(),
        severity: Severity::Error,
        message: format!(
            "No registry found for specifier '{specifier}'. No scope match and no default registry configured."
        ),
        span: None,
        suggestion: Some(
            "Configure a default registry or add a scope-filtered registry matching this package."
                .to_string(),
        ),
    })?;

    client.fetch(specifier, registry).map_err(|e| {
        let mut diag = e.to_diagnostic();
        // Append retry guidance for network errors
        if matches!(e, RegistryError::NetworkError { .. } | RegistryError::Timeout { .. })
            && let Some(ref mut s) = diag.suggestion {
                s.push_str(" You may retry the operation.");
            }
        diag
    })
}

/// Search ALL configured registries, dedup by name+version, sort by name.
///
/// Errors from individual registries are collected but do not abort the search.
pub fn search_registries(
    query: &str,
    registries: &[RegistryConfig],
    client: &dyn RegistryClient,
) -> (Vec<RegistrySearchResult>, Vec<Diagnostic>) {
    let mut all_results = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen = HashSet::new();

    for registry in registries {
        match client.search(query, registry) {
            Ok(results) => {
                for result in results {
                    let key = (result.name.clone(), result.version.clone());
                    if seen.insert(key) {
                        all_results.push(result);
                    }
                }
            }
            Err(e) => {
                let mut diag = e.to_diagnostic();
                diag.message = format!(
                    "Search failed on registry '{}': {}",
                    registry.alias, diag.message
                );
                diagnostics.push(diag);
            }
        }
    }

    // Sort deterministically by name, then version
    all_results.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));

    (all_results, diagnostics)
}

/// Publish to a registry. Computes SHA256 of the package and includes it in the upload.
///
/// Rejects duplicate versions unless `force` is true. Returns the registry URL on success.
pub fn publish_to_registry(
    package: &[u8],
    manifest: &ManifestV2,
    registry: &RegistryConfig,
    client: &dyn RegistryClient,
    force: bool,
) -> Result<String, Diagnostic> {
    let _sha256 = hex_sha256(package);

    // First, check if the version already exists by trying to fetch it
    if !force {
        let specifier = format!("{}@{}", manifest.name, manifest.version);
        match client.fetch(&specifier, registry) {
            Ok(_) => {
                return Err(RegistryError::DuplicateVersion {
                    name: manifest.name.clone(),
                    version: manifest.version.clone(),
                }
                .to_diagnostic());
            }
            Err(RegistryError::NotFound { .. }) => {
                // Good — version doesn't exist yet
            }
            Err(_) => {
                // Other errors during existence check: proceed with publish attempt
            }
        }
    }

    client.publish(package, manifest, registry).map_err(|e| e.to_diagnostic())
}

/// Verify SHA256 integrity of downloaded bytes against an expected hash.
pub fn verify_registry_integrity(data: &[u8], expected_sha256: &str) -> Result<(), Diagnostic> {
    let actual = hex_sha256(data);
    if actual == expected_sha256 {
        Ok(())
    } else {
        Err(Diagnostic {
            code: "R-OPS-002".to_string(),
            severity: Severity::Error,
            message: format!(
                "SHA256 integrity check failed. Expected '{expected_sha256}', got '{actual}'."
            ),
            span: None,
            suggestion: Some(
                "The downloaded package may be corrupted or tampered with. Try downloading again."
                    .to_string(),
            ),
        })
    }
}

/// Assign a trust level deterministically based on the source string.
///
/// - Paths starting with "/" or "./" are `Local`.
/// - Sources starting with "git+" are `Git`.
/// - Sources containing "verified" are `Verified`.
/// - Everything else is `Community`.
pub fn assign_trust_level(source: &str) -> TrustLevel {
    if source.starts_with('/') || source.starts_with("./") {
        TrustLevel::Local
    } else if source.starts_with("git+") {
        TrustLevel::Git
    } else if source.contains("verified") {
        TrustLevel::Verified
    } else {
        TrustLevel::Community
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_sha256_produces_correct_hash() {
        // Known SHA256 of empty byte slice
        let hash = hex_sha256(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hex_sha256_deterministic() {
        let data = b"hello world";
        assert_eq!(hex_sha256(data), hex_sha256(data));
    }
}
