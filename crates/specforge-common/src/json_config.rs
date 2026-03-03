use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{CompilerConfig, CoverageConfig, EnhancementPolicy, EntityKindPolicy, FormatVersion, GenConfig, Module, NamingStyle, ResultStyle};

fn default_version() -> String {
    "1.0".to_string()
}

fn default_spec_root() -> String {
    ".".to_string()
}

/// Provider configuration in `specforge.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonProviderConfig {
    pub package: String,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub id_patterns: HashMap<String, String>,
    /// Pass-through fields for provider-specific config.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Coverage configuration in `specforge.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonCoverageConfig {
    #[serde(default)]
    pub threshold: Option<u32>,
    #[serde(default)]
    pub require_violation_tests: bool,
    #[serde(default)]
    pub fail_on_unknown_ids: bool,
}

/// Code generation configuration per language in `specforge.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonGenConfig {
    pub out: String,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub naming: Option<String>,
    #[serde(default)]
    pub tests: Option<String>,
    /// Pass-through fields for external generators.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// The top-level `specforge.json` configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecForgeJsonConfig {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_spec_root")]
    pub spec_root: String,
    #[serde(default)]
    pub strict: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_prefix: Option<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub providers: HashMap<String, JsonProviderConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub personas: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub surfaces: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_dirs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<JsonCoverageConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(rename = "gen")]
    pub generators: HashMap<String, JsonGenConfig>,
    #[serde(default)]
    pub enhancement_policy: EnhancementPolicy,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub enhancement_overrides: HashMap<String, String>,
    #[serde(default)]
    pub entity_kind_policy: EntityKindPolicy,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub entity_kinds: HashMap<String, String>,
}

impl SpecForgeJsonConfig {
    /// Convert this JSON config to the internal `CompilerConfig` used by the pipeline.
    pub fn to_compiler_config(&self) -> CompilerConfig {
        let version = self
            .version
            .parse::<FormatVersion>()
            .unwrap_or(FormatVersion::CURRENT);

        let plugins: Vec<Module> = self
            .plugins
            .iter()
            .filter_map(|p| Module::from_package_name(p))
            .collect();

        // Wasm package specifiers are plugins NOT matched by Module::from_package_name()
        let wasm_package_specifiers: Vec<String> = self
            .plugins
            .iter()
            .filter(|p| Module::from_package_name(p).is_none())
            .cloned()
            .collect();

        let mut provider_schemes = Vec::new();
        let mut provider_kinds = HashMap::new();
        let mut provider_id_patterns = HashMap::new();

        for (scheme, cfg) in &self.providers {
            provider_schemes.push(scheme.clone());
            if !cfg.kinds.is_empty() {
                provider_kinds.insert(scheme.clone(), cfg.kinds.clone());
            }
            if !cfg.id_patterns.is_empty() {
                provider_id_patterns.insert(scheme.clone(), cfg.id_patterns.clone());
            }
        }

        let personas: Vec<(String, String)> = self
            .personas
            .iter()
            .map(|(id, display)| (id.clone(), display.clone()))
            .collect();

        let surfaces: Vec<(String, String)> = self
            .surfaces
            .iter()
            .map(|(id, display)| (id.clone(), display.clone()))
            .collect();

        let gen_configs: Vec<GenConfig> = self
            .generators
            .iter()
            .map(|(name, cfg)| {
                let result_style = cfg
                    .result
                    .as_deref()
                    .and_then(ResultStyle::from_str_opt)
                    .unwrap_or_default();
                let naming = cfg
                    .naming
                    .as_deref()
                    .and_then(NamingStyle::from_str_opt)
                    .unwrap_or_default();
                let extra: HashMap<String, String> = cfg
                    .extra
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect();
                GenConfig {
                    name: name.clone(),
                    out: cfg.out.clone(),
                    result_style,
                    readonly: cfg.readonly,
                    naming,
                    tests: cfg.tests.clone(),
                    extra,
                }
            })
            .collect();

        let coverage = CoverageConfig {
            threshold: self.coverage.as_ref().and_then(|c| c.threshold),
            require_violation_tests: self.coverage.as_ref().is_some_and(|c| c.require_violation_tests),
            fail_on_unknown_ids: self.coverage.as_ref().is_some_and(|c| c.fail_on_unknown_ids),
            test_dirs: self.test_dirs.clone(),
        };

        CompilerConfig {
            name: self.name.clone(),
            version,
            namespace: self.namespace.clone(),
            display_prefix: self.display_prefix.clone(),
            plugins,
            provider_schemes,
            provider_kinds,
            provider_id_patterns,
            personas,
            surfaces,
            strict: self.strict,
            gen_configs,
            coverage,
            custom_entities: HashMap::new(),
            enhancement_policy: self.enhancement_policy,
            enhancement_overrides: self.enhancement_overrides.clone(),
            wasm_package_specifiers,
            entity_kind_policy: self.entity_kind_policy,
            entity_kind_overrides: self.entity_kinds.clone(),
        }
    }

