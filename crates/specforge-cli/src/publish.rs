use serde_json::json;
use specforge_registry::{
    AuthMethod, CredentialStore, HttpRegistryClient, ManifestV2, RegistryConfig,
    RegistryCredential,
    client::credentials::{credentials_path, read_credentials},
    find_registry_for_specifier, load_or_create_signing_key, parse_registries_from_config,
    publish_to_registry,
};
use std::path::Path;

pub fn run(path: &Path, format: &str) -> i32 {
    // Load manifest
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        print_error(
            format,
            "no manifest.json found in current directory",
            "E-PUB-001",
        );
        return 1;
    }

    let manifest_content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(e) => {
            print_error(
                format,
                &format!("failed to read manifest.json: {}", e),
                "E-PUB-001",
            );
            return 1;
        }
    };

    let manifest: ManifestV2 = match serde_json::from_str(&manifest_content) {
        Ok(m) => m,
        Err(e) => {
            print_error(
                format,
                &format!("invalid manifest.json: {}", e),
                "E-PUB-002",
            );
            return 1;
        }
    };

    // Load wasm binary
    let wasm_path = path.join(&manifest.wasm_path);
    if !wasm_path.exists() {
        print_error(
            format,
            &format!("wasm binary not found at '{}'", wasm_path.display()),
            "E-PUB-003",
        );
        return 1;
    }

    let wasm_bytes = match std::fs::read(&wasm_path) {
        Ok(b) => b,
        Err(e) => {
            print_error(
                format,
                &format!("failed to read wasm binary: {}", e),
                "E-PUB-003",
            );
            return 1;
        }
    };

    // Load registry config
    let project_config_path = path.join("specforge.json");
    let registries = load_registries(&project_config_path);

    let registry = match find_registry_for_specifier(&manifest.name, &registries) {
        Some(r) => r,
        None => {
            print_error(
                format,
                "no registry configured for this package scope",
                "R-OPS-001",
            );
            return 1;
        }
    };
    // Load publish credential: SPECFORGE_REGISTRY_TOKEN overrides stored credentials.
    let credential = load_credential(registry);

    // Load (or first-run generate) the publisher signing key.
    let (signing_key, key_created) = match load_or_create_signing_key() {
        Ok(pair) => pair,
        Err(message) => {
            print_error(format, &message, "SIGNING_KEY_ERROR");
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
                "json" => {
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
                _ => {
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
            print_error(format, &diag.message, &diag.code);
            1
        }
    }
}

fn load_registries(config_path: &Path) -> Vec<RegistryConfig> {
    if !config_path.exists() {
        return vec![default_registry()];
    }

    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => return vec![default_registry()],
    };

    let (registries, _) = parse_registries_from_config(&content);
    if registries.is_empty() {
        vec![default_registry()]
    } else {
        registries
    }
}

fn default_registry() -> RegistryConfig {
    RegistryConfig {
        alias: "default".to_string(),
        url: "https://registry.specforge.dev/v1".to_string(),
        scope_filter: None,
        default_registry: true,
    }
}

fn print_error(format: &str, message: &str, code: &str) {
    match format {
        "json" => {
            let output = json!({"error": message, "code": code});
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        _ => eprintln!("error[{}]: {}", code, message),
    }
}

/// Determine the credential for a publish: `SPECFORGE_REGISTRY_TOKEN` wins
/// when set to a non-blank value, otherwise the stored credential for the
/// registry alias is used.
fn select_credential(
    env_token: Option<String>,
    store: &CredentialStore,
    alias: &str,
) -> Option<RegistryCredential> {
    if let Some(token) = env_token
        && !token.trim().is_empty()
    {
        return Some(RegistryCredential {
            alias: alias.to_string(),
            auth_method: AuthMethod::Bearer(token),
        });
    }
    store.get_credential(alias)
}

fn load_credential(registry: &RegistryConfig) -> Option<RegistryCredential> {
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
        store.set_token(alias, token.to_string());
        store
    }

    #[test]
    fn env_token_overrides_stored_credential() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(Some("env-token".to_string()), &store, "default").unwrap();
        assert_eq!(cred.alias, "default");
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("env-token".to_string())
        );
    }

    #[test]
    fn blank_env_token_falls_back_to_store() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(Some("  ".to_string()), &store, "default").unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("stored-token".to_string())
        );
    }

    #[test]
    fn stored_credential_used_when_env_unset() {
        let store = store_with_token("default", "stored-token");
        let cred = select_credential(None, &store, "default").unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("stored-token".to_string())
        );
    }

    #[test]
    fn missing_alias_yields_no_credential() {
        let store = store_with_token("other", "stored-token");
        assert!(select_credential(None, &store, "default").is_none());
    }
}
