//! Package signing for registry publishes (wayfinder map #8, spec #21 / ticket T1).
//!
//! Publisher-held Ed25519 keys over a canonical payload:
//! `{name, version, wasm_sha256, manifest_sha256, signed_at}` (camelCase JSON,
//! struct field order = wire order, so serialization is deterministic).
//!
//! The signature travels as a JSON object `{"sig", "keyId", "pubkey", "signedAt"}`
//! (hex except the timestamp): the registry stores and serves it, but is never
//! the trust anchor — the client verifies against the pubkey in the object and
//! pins the key id (map decisions #11/#12). Because the timestamp rides inside
//! the object, a verifier can rebuild the canonical payload from served
//! metadata plus the downloaded wasm and manifest bytes.

use ed25519_dalek::{Signature, Signer, SigningKey as DalekSigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A package signature plus everything needed to verify it offline.
///
/// Serialized form is the exact JSON uploaded in the `signature` multipart
/// field and stored in `packages.signature`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSignature {
    /// Hex-encoded Ed25519 signature (128 hex chars).
    pub sig: String,
    /// Short public key id: first 16 hex chars of SHA256(pubkey).
    #[serde(rename = "keyId")]
    pub key_id: String,
    /// Hex-encoded Ed25519 public key (64 hex chars).
    #[serde(rename = "pubkey")]
    pub public_key: String,
    /// RFC3339 signing timestamp — carried in the object so a verifier can
    /// rebuild the canonical payload from served metadata alone.
    #[serde(rename = "signedAt")]
    pub signed_at: String,
}

/// Canonical signing payload. Field order is wire order: serde serializes
/// struct fields in declaration order, so the bytes are deterministic.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedPayload<'a> {
    name: &'a str,
    version: &'a str,
    wasm_sha256: &'a str,
    manifest_sha256: &'a str,
    signed_at: &'a str,
}

/// A publisher signing key: Ed25519 secret key with derived id helpers.
#[derive(Debug, Clone)]
pub struct SigningKey {
    inner: DalekSigningKey,
}

impl SigningKey {
    /// Generate a fresh random signing key from OS entropy.
    pub fn generate() -> Self {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).expect("OS entropy unavailable");
        Self::from_bytes(&secret)
    }

    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        Self {
            inner: DalekSigningKey::from_bytes(secret),
        }
    }

    /// Short public key id: first 16 hex chars of SHA256(pubkey).
    pub fn key_id(&self) -> String {
        key_id_from_pubkey(&self.public_key_hex())
    }

    pub fn public_key_hex(&self) -> String {
        hex::encode(self.inner.verifying_key().to_bytes())
    }

    /// Sign the canonical payload for a package and produce the wire object.
    pub fn sign_package(
        &self,
        name: &str,
        version: &str,
        wasm_sha256: &str,
        manifest_sha256: &str,
        signed_at: &str,
    ) -> PackageSignature {
        let payload = canonical_payload(name, version, wasm_sha256, manifest_sha256, signed_at);
        let sig = self.inner.sign(&payload).to_bytes();
        PackageSignature {
            sig: hex::encode(sig),
            key_id: self.key_id(),
            public_key: self.public_key_hex(),
            signed_at: signed_at.to_string(),
        }
    }
}

/// Build the deterministic canonical payload bytes for a package.
pub fn canonical_payload(
    name: &str,
    version: &str,
    wasm_sha256: &str,
    manifest_sha256: &str,
    signed_at: &str,
) -> Vec<u8> {
    serde_json::to_vec(&SignedPayload {
        name,
        version,
        wasm_sha256,
        manifest_sha256,
        signed_at,
    })
    .expect("payload serialization cannot fail")
}

/// Verify a package signature using only served metadata.
///
/// Rebuilds the canonical payload from the package identity, the wasm hash of
/// the downloaded bytes, the manifest hash of the served manifest, and the
/// timestamp carried inside the signature object. Also fails when the pubkey
/// does not match the claimed key id (wire-object integrity).
pub fn verify_signature(
    name: &str,
    version: &str,
    wasm_sha256: &str,
    manifest_sha256: &str,
    signature: &PackageSignature,
) -> Result<(), String> {
    let expected_key_id = key_id_from_pubkey(&signature.public_key);
    if expected_key_id != signature.key_id {
        return Err(format!(
            "key id mismatch: signature claims '{}' but pubkey hashes to '{}'",
            signature.key_id, expected_key_id
        ));
    }
    let key_bytes: [u8; 32] = hex::decode(&signature.public_key)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| "invalid public key encoding".to_string())?;
    let verifying_key =
        VerifyingKey::from_bytes(&key_bytes).map_err(|e| format!("invalid public key: {}", e))?;
    let sig_bytes: [u8; 64] = hex::decode(&signature.sig)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| "invalid signature encoding".to_string())?;
    let signature_bytes = Signature::from_bytes(&sig_bytes);
    let payload = canonical_payload(
        name,
        version,
        wasm_sha256,
        manifest_sha256,
        &signature.signed_at,
    );
    verifying_key
        .verify(&payload, &signature_bytes)
        .map_err(|e| format!("signature verification failed: {}", e))
}

/// Short public key id: first 16 hex chars of SHA256(pubkey).
pub fn key_id_from_pubkey(public_key_hex: &str) -> String {
    let decoded = hex::decode(public_key_hex).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&decoded);
    hex::encode(hasher.finalize())[..16].to_string()
}

/// Default on-disk location of the publisher signing key.
pub fn signing_key_path() -> PathBuf {
    crate::client::credentials::dirs_home()
        .join(".specforge")
        .join("signing-key.json")
}