    /// Create a minimal config suitable for `specforge init`.
    pub fn minimal(name: &str) -> Self {
        Self {
            schema: Some("https://specforge.dev/schema/specforge.json".to_string()),
            name: name.to_string(),
            version: "1.0".to_string(),
            spec_root: ".".to_string(),
            strict: false,
            namespace: None,
            display_prefix: None,
            plugins: Vec::new(),
            providers: HashMap::new(),
            personas: HashMap::new(),
            surfaces: HashMap::new(),
            test_dirs: Vec::new(),
            coverage: None,
            generators: HashMap::new(),
            enhancement_policy: EnhancementPolicy::default(),
            enhancement_overrides: HashMap::new(),
            entity_kind_policy: EntityKindPolicy::default(),
            entity_kinds: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal() {
        let json = r#"{
            "name": "test-project",
            "version": "1.0"
        }"#;
        let config: SpecForgeJsonConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test-project");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.spec_root, ".");
        assert!(!config.strict);
        assert!(config.plugins.is_empty());
    }

    #[test]
    fn deserialize_full() {
        let json = r#"{
            "$schema": "https://specforge.dev/schema/specforge.json",
            "name": "healthcare-platform",
            "version": "1.0",
            "spec_root": "./spec",
            "strict": false,
            "namespace": "@healthcare",
            "display_prefix": "HP",
            "plugins": [
                "@specforge/product",
                "@specforge/governance"
            ],
            "providers": {
                "gh": {
                    "package": "@specforge/gh",
                    "repo": "healthorg/platform",
                    "kinds": ["issue", "pr"],
                    "id_patterns": {
                        "issue": "^\\d+$",
                        "pr": "^\\d+$"
                    }
                }
            },
            "personas": {
                "clinician": "Healthcare Provider",
                "patient": "Patient User"
            },
            "surfaces": {
                "web": "Web Portal",
                "api": "HL7 FHIR API"
            },
            "test_dirs": ["tests/"],
            "gen": {
                "typescript": {
                    "out": "src/generated/",
                    "result": "hex-di",
                    "readonly": true,
                    "naming": "camelCase",
                    "tests": "@specforge/vitest"
                }
            }
        }"#;
        let config: SpecForgeJsonConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "healthcare-platform");
        assert_eq!(config.namespace.as_deref(), Some("@healthcare"));
        assert_eq!(config.plugins.len(), 2);
        assert!(config.providers.contains_key("gh"));
        assert_eq!(config.personas.get("clinician").unwrap(), "Healthcare Provider");
        assert_eq!(config.generators.get("typescript").unwrap().out, "src/generated/");
    }

    #[test]
    fn to_compiler_config_minimal() {
        let json_config = SpecForgeJsonConfig::minimal("test");
        let config = json_config.to_compiler_config();
        assert_eq!(config.name, "test");
        assert_eq!(config.version, FormatVersion::CURRENT);
        assert!(config.plugins.is_empty());
        assert!(!config.strict);
    }

    #[test]
    fn to_compiler_config_with_plugins() {
        let mut json_config = SpecForgeJsonConfig::minimal("test");
        json_config.plugins = vec![
            "@specforge/product".to_string(),
            "@specforge/governance".to_string(),
        ];
        let config = json_config.to_compiler_config();
        assert_eq!(config.plugins.len(), 2);
        assert!(config.has_plugin(Module::Product));
        assert!(config.has_plugin(Module::Governance));
    }

    #[test]
    fn to_compiler_config_with_providers() {
        let mut json_config = SpecForgeJsonConfig::minimal("test");
        let mut id_patterns = HashMap::new();
        id_patterns.insert("issue".to_string(), r"^\d+$".to_string());
        json_config.providers.insert(
            "gh".to_string(),
            JsonProviderConfig {
                package: "@specforge/gh".to_string(),
                repo: Some("org/repo".to_string()),
                kinds: vec!["issue".to_string(), "pr".to_string()],
                id_patterns,
                extra: HashMap::new(),
            },
        );
        let config = json_config.to_compiler_config();
        assert!(config.has_provider_scheme("gh"));
        assert!(config.has_provider_kind("gh", "issue"));
        assert_eq!(config.get_id_pattern("gh", "issue"), Some(r"^\d+$"));
    }

    #[test]
    fn to_compiler_config_with_gen() {
        let mut json_config = SpecForgeJsonConfig::minimal("test");
        json_config.generators.insert(
            "typescript".to_string(),
            JsonGenConfig {
                out: "src/gen/".to_string(),
                result: Some("hex-di".to_string()),
                readonly: true,
                naming: Some("camelCase".to_string()),
                tests: Some("@specforge/vitest".to_string()),
                extra: HashMap::new(),
            },
        );
        let config = json_config.to_compiler_config();
        assert_eq!(config.gen_configs.len(), 1);
        let gen_cfg = &config.gen_configs[0];
        assert_eq!(gen_cfg.name, "typescript");
        assert_eq!(gen_cfg.out, "src/gen/");
        assert_eq!(gen_cfg.result_style, ResultStyle::HexDi);
        assert!(gen_cfg.readonly);
        assert_eq!(gen_cfg.naming, NamingStyle::CamelCase);
        assert_eq!(gen_cfg.tests.as_deref(), Some("@specforge/vitest"));
    }

    #[test]
    fn serialize_minimal_roundtrip() {
        let config = SpecForgeJsonConfig::minimal("test");
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: SpecForgeJsonConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.version, "1.0");
    }

    #[test]
    fn to_compiler_config_with_coverage() {
        let mut json_config = SpecForgeJsonConfig::minimal("test");
        json_config.test_dirs = vec!["tests/".to_string()];
        json_config.coverage = Some(JsonCoverageConfig {
            threshold: Some(80),
            require_violation_tests: true,
            fail_on_unknown_ids: true,
        });
        let config = json_config.to_compiler_config();
        assert_eq!(config.coverage.threshold, Some(80));
        assert!(config.coverage.require_violation_tests);
        assert!(config.coverage.fail_on_unknown_ids);
        assert_eq!(config.coverage.test_dirs, vec!["tests/"]);
    }

    #[test]
    fn to_compiler_config_coverage_defaults() {
        let json_config = SpecForgeJsonConfig::minimal("test");
        let config = json_config.to_compiler_config();
        assert_eq!(config.coverage.threshold, None);
        assert!(!config.coverage.require_violation_tests);
        assert!(!config.coverage.fail_on_unknown_ids);
        assert!(config.coverage.test_dirs.is_empty());
    }

    #[test]
    fn unknown_plugin_ignored() {
        let mut json_config = SpecForgeJsonConfig::minimal("test");
        json_config.plugins = vec!["@specforge/unknown".to_string()];
        let config = json_config.to_compiler_config();
        assert!(config.plugins.is_empty());
    }
}
