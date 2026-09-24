#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use specforge_common::{Diagnostic, Severity};

use super::registry_client::{
    RegistryClient, RegistryError, RegistryResponse, RegistrySearchResult,
};
use super::registry_config::{RegistryConfig, RegistryCredential, find_registry_for_specifier};
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
        if matches!(
            e,
            RegistryError::NetworkError { .. } | RegistryError::Timeout { .. }
        ) && let Some(ref mut s) = diag.suggestion
        {
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

/// Publish to a registry, optionally signing the package.
///
/// Serializes the manifest once — the uploaded bytes, the `manifest_sha256`
/// inside the signature payload, and the stored manifest are byte-identical.
/// When `signing` is provided, the upload carries a [`PackageSignature`] over
/// `{name, version, wasm_sha256, manifest_sha256, signed_at}`.
/// `credential`, when provided, authenticates the upload.
/// Rejects duplicate versions unless `force` is true. Returns the registry URL on success.
pub fn publish_to_registry(
    package: &[u8],
    manifest: &ManifestV2,
    registry: &RegistryConfig,
    credential: Option<&RegistryCredential>,
    client: &dyn RegistryClient,
    force: bool,
    signing: Option<&crate::SigningKey>,
) -> Result<String, Diagnostic> {
    let manifest_json = serde_json::to_string(manifest).map_err(|e| Diagnostic {
        code: "R-OPS-003".to_string(),
        severity: specforge_common::Severity::Error,
        message: format!("failed to serialize manifest: {}", e),
        span: None,
        suggestion: None,
    })?;

    // Sign when a key is provided: the payload binds the exact uploaded
    // manifest bytes and the wasm hash to the publisher key.
    let signature = signing.map(|key| {
        let signature = key.sign_package(
            &manifest.name,
            &manifest.version,
            &hex_sha256(package),
            &hex_sha256(manifest_json.as_bytes()),
            &chrono::Utc::now().to_rfc3339(),
        );
        serde_json::to_string(&signature).expect("signature serialization cannot fail")
    });

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

    client
        .publish(
            package,
            manifest,
            &manifest_json,
            signature.as_deref(),
            registry,
            credential,
        )
        .map_err(|e| e.to_diagnostic())
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

/// Outcome of checking a registry package's publisher signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustCheck {
    /// Signature present and valid; carries the verified key id.
    Verified { key_id: String },
    /// No signature on the package (caller decides whether that is allowed).
    Unsigned,
}

/// Verify a downloaded package's publisher signature against served metadata.
///
/// Uses only data the client holds or the registry serves: the downloaded wasm
/// bytes, the served manifest JSON, and the signature wire object — the
/// registry is not the trust anchor (spec #21). Fails on: tampered wasm,
/// tampered manifest, wrong key, or metadata inconsistency between the
/// server-extracted key id and the signature object.
pub fn verify_package_signature(
    response: &RegistryResponse,
    wasm_bytes: &[u8],
) -> Result<TrustCheck, Diagnostic> {
    if response.signature.is_empty() {
        return Ok(TrustCheck::Unsigned);
    }

    let code = "R-TRUST-002";
    let signature: crate::PackageSignature =
        serde_json::from_str(&response.signature).map_err(|e| Diagnostic {
            code: code.to_string(),
            severity: Severity::Error,
            message: format!(
                "unparseable package signature for '{}': {}",
                response.name, e
            ),
            span: None,
            suggestion: Some("refuse this package; the registry response is malformed".to_string()),
        })?;

    // Cross-check the server-extracted key id against the signature object:
    // a mismatch means registry metadata was edited independently of the
    // signature (or is stale).
    if !response.key_id.is_empty() && response.key_id != signature.key_id {
        return Err(Diagnostic {
            code: "R-TRUST-004".to_string(),
            severity: Severity::Error,
            message: format!(
                "metadata inconsistency for '{}': registry says key '{}' but signature carries '{}'",
                response.name, response.key_id, signature.key_id
            ),
            span: None,
            suggestion: Some("refuse this package and verify the registry".to_string()),
        });
    }

    let manifest_sha256 = hex_sha256(response.manifest.as_bytes());
    let wasm_sha256 = hex_sha256(wasm_bytes);
    crate::verify_signature(
        &response.name,
        &response.version,
        &wasm_sha256,
        &manifest_sha256,
        &signature,
    )
    .map_err(|message| Diagnostic {
        code: code.to_string(),
        severity: Severity::Error,
        message: format!(
            "signature verification failed for '{}': {}",
            response.name, message
        ),
        span: None,
        suggestion: Some(
            "the package does not match its publisher signature; do not install it".to_string(),
        ),
    })?;

    Ok(TrustCheck::Verified {
        key_id: signature.key_id,
    })
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
