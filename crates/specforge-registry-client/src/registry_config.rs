use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::PackageName;
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
            diagnostics.push(
                Diagnostic::new(
                    codes::E067,
                    format!("Failed to parse registry config JSON: {e}"),
                )
                .with_suggestion("Ensure the configuration is valid JSON.".to_string()),
            );
            return (Vec::new(), diagnostics);
        }
    };

    let registries_value = match parsed.get("registries") {
        Some(v) => v,
        None => {
            diagnostics.push(
                Diagnostic::new(
                    codes::I003,
                    "No registries configured and no default registry set.".to_string(),
                )
                .with_suggestion("Add a \"registries\" array to your configuration.".to_string()),
            );
            return (Vec::new(), diagnostics);
        }
    };

    let registries_array = match registries_value.as_array() {
        Some(arr) => arr,
        None => {
            diagnostics.push(Diagnostic::new(
                codes::E067,
                "\"registries\" must be a JSON array.".to_string(),
            ));
            return (Vec::new(), diagnostics);
        }
    };

    let mut registries = Vec::new();
    let mut seen_aliases = std::collections::HashSet::new();

    for (i, entry) in registries_array.iter().enumerate() {
        match serde_json::from_value::<RegistryConfig>(entry.clone()) {
            Ok(reg) => {
                if !seen_aliases.insert(reg.alias.clone()) {
                    diagnostics.push(
                        Diagnostic::new(
                            codes::W140,
                            format!("Duplicate registry alias \"{}\" at index {i}.", reg.alias),
                        )
                        .with_suggestion("Use unique aliases for each registry.".to_string()),
                    );
                }
                registries.push(reg);
            }
            Err(e) => {
                diagnostics.push(Diagnostic::new(
                    codes::E067,
                    format!("Failed to parse registry entry at index {i}: {e}"),
                ));
            }
        }
    }

    let has_default = registries.iter().any(|r| r.default_registry);
    if registries.is_empty() || !has_default {
        diagnostics.push(
            Diagnostic::new(
                codes::I003,
                "No registries configured and no default registry set.".to_string(),
            )
            .with_suggestion(
                "Set \"default_registry\": true on one of your registries.".to_string(),
            ),
        );
    }

    (registries, diagnostics)
}

/// Finds the registry that serves `name`.
///
/// For a scoped name, a registry whose `scope_filter` is its scope;
/// otherwise the first registry with `default_registry: true`.
pub fn find_registry_for<'a>(
    name: &PackageName,
    registries: &'a [RegistryConfig],
) -> Option<&'a RegistryConfig> {
    if let Some(scope) = name.scope()
        && let Some(registry) = registries
            .iter()
            .find(|r| r.scope_filter.as_deref() == Some(scope))
    {
        return Some(registry);
    }
    registries.iter().find(|r| r.default_registry)
}
