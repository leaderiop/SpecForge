use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryConfig {
    pub alias: String,
    pub url: String,
    #[serde(default)]
    pub scope_filter: Option<String>,
    #[serde(default)]
    pub default_registry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryCredential {
    pub alias: String,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
    TokenEnvVar(String),
    TokenFile(PathBuf),
    Bearer(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    Local,
    Git,
    Community,
    Verified,
}

/// Parses the `"registries"` array from a JSON config string.
///
/// Returns a tuple of (parsed registries, diagnostics).
/// Produces an I003 info diagnostic when no default registry is configured.
/// Produces a W-level diagnostic for duplicate aliases.
pub fn parse_registries_from_config(config_json: &str) -> (Vec<RegistryConfig>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();

    let parsed: serde_json::Value = match serde_json::from_str(config_json) {
        Ok(v) => v,
        Err(e) => {
            diagnostics.push(Diagnostic {
                code: "E-REG-001".to_string(),
                severity: Severity::Error,
                message: format!("Failed to parse registry config JSON: {e}"),
                span: None,
                suggestion: Some("Ensure the configuration is valid JSON.".to_string()),
            });
            return (Vec::new(), diagnostics);
        }
    };

    let registries_value = match parsed.get("registries") {
        Some(v) => v,
        None => {
            diagnostics.push(Diagnostic {
                code: "I003".to_string(),
                severity: Severity::Info,
                message: "No registries configured and no default registry set.".to_string(),
                span: None,
                suggestion: Some(
                    "Add a \"registries\" array to your configuration.".to_string(),
                ),
            });
            return (Vec::new(), diagnostics);
        }
    };

    let registries_array = match registries_value.as_array() {
        Some(arr) => arr,
        None => {
            diagnostics.push(Diagnostic {
                code: "E-REG-002".to_string(),
                severity: Severity::Error,
                message: "\"registries\" must be a JSON array.".to_string(),
                span: None,
                suggestion: None,
            });
            return (Vec::new(), diagnostics);
        }
    };

    let mut registries = Vec::new();
    let mut seen_aliases = std::collections::HashSet::new();

    for (i, entry) in registries_array.iter().enumerate() {
        match serde_json::from_value::<RegistryConfig>(entry.clone()) {
            Ok(reg) => {
                if !seen_aliases.insert(reg.alias.clone()) {
                    diagnostics.push(Diagnostic {
                        code: "W-REG-001".to_string(),
                        severity: Severity::Warning,
                        message: format!(
                            "Duplicate registry alias \"{}\" at index {i}.",
                            reg.alias
                        ),
                        span: None,
                        suggestion: Some("Use unique aliases for each registry.".to_string()),
                    });
                }
                registries.push(reg);
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    code: "E-REG-003".to_string(),
                    severity: Severity::Error,
                    message: format!("Failed to parse registry entry at index {i}: {e}"),
                    span: None,
                    suggestion: None,
                });
            }
        }
    }

    let has_default = registries.iter().any(|r| r.default_registry);
    if registries.is_empty() || !has_default {
        diagnostics.push(Diagnostic {
            code: "I003".to_string(),
            severity: Severity::Info,
            message: "No registries configured and no default registry set.".to_string(),
            span: None,
            suggestion: Some(
                "Set \"default_registry\": true on one of your registries.".to_string(),
            ),
        });
    }

    (registries, diagnostics)
}

/// Finds the registry matching a given specifier.
///
/// If the specifier starts with `@scope/`, looks for a registry whose
/// `scope_filter` matches that scope. Falls back to the first registry
/// with `default_registry: true`.
pub fn find_registry_for_specifier<'a>(
    specifier: &str,
    registries: &'a [RegistryConfig],
) -> Option<&'a RegistryConfig> {
    // Extract scope from specifier (e.g., "@specforge" from "@specforge/software")
    if let Some(slash_pos) = specifier.find('/') {
        let scope = &specifier[..slash_pos];
        // Look for a registry with a matching scope_filter
        if let Some(reg) = registries.iter().find(|r| {
            r.scope_filter
                .as_deref()
                .is_some_and(|sf| sf == scope)
        }) {
            return Some(reg);
        }
    }

    // Fall back to default registry
    registries.iter().find(|r| r.default_registry)
}
