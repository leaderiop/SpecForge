use crate::{KindRegistry, ManifestV2};
use specforge_common::{Diagnostic, Severity};

/// Provider configuration from specforge.json.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub scheme: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
}

/// Entry in the scheme registry mapping a URI scheme to a provider.
#[derive(Debug, Clone)]
pub struct SchemeRegistryEntry {
    pub scheme: String,
    pub provider_name: String,
    pub extension_name: String,
}

/// Registry of URI schemes to provider mappings.
#[derive(Debug, Default)]
pub struct ProviderSchemeRegistry {
    pub entries: Vec<SchemeRegistryEntry>,
}

impl ProviderSchemeRegistry {
    pub fn find_by_scheme(&self, scheme: &str) -> Option<&SchemeRegistryEntry> {
        self.entries.iter().find(|e| e.scheme == scheme)
    }
}

/// Parse provider configurations from the specforge.json `providers` array.
pub fn load_provider_configurations(
    config: &serde_json::Value,
) -> (Vec<ProviderConfig>, Vec<Diagnostic>) {
    let mut providers = Vec::new();
    let mut diagnostics = Vec::new();

    let arr = match config.get("providers").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return (providers, diagnostics),
    };

    for (i, entry) in arr.iter().enumerate() {
        let name = match entry.get("alias").or_else(|| entry.get("name")).and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                diagnostics.push(Diagnostic {
                    code: "W032".to_string(),
                    severity: Severity::Warning,
                    message: format!("providers[{}]: missing 'alias' or 'name' field", i),
                    span: None,
                    suggestion: Some(
                        "add an 'alias' field to the provider entry".to_string(),
                    ),
                });
                continue;
            }
        };

        let scheme = entry
            .get("scheme")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if scheme.is_empty() {
            diagnostics.push(Diagnostic {
                code: "W032".to_string(),
                severity: Severity::Warning,
                message: format!("providers[{}] '{}': missing 'scheme' field", i, name),
                span: None,
                suggestion: Some(
                    "add a 'scheme' field (e.g., \"gh\", \"jira\")".to_string(),
                ),
            });
            continue;
        }

        let base_url = entry
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let api_key_env = entry
            .get("apiKeyEnv")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        providers.push(ProviderConfig {
            name,
            scheme,
            base_url,
            api_key_env,
        });
    }

    (providers, diagnostics)
}

/// Register provider schemes from provider configs + extension manifests.
/// Duplicate schemes across extensions produce E033.
pub fn register_provider_schemes(
    providers: &[ProviderConfig],
    manifests: &[(String, ManifestV2)],
) -> (ProviderSchemeRegistry, Vec<Diagnostic>) {
    let mut registry = ProviderSchemeRegistry::default();
    let mut diagnostics = Vec::new();
    let mut seen_schemes: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    // Register from manifests that contribute providers
    for (ext_name, manifest) in manifests {
        if !manifest.contributes.providers {
            continue;
        }

        // Find matching provider config
        for provider in providers {
            if seen_schemes.contains_key(&provider.scheme) {
                let existing_ext = &seen_schemes[&provider.scheme];
                if existing_ext != ext_name {
                    diagnostics.push(Diagnostic {
                        code: "E033".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "scheme '{}' registered by extension '{}' conflicts with '{}'",
                            provider.scheme, ext_name, existing_ext
                        ),
                        span: None,
                        suggestion: Some(
                            "use distinct schemes for each provider extension".to_string(),
                        ),
                    });
                }
                continue;
            }

            seen_schemes.insert(provider.scheme.clone(), ext_name.clone());
            registry.entries.push(SchemeRegistryEntry {
                scheme: provider.scheme.clone(),
                provider_name: provider.name.clone(),
                extension_name: ext_name.clone(),
            });
        }
    }

    // Warn about providers without matching manifests
    for provider in providers {
        let has_manifest = manifests
            .iter()
            .any(|(_, m)| m.contributes.providers);
        if !has_manifest {
            diagnostics.push(Diagnostic {
                code: "W033".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "provider '{}' configured but no extension contributes providers",
                    provider.name
                ),
                span: None,
                suggestion: Some(
                    "install an extension that contributes providers".to_string(),
                ),
            });
            break;
        }
    }

    (registry, diagnostics)
}

/// Validate a provider reference (scheme:target) against the registry.
pub fn validate_provider_ref(
    scheme: &str,
    target: &str,
    registry: &ProviderSchemeRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if registry.find_by_scheme(scheme).is_none() {
        diagnostics.push(Diagnostic {
            code: "E034".to_string(),
            severity: Severity::Error,
            message: format!("unknown provider scheme '{}' in ref '{}:{}'", scheme, scheme, target),
            span: None,
            suggestion: Some(format!(
                "configure a provider for scheme '{}' in specforge.json",
                scheme
            )),
        });
    }

    diagnostics
}

/// Validate the format of a ref target string.
pub fn validate_ref_target_format(target: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if target.is_empty() {
        diagnostics.push(Diagnostic {
            code: "W034".to_string(),
            severity: Severity::Warning,
            message: "ref target is empty".to_string(),
            span: None,
            suggestion: Some("provide a non-empty ref target (e.g., \"42\", \"PROJ-123\")".to_string()),
        });
        return diagnostics;
    }

    // Check for obviously invalid characters
    if target.contains(|c: char| c.is_control()) {
        diagnostics.push(Diagnostic {
            code: "W034".to_string(),
            severity: Severity::Warning,
            message: format!("ref target '{}' contains control characters", target),
            span: None,
            suggestion: Some("remove control characters from the ref target".to_string()),
        });
    }

    diagnostics
}

/// Validate that provider-referenced entity kinds exist in the KindRegistry.
pub fn validate_provider_kinds(
    providers: &[ProviderConfig],
    _kind_reg: &KindRegistry,
) -> Vec<Diagnostic> {
    let diagnostics = Vec::new();

    // Soft validation: providers don't necessarily reference entity kinds.
    // Future: if providers declare kind-scoped routes, validate against kind_reg.
    let _ = providers;

    diagnostics
}

/// Load extension manifests from a directory.
/// Each .json file in the directory is parsed as a ManifestV2.
pub fn load_extension_manifests(
    dir: &std::path::Path,
) -> (Vec<ManifestV2>, Vec<Diagnostic>) {
    let mut manifests = Vec::new();
    let mut diagnostics = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return (manifests, diagnostics),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<ManifestV2>(&content) {
                    Ok(m) => manifests.push(m),
                    Err(e) => {
                        diagnostics.push(Diagnostic {
                            code: "E030".to_string(),
                            severity: Severity::Error,
                            message: format!(
                                "malformed manifest at '{}': {}",
                                path.display(),
                                e
                            ),
                            span: None,
                            suggestion: Some(
                                "check the manifest JSON syntax".to_string(),
                            ),
                        });
                    }
                },
                Err(e) => {
                    diagnostics.push(Diagnostic {
                        code: "E030".to_string(),
                        severity: Severity::Error,
                        message: format!(
                            "cannot read manifest at '{}': {}",
                            path.display(),
                            e
                        ),
                        span: None,
                        suggestion: None,
                    });
                }
            }
        }
    }

    (manifests, diagnostics)
}

/// Register entity types from manifests into registries.
/// Delegates to populate_registries under the hood.
pub fn register_extension_entity_types(
    manifests: &[ManifestV2],
) -> (
    KindRegistry,
    crate::FieldRegistry,
    crate::EdgeRegistry,
    Vec<Diagnostic>,
) {
    crate::populate_registries(manifests)
}
