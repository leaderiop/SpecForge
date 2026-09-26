use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};

use super::registry_config::{AuthMethod, RegistryCredential};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialStore {
    pub registries: HashMap<String, CredentialEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialEntry {
    Token {
        /// The raw token. Empty when the secret lives in the OS keyring
        /// (`in_keyring = true`); legacy plaintext stores keep it here until
        /// the next login migrates it.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
        /// Secret stored in the OS keyring, not in this file.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        in_keyring: bool,
    },
    EnvVar {
        token_env: String,
    },
}

impl CredentialStore {
    pub fn get_credential(&self, alias: &str) -> Option<RegistryCredential> {
        self.get_credential_detail(alias).ok().flatten()
    }

    /// Resolve a credential, pulling keyring-backed secrets and rejecting
    /// expired tokens with an actionable re-login diagnostic (spec #21, T5).
    pub fn get_credential_detail(
        &self,
        alias: &str,
    ) -> Result<Option<RegistryCredential>, Diagnostic> {
        let entry = match self.registries.get(alias) {
            Some(e) => e,
            None => return Ok(None),
        };
        let auth_method = match entry {
            CredentialEntry::Token {
                token,
                expires_at,
                in_keyring,
            } => {
                // Expiry is known metadata: a clear re-login error beats a
                // cryptic 401 from the server later.
                if let Some(expires_at) = expires_at
                    .as_deref()
                    .and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok())
                    .filter(|deadline| chrono::Utc::now() > *deadline)
                {
                    return Err(Diagnostic {
                        code: "R-AUTH-020".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "stored token for registry '{}' expired at {}",
                            alias, expires_at
                        ),
                        span: None,
                        suggestion: Some(format!(
                            "run: specforge login --registry {} --token <NEW_TOKEN>",
                            alias
                        )),
                    });
                }
                let secret = if *in_keyring {
                    match super::secrets::load_secret(alias) {
                        Ok(Some(secret)) if !secret.is_empty() => secret,
                        Ok(_) => {
                            return Err(Diagnostic {
                                code: "R-AUTH-021".to_string(),
                                severity: Severity::Error,
                                message: format!(
                                    "keyring credential for '{}' is unreadable or missing",
                                    alias
                                ),
                                span: None,
                                suggestion: Some(format!(
                                    "run: specforge login --registry {} --token <NEW_TOKEN>",
                                    alias
                                )),
                            });
                        }
                        Err(message) => {
                            return Err(Diagnostic {
                                code: "R-AUTH-021".to_string(),
                                severity: Severity::Error,
                                message,
                                span: None,
                                suggestion: Some(format!(
                                    "run: specforge login --registry {} --token <NEW_TOKEN>",
                                    alias
                                )),
                            });
                        }
                    }
                } else {
                    // Either a legacy plaintext entry, or the file fallback
                    // written when the keyring round-trip failed at login.
                    if !token.is_empty() {
                        token.clone()
                    } else {
                        match super::secrets::load_secret(alias) {
                            Ok(Some(secret)) if !secret.is_empty() => secret,
                            _ => token.clone(),
                        }
                    }
                };
                AuthMethod::Bearer(secret)
            }
            CredentialEntry::EnvVar { token_env } => AuthMethod::TokenEnvVar(token_env.clone()),
        };
        Ok(Some(RegistryCredential {
            alias: alias.to_string(),
            auth_method,
        }))
    }

    /// Store a login: the secret goes to the OS keyring when available
    /// (fallback: 0600 file), the file keeps only metadata. Plaintext
    /// entries migrate to the keyring on the next login through this path.
    pub fn set_token(
        &mut self,
        alias: &str,
        token: String,
        expires_at: Option<String>,
    ) -> Result<(), String> {
        let mut backend = super::secrets::store_secret(alias, &token)?;
        // Self-verify the round trip. Some platform keychain domains accept a
        // write but subsequent reads return NoEntry (observed on macOS with
        // keyring-rs v3's data-protection domain), which would lock the user
        // out on the next command. When the keyring claims success but the
        // read-back fails, force the file backend.
        if backend == super::secrets::SecretBackend::Keyring {
            let readable = super::secrets::load_secret(alias)
                .ok()
                .flatten()
                .is_some_and(|s| s == token);
            if !readable {
                // drop the orphaned keyring item (best-effort) and use the
                // file backend instead
                super::secrets::delete_secret(alias);
                backend = super::secrets::store_secret_file(alias, &token)?;
            }
        }
        self.registries.insert(
            alias.to_string(),
            CredentialEntry::Token {
                token: String::new(),
                expires_at,
                in_keyring: backend == super::secrets::SecretBackend::Keyring,
            },
        );
        Ok(())
    }

    pub fn remove(&mut self, alias: &str) -> bool {
        // Best-effort: also drop any keyring/file secret for this alias.
        super::secrets::delete_secret(alias);
        self.registries.remove(alias).is_some()
    }
}

