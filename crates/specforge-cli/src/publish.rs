use crate::OutputFormat;
use serde_json::json;
use specforge_common::{Diagnostic, codes};
use specforge_registry_client::{
    AuthMethod, CredentialStore, HttpRegistryClient, RegistryConfig, RegistryCredential,
    credentials::{credentials_path, read_credentials},
    find_registry_for, load_or_create_signing_key, publish_to_registry,
};
use std::path::Path;

/// Publish the extension at `extension` (a `.wasm` component or its crate
/// directory) to a registry `project`'s `specforge.json` configures. The
/// package's manifest is the declaration read from the binary (ADR 0012):
/// a binary whose declaration has errors is refused before any network
/// call.
pub fn run(extension: &Path, project: &Path, format: OutputFormat) -> i32 {
    let binary = match specforge_ops::publish::binary_at(extension) {
        Ok(binary) => binary,
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };
    let wasm_bytes = match std::fs::read(&binary) {
        Ok(b) => b,
        Err(e) => {
            format.print_error(
                &format!("failed to read {}: {}", binary.display(), e),
                codes::E040,
            );
            return 1;
        }
    };
    let prepared = match specforge_ops::publish::prepare(wasm_bytes) {
        Ok(prepared) => prepared,
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };
    format.eprint_diagnostics(&prepared.diagnostics);
    let declaration = &prepared.declaration;
    let wasm_bytes = &prepared.wasm;

    // No registry configured: fail before any network call (ADR 0004 N1).
    let registries = match specforge_ops_registry::configured(project, "publish") {
        Ok(configured) => {
            format.eprint_diagnostics(&configured.diagnostics);
            configured.registries
        }
        Err(error) => {
            format.print_op_error(&error);
            return 1;
        }
    };

    let package = match declaration.package_name() {
        Ok(package) => package,
        Err(why) => {
            format.print_diagnostic(&specforge_common::package::invalid(&why));
            return 1;
        }
    };
    let registry = match find_registry_for(&package, &registries) {
        Some(r) => r,
        None => {
            format.print_error(
                "no registry configured for this package scope",
                codes::R_OPS_001,
            );
            return 1;
        }
    };
    // Load publish credential: SPECFORGE_REGISTRY_TOKEN overrides stored credentials.
    let credential = match load_credential(registry) {
        Ok(credential) => credential,
        Err(diag) => {
            format.print_diagnostic(&diag);
            return 1;
        }
    };

    // Load (or first-run generate) the publisher signing key.
    let (signing_key, key_created) = match load_or_create_signing_key() {
        Ok(pair) => pair,
        Err(message) => {
            format.print_op_error(&specforge_ops::OpError::new(
                specforge_ops::OpErrorKind::Internal,
                "SIGNING_KEY_ERROR",
                message,
            ));
            return 1;
        }
    };

    // Publish
    let client = HttpRegistryClient::new();
    match publish_to_registry(
        wasm_bytes,
        declaration,
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
                        "name": declaration.name(),
                        "version": declaration.version(),
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
                    println!(
                        "published {} v{}",
                        declaration.name(),
                        declaration.version()
                    );
                    println!("  url: {}", url);
                    println!("  size: {} bytes", wasm_bytes.len());
                    println!("  signed by key: {}", key_id);
                }
            }
            0
        }
        Err(diag) => {
            format.print_diagnostic(&diag);
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