/// Load the local signing key, generating and persisting one on first use.
///
/// Returns the key and whether it was newly created.
pub fn load_or_create_signing_key() -> Result<(SigningKey, bool), String> {
    load_or_create_signing_key_at(&signing_key_path())
}

/// [`load_or_create_signing_key`] at an explicit path (tests, custom homes).
pub fn load_or_create_signing_key_at(path: &Path) -> Result<(SigningKey, bool), String> {
    if let Ok(content) = std::fs::read_to_string(path) {
        #[derive(Deserialize)]
        struct StoredKey {
            #[serde(rename = "secretKey")]
            secret_key: String,
        }
        let stored: StoredKey = serde_json::from_str(&content)
            .map_err(|e| format!("corrupt signing key file {}: {}", path.display(), e))?;
        let secret: [u8; 32] = hex::decode(stored.secret_key.trim())
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| {
                format!(
                    "corrupt signing key file {}: bad secret encoding",
                    path.display()
                )
            })?;
        return Ok((SigningKey::from_bytes(&secret), false));
    }

    let key = SigningKey::generate();
    #[derive(Serialize)]
    struct StoredKey {
        #[serde(rename = "secretKey")]
        secret_key: String,
        #[serde(rename = "createdAt")]
        created_at: String,
    }
    let stored = StoredKey {
        secret_key: hex::encode(key.inner.to_bytes()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&stored).unwrap())
        .map_err(|e| format!("failed to write signing key {}: {}", path.display(), e))?;
    restrict_permissions(path);
    Ok((key, true))
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    const NAME: &str = "@acme/greeter";
    const VERSION: &str = "1.2.0";
    const WASM: &str = "aaaa";
    const MANIFEST: &str = "bbbb";
    const AT: &str = "2026-09-24T00:00:00+00:00";

    #[test]
    fn canonical_payload_is_deterministic_and_ordered() {
        let a = canonical_payload(NAME, VERSION, WASM, MANIFEST, AT);
        let b = canonical_payload(NAME, VERSION, WASM, MANIFEST, AT);
        assert_eq!(a, b);
        let text = String::from_utf8(a).unwrap();
        // Field order is wire order: name < version < wasmSha256 < manifestSha256 < signedAt.
        let name_pos = text.find("\"name\"").unwrap();
        let version_pos = text.find("\"version\"").unwrap();
        let wasm_pos = text.find("\"wasmSha256\"").unwrap();
        let manifest_pos = text.find("\"manifestSha256\"").unwrap();
        let at_pos = text.find("\"signedAt\"").unwrap();
        assert!(name_pos < version_pos);
        assert!(version_pos < wasm_pos);
        assert!(wasm_pos < manifest_pos);
        assert!(manifest_pos < at_pos);
    }

    #[test]
    fn canonical_payload_changes_when_any_field_changes() {
        let base = canonical_payload(NAME, VERSION, WASM, MANIFEST, AT);
        assert_ne!(
            base,
            canonical_payload("@acme/other", VERSION, WASM, MANIFEST, AT)
        );
        assert_ne!(base, canonical_payload(NAME, "9.9.9", WASM, MANIFEST, AT));
        assert_ne!(base, canonical_payload(NAME, VERSION, "cccc", MANIFEST, AT));
        assert_ne!(base, canonical_payload(NAME, VERSION, WASM, "dddd", AT));
        assert_ne!(
            base,
            canonical_payload(NAME, VERSION, WASM, MANIFEST, "2027-01-01T00:00:00+00:00")
        );
    }

    #[test]
    fn sign_then_verify_roundtrips() {
        let key = SigningKey::generate();
        let sig = key.sign_package(NAME, VERSION, WASM, MANIFEST, AT);
        assert_eq!(sig.key_id, key.key_id());
        assert_eq!(sig.key_id.len(), 16);
        assert_eq!(sig.signed_at, AT);
        verify_signature(NAME, VERSION, WASM, MANIFEST, &sig).expect("should verify");
    }

    #[test]
    fn verification_fails_on_tampered_payload() {
        let key = SigningKey::generate();
        let sig = key.sign_package(NAME, VERSION, WASM, MANIFEST, AT);
        let err = verify_signature(NAME, VERSION, "cccc", MANIFEST, &sig).unwrap_err();
        assert!(err.contains("verification failed"), "{}", err);
    }

    #[test]
    fn verification_fails_when_pubkey_does_not_match_key_id() {
        let key = SigningKey::generate();
        let other = SigningKey::generate();
        let mut sig = key.sign_package(NAME, VERSION, WASM, MANIFEST, AT);
        sig.key_id = other.key_id();
        let err = verify_signature(NAME, VERSION, WASM, MANIFEST, &sig).unwrap_err();
        assert!(err.contains("key id mismatch"), "{}", err);
    }

    #[test]
    fn different_keys_produce_different_key_ids() {
        let a = SigningKey::generate();
        let b = SigningKey::generate();
        assert_ne!(a.key_id(), b.key_id());
    }

    #[test]
    fn signing_key_persists_and_reloads_identically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("signing-key.json");
        let (key, created) = load_or_create_signing_key_at(&path).unwrap();
        assert!(created);
        let (reloaded, created_again) = load_or_create_signing_key_at(&path).unwrap();
        assert!(!created_again);
        assert_eq!(key.key_id(), reloaded.key_id());
        assert_eq!(key.public_key_hex(), reloaded.public_key_hex());
        // The reloaded key signs identically.
        let a = key.sign_package(NAME, VERSION, WASM, MANIFEST, AT);
        let b = reloaded.sign_package(NAME, VERSION, WASM, MANIFEST, AT);
        assert_eq!(a, b);
    }
}
