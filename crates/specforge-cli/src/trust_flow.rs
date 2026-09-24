//! Trust flow shared by `specforge add` and `specforge update` (spec #21, T2).
//!
//! Sequence per package, after sha256 integrity:
//! 1. verify the publisher signature offline (`verify_package_signature`)
//! 2. refuse unsigned packages unless `--allow-unsigned`
//! 3. refuse keys on the deny list (config-level revocation)
//! 4. TOFU: pin on first install; on later installs require a match —
//!    a mismatch is a key change, resolved interactively (`--yes` for CI)
//! 5. operator allowlist (`trusted_keys`) accepts a key without a prior pin
//!    and re-pins it

use specforge_registry::{
    KnownKeys, RegistryResponse, TrustCheck, load_known_keys, save_known_keys,
    verify_package_signature,
};
use std::io::Write;
use std::path::Path;

use specforge_common::Diagnostic;

/// Outcome of the trust flow: the key id to record in the lockfile (if any).
#[derive(Debug)]
pub struct TrustOutcome {
    pub key_id: Option<String>,
}

/// Run the verification + TOFU flow for a downloaded registry package.
///
/// `known_keys_path_override` exists for tests; production callers pass
/// `None` to use `~/.specforge/known-keys.json`.
pub fn check_and_pin(
    name: &str,
    response: &RegistryResponse,
    wasm_bytes: &[u8],
    allow_unsigned: bool,
    assume_yes: bool,
    format: &str,
    known_keys_path_override: Option<&Path>,
) -> Result<TrustOutcome, Diagnostic> {
    let unsigned = |message: String, suggestion: Option<String>| Diagnostic {
        code: "R-TRUST-001".to_string(),
        severity: specforge_common::Severity::Error,
        message,
        span: None,
        suggestion,
    };

    match verify_package_signature(response, wasm_bytes)? {
        TrustCheck::Unsigned => {
            if allow_unsigned {
                if format != "json" {
                    eprintln!(
                        "warning: installing UNSIGNED package '{}' (--allow-unsigned)",
                        name
                    );
                }
                Ok(TrustOutcome { key_id: None })
            } else {
                Err(unsigned(
                    format!("package '{}' is not signed", name),
                    Some("re-run with --allow-unsigned to accept the risk".to_string()),
                ))
            }
        }
        TrustCheck::Verified { key_id } => {
            let mut known = match known_keys_path_override {
                Some(p) => load_known_keys_at(p),
                None => load_known_keys(),
            };

            // Config-level revocation wins over everything.
            if known.is_denied(&key_id) {
                return Err(Diagnostic {
                    code: "R-TRUST-005".to_string(),
                    severity: specforge_common::Severity::Error,
                    message: format!(
                        "publisher key '{}' for '{}' is denied in your known-keys config",
                        key_id, name
                    ),
                    span: None,
                    suggestion: Some(
                        "remove the key from denied_keys only if you trust it again".to_string(),
                    ),
                });
            }

            // Own the pin so the mutable re-pin below doesn't conflict with
            // the borrow the match arms would otherwise hold.
            let existing_pin = known.pin_for(name).map(str::to_string);
            match existing_pin {
                None => {
                    known.pin(name, &key_id);
                    save(&known, known_keys_path_override)?;
                    if format != "json" {
                        eprintln!("key pinned for '{}': {}", name, key_id);
                    }
                    Ok(TrustOutcome {
                        key_id: Some(key_id),
                    })
                }
                Some(pinned) if pinned == key_id => Ok(TrustOutcome {
                    key_id: Some(key_id),
                }),
                // Key change: the pin and the new signature disagree.
                Some(pinned) => {
                    let trusted = known.is_trusted(&key_id);
                    let accepted =
                        trusted || assume_yes || prompt_accept(name, &pinned, &key_id, format);
                    if !accepted {
                        return Err(Diagnostic {
                            code: "R-TRUST-003".to_string(),
                            severity: specforge_common::Severity::Error,
                            message: format!(
                                "key change rejected for '{}': pinned '{}' but package is signed '{}'",
                                name, pinned, key_id
                            ),
                            span: None,
                            suggestion: Some(format!(
                                "if you trust the new key, re-run with --yes (or add '{}' to trusted_keys)",
                                key_id
                            )),
                        });
                    }
                    known.pin(name, &key_id);
                    save(&known, known_keys_path_override)?;
                    if format != "json" {
                        eprintln!("re-pinned key for '{}': {} -> {}", name, pinned, key_id);
                    }
                    Ok(TrustOutcome {
                        key_id: Some(key_id),
                    })
                }
            }
        }
    }
}

fn save(known: &KnownKeys, override_path: Option<&Path>) -> Result<(), Diagnostic> {
    let result = match override_path {
        Some(p) => save_known_keys_at(p, known),
        None => save_known_keys(known),
    };
    result.map_err(|message| Diagnostic {
        code: "R-TRUST-006".to_string(),
        severity: specforge_common::Severity::Error,
        message,
        span: None,
        suggestion: Some("check permissions on the file".to_string()),
    })
}

