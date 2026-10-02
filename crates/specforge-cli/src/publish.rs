use crate::OutputFormat;
use serde_json::json;
use specforge_common::Diagnostic;
use specforge_registry::ManifestV2;
use specforge_registry_client::{
    AuthMethod, CredentialStore, HttpRegistryClient, RegistryConfig, RegistryCredential,
    credentials::{credentials_path, read_credentials},
    find_registry_for_specifier, load_or_create_signing_key, publish_to_registry,
};
use std::path::Path;

pub fn run(path: &Path, format: OutputFormat) -> i32 {
    // Load manifest
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        format.print_error("no manifest.json found in current directory", "E040");
        return 1;
    }

    let manifest_content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            format.print_error(&format!("failed to read manifest.json: {}", e), "E040");
            return 1;
        }
    };

    // A misspelled field would otherwise vanish silently at parse time.
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&manifest_content) {
        for warning in specforge_registry::unknown_manifest_fields(&raw) {
            eprintln!("warning[{}]: {}", warning.code, warning.message);
        }
    }

    let manifest: ManifestV2 = match serde_json::from_str(&manifest_content) {
        Ok(m) => m,
        Err(e) => {
            format.print_error(&format!("invalid manifest.json: {}", e), "E030");
            return 1;
        }
    };

    // Load wasm binary
    let wasm_path = path.join(&manifest.wasm_path);
    if !wasm_path.exists() {
        format.print_error(
            &format!("wasm binary not found at '{}'", wasm_path.display()),
            "E040",
        );
        return 1;
    }

    let wasm_bytes = match std::fs::read(&wasm_path) {
        Ok(b) => b,
        Err(e) => {
            format.print_error(&format!("failed to read wasm binary: {}", e), "E040");
            return 1;
        }
    };

    // No registry configured: fail before any network call (ADR 0004 N1).
    let registries = match specforge_ops::registry::configured(path, "publish") {
        Ok(configured) => {
            format.eprint_diagnostics(&configured.diagnostics);
            configured.registries
        }
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };

    let registry = match find_registry_for_specifier(&manifest.name, &registries) {
        Some(r) => r,
        None => {
            format.print_error("no registry configured for this package scope", "R-OPS-001");
            return 1;
        }
    };
    // Load publish credential: SPECFORGE_REGISTRY_TOKEN overrides stored credentials.
    let credential = match load_credential(registry) {
        Ok(credential) => credential,
        Err(diag) => {
            format.print_error(&diag.message, &diag.code);
            return 1;
        }
    };

    // Load (or first-run generate) the publisher signing key.
    let (signing_key, key_created) = match load_or_create_signing_key() {
        Ok(pair) => pair,
        Err(message) => {
            format.print_error(&message, "SIGNING_KEY_ERROR");
            return 1;
        }
    };

    // Publish
    let client = HttpRegistryClient::new();
    match publish_to_registry(
        &wasm_bytes,
        &manifest,
        registry,
        credential.as_ref(),
        &client,
        false,
        Some(&signing_key),
    ) {
        Ok(url) => {
            let key_id = signing_key.key_id();
            match format {
                OutputFormat::Json => {
                    let output = json!({
                        "action": "publish",
                        "name": manifest.name,
                        "version": manifest.version,
                        "url": url,
                        "size_bytes": wasm_bytes.len(),
                        "key_id": key_id,
                        "signed": true,
                        "key_created": key_created,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                OutputFormat::Human => {
                    if key_created {
                        println!("generated publisher signing key {}", key_id);
                    }
                    println!("published {} v{}", manifest.name, manifest.version);
                    println!("  url: {}", url);
                    println!("  size: {} bytes", wasm_bytes.len());
                    println!("  signed by key: {}", key_id);
                }
            }
            0
        }
        Err(diag) => {
            format.print_error(&diag.message, &diag.code);
            1
        }
    }
}

/// Determine the credential for a publish: `SPECFORGE_REGISTRY_TOKEN` wins
/// when set to a non-blank value, otherwise the stored credential for the
/// registry alias is used.
fn select_credential(
    env_token: Option<String>,
    store: &CredentialStore,
    alias: &str,
) -> Result<Option<RegistryCredential>, Diagnostic> {
    if let Some(token) = env_token
        && !token.trim().is_empty()
    {
        return Ok(Some(RegistryCredential {
            alias: alias.to_string(),
            auth_method: AuthMethod::Bearer(token),
        }));
    }
    // Keyring-backed secrets and expired tokens surface clear diagnostics.
    store.get_credential_detail(alias)
}

fn load_credential(registry: &RegistryConfig) -> Result<Option<RegistryCredential>, Diagnostic> {
    let env_token = std::env::var("SPECFORGE_REGISTRY_TOKEN").ok();
    let store = match read_credentials(&credentials_path()) {
        Ok(store) => store,
        Err(diag) => {
            eprintln!("warning: ignoring stored credentials: {}", diag.message);
            CredentialStore::default()
        }
    };
    select_credential(env_token, &store, &registry.alias)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_token(alias: &str, token: &str) -> CredentialStore {
        let mut store = CredentialStore::default();
        // Legacy plaintext form (what a pre-keyring credentials.json holds).
        store.registries.insert(
            alias.to_string(),
            specforge_registry_client::credentials::CredentialEntry::Token {
                token: token.to_string(),
                expires_at: None,
                in_keyring: false,
            },
        );
        store
    }

    #[test]
    fn env_token_overrides_stored_credential() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(Some("env-token".to_string()), &store, "default")
            .unwrap()
            .unwrap();
        assert_eq!(cred.alias, "default");
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("env-token".to_string())
        );
    }

    #[test]
    fn blank_env_token_falls_back_to_store() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(Some("  ".to_string()), &store, "default")
            .unwrap()
            .unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("stored-token".to_string())
        );
    }

    #[test]
    fn stored_credential_used_when_env_unset() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(None, &store, "default").unwrap().unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("stored-token".to_string())
        );
    }

    #[test]
    fn missing_alias_yields_no_credential() {
        let store = store_with_token("other", "stored-token");
        assert!(
            select_credential(None, &store, "default")
                .unwrap()
                .is_none()
        );
    }
}