pub fn credentials_path() -> PathBuf {
    dirs_home().join(".specforge").join("credentials.json")
}

pub fn read_credentials(path: &Path) -> Result<CredentialStore, Diagnostic> {
    if !path.exists() {
        return Ok(CredentialStore::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| Diagnostic {
        code: "R012".to_string(),
        severity: Severity::Error,
        message: format!("failed to read credentials file: {}", e),
        span: None,
        suggestion: Some(format!("check permissions on '{}'", path.display())),
    })?;

    serde_json::from_str(&content).map_err(|e| Diagnostic {
        code: "R012".to_string(),
        severity: Severity::Error,
        message: format!("invalid credentials file format: {}", e),
        span: None,
        suggestion: Some(format!(
            "delete '{}' and run `specforge login` again",
            path.display()
        )),
    })
}

pub fn write_credentials(path: &Path, store: &CredentialStore) -> Result<(), Diagnostic> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Diagnostic {
            code: "R013".to_string(),
            severity: Severity::Error,
            message: format!("failed to create credentials directory: {}", e),
            span: None,
            suggestion: None,
        })?;
    }

    let json = serde_json::to_string_pretty(store).map_err(|e| Diagnostic {
        code: "R013".to_string(),
        severity: Severity::Error,
        message: format!("failed to serialize credentials: {}", e),
        span: None,
        suggestion: None,
    })?;

    std::fs::write(path, json).map_err(|e| Diagnostic {
        code: "R013".to_string(),
        severity: Severity::Error,
        message: format!("failed to write credentials file: {}", e),
        span: None,
        suggestion: Some(format!("check write permissions on '{}'", path.display())),
    })?;
    restrict_permissions(path);
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

pub(crate) fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install_mock_keyring() {
        use std::sync::OnceLock;
        static ONCE: OnceLock<()> = OnceLock::new();
        ONCE.get_or_init(|| {
            keyring::set_default_credential_builder(Box::new(
                crate::client::secrets::tests::mock::Builder,
            ));
        });
    }

    /// The in-memory keyring mock is process-global: tests that exercise
    /// multi-step set/read sequences on the same alias must hold this lock
    /// or they interleave and observe each other's tokens.
    static MOCK_KEYRING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_mock_keyring() -> std::sync::MutexGuard<'static, ()> {
        MOCK_KEYRING_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn set_token_keeps_secret_out_of_the_file() {
        install_mock_keyring();
        let _keyring_guard = lock_mock_keyring();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");

        let mut store = CredentialStore::default();
        store
            .set_token(
                "default",
                "sfr_super_secret".to_string(),
                Some("2027-01-01T00:00:00+00:00".to_string()),
            )
            .unwrap();
        write_credentials(&path, &store).unwrap();

        // The file on disk must not contain the token.
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert!(
            !on_disk.contains("sfr_super_secret"),
            "plaintext token leaked to file"
        );
        assert!(on_disk.contains("\"in_keyring\": true"));

        // The store reconstructs the credential from the keyring.
        let loaded = read_credentials(&path).unwrap();
        let cred = loaded.get_credential("default").unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("sfr_super_secret".to_string())
        );
    }

    #[test]
    fn expired_token_is_refused_with_relogin_hint() {
        install_mock_keyring();
        let _keyring_guard = lock_mock_keyring();
        let mut store = CredentialStore::default();
        store
            .set_token(
                "default",
                "sfr_old".to_string(),
                Some("2020-01-01T00:00:00+00:00".to_string()),
            )
            .unwrap();

        let err = store.get_credential_detail("default").unwrap_err();
        assert_eq!(err.code, "R-AUTH-020");
        assert!(err.message.contains("expired"));
        assert!(
            err.suggestion
                .unwrap_or_default()
                .contains("specforge login")
        );
    }

    #[test]
    fn legacy_plaintext_entries_still_resolve() {
        let mut store = CredentialStore::default();
        store.registries.insert(
            "default".to_string(),
            CredentialEntry::Token {
                token: "sfr_legacy".to_string(),
                expires_at: None,
                in_keyring: false,
            },
        );
        let cred = store.get_credential("default").unwrap();
        assert_eq!(
            cred.auth_method,
            AuthMethod::Bearer("sfr_legacy".to_string())
        );
    }
}