/// Ask the human to accept a key change. Refusal is the default.
fn prompt_accept(name: &str, old: &str, new: &str, format: &str) -> bool {
    if format == "json" {
        // Non-interactive output mode: never prompt.
        return false;
    }
    eprintln!(
        "KEY CHANGE for '{}': pinned '{}' but new package is signed '{}'",
        name, old, new
    );
    eprint!("trust the new key and re-pin? [y/N] ");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

// Re-exported so callers can hit the same store paths in tests.
pub use specforge_registry::client::trust::{load_known_keys_at, save_known_keys_at};

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_registry::SigningKey;

    fn signed_response(key: &SigningKey, manifest_json: &str, wasm: &[u8]) -> RegistryResponse {
        use sha2::{Digest, Sha256};
        let hash = |d: &[u8]| {
            let mut h = Sha256::new();
            h.update(d);
            hex::encode(h.finalize())
        };
        let sig = key.sign_package(
            "@acme/tool",
            "1.0.0",
            &hash(wasm),
            &hash(manifest_json.as_bytes()),
            "2026-09-24T00:00:00+00:00",
        );
        RegistryResponse {
            name: "@acme/tool".to_string(),
            version: "1.0.0".to_string(),
            wasm_url: String::new(),
            sha256: hash(wasm),
            signature: serde_json::to_string(&sig).unwrap(),
            key_id: sig.key_id.clone(),
            manifest: manifest_json.to_string(),
        }
    }

    fn unsigned_response() -> RegistryResponse {
        RegistryResponse {
            name: "@acme/tool".to_string(),
            version: "1.0.0".to_string(),
            wasm_url: String::new(),
            sha256: "x".to_string(),
            signature: String::new(),
            key_id: String::new(),
            manifest: String::new(),
        }
    }

    const MANIFEST: &str = r#"{"name":"@acme/tool","version":"1.0.0"}"#;
    const WASM: &[u8] = b"\0asm-bytes";

    #[test]
    fn unsigned_package_is_refused_without_flag() {
        let response = unsigned_response();
        let err =
            check_and_pin("@acme/tool", &response, WASM, false, false, "human", None).unwrap_err();
        assert_eq!(err.code, "R-TRUST-001");
        assert!(
            err.suggestion
                .unwrap_or_default()
                .contains("--allow-unsigned")
        );
    }

    #[test]
    fn unsigned_package_passes_with_flag_and_no_pin() {
        let response = unsigned_response();
        let outcome =
            check_and_pin("@acme/tool", &response, WASM, true, false, "human", None).unwrap();
        assert!(outcome.key_id.is_none());
    }

    #[test]
    fn first_verified_install_pins_the_key() {
        let key = SigningKey::generate();
        let response = signed_response(&key, MANIFEST, WASM);
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        let outcome = check_and_pin(
            "@acme/tool",
            &response,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap();
        assert_eq!(outcome.key_id.as_deref(), Some(key.key_id().as_str()));

        let known = load_known_keys_at(&store);
        assert_eq!(known.pin_for("@acme/tool"), Some(key.key_id().as_str()));
    }

    #[test]
    fn matching_pin_accepts_without_reprompt() {
        let key = SigningKey::generate();
        let response = signed_response(&key, MANIFEST, WASM);
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        check_and_pin(
            "@acme/tool",
            &response,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap();
        // Second install of the same package/key: accepted, pin unchanged.
        let outcome = check_and_pin(
            "@acme/tool",
            &response,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap();
        assert_eq!(outcome.key_id.as_deref(), Some(key.key_id().as_str()));
    }

    #[test]
    fn key_change_without_consent_is_refused() {
        let key_a = SigningKey::generate();
        let key_b = SigningKey::generate();
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        let first = signed_response(&key_a, MANIFEST, WASM);
        check_and_pin(
            "@acme/tool",
            &first,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap();

        // Different key signs the same package: non-interactive refusal.
        let second = signed_response(&key_b, MANIFEST, WASM);
        let err = check_and_pin(
            "@acme/tool",
            &second,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap_err();
        assert_eq!(err.code, "R-TRUST-003");

        // assume_yes accepts and re-pins.
        let outcome = check_and_pin(
            "@acme/tool",
            &second,
            WASM,
            false,
            true,
            "human",
            Some(&store),
        )
        .unwrap();
        assert_eq!(outcome.key_id.as_deref(), Some(key_b.key_id().as_str()));
        let known = load_known_keys_at(&store);
        assert_eq!(known.pin_for("@acme/tool"), Some(key_b.key_id().as_str()));
    }

    #[test]
    fn trusted_key_is_accepted_despite_missing_pin() {
        let key = SigningKey::generate();
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        // Pre-seed the allowlist with this key.
        let mut known = KnownKeys::default();
        known.trusted_keys.push(key.key_id());
        save_known_keys_at(&store, &known).unwrap();

        let response = signed_response(&key, MANIFEST, WASM);
        let outcome = check_and_pin(
            "@acme/tool",
            &response,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap();
        assert_eq!(outcome.key_id.as_deref(), Some(key.key_id().as_str()));
    }

    #[test]
    fn denied_key_is_refused_even_if_pinned() {
        let key = SigningKey::generate();
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        let mut known = KnownKeys::default();
        known.pins.insert("@acme/tool".to_string(), key.key_id());
        known.denied_keys.push(key.key_id());
        save_known_keys_at(&store, &known).unwrap();

        let response = signed_response(&key, MANIFEST, WASM);
        let err = check_and_pin(
            "@acme/tool",
            &response,
            WASM,
            false,
            false,
            "human",
            Some(&store),
        )
        .unwrap_err();
        assert_eq!(err.code, "R-TRUST-005");
    }

    #[test]
    fn tampered_wasm_is_refused_even_with_valid_pin() {
        let key = SigningKey::generate();
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("known-keys.json");

        let response = signed_response(&key, MANIFEST, WASM);
        let tampered: &[u8] = b"\0asm-evil";

        // No --allow-unsigned escape for broken signatures.
        let err = check_and_pin(
            "@acme/tool",
            &response,
            tampered,
            true,
            false,
            "human",
            Some(&store),
        )
        .unwrap_err();
        assert_eq!(err.code, "R-TRUST-002");
    }

    // Keep the unused import referenced when hex is only used in helpers.
    #[allow(unused_imports)]
    use sha2::Digest as _;
}
