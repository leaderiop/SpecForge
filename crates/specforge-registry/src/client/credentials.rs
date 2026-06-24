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
        token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        expires_at: Option<String>,
    },
    EnvVar {
        token_env: String,
    },
}

impl CredentialStore {
    pub fn get_credential(&self, alias: &str) -> Option<RegistryCredential> {
        self.registries.get(alias).map(|entry| {
            let auth_method = match entry {
                CredentialEntry::Token { token, .. } => AuthMethod::Bearer(token.clone()),
                CredentialEntry::EnvVar { token_env } => AuthMethod::TokenEnvVar(token_env.clone()),
            };
            RegistryCredential {
                alias: alias.to_string(),
                auth_method,
            }
        })
    }

    pub fn set_token(&mut self, alias: &str, token: String) {
        self.registries.insert(
            alias.to_string(),
            CredentialEntry::Token {
                token,
                expires_at: None,
            },
        );
    }

    pub fn remove(&mut self, alias: &str) -> bool {
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
        suggestion: Some(format!("delete '{}' and run `specforge login` again", path.display())),
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
    })
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
