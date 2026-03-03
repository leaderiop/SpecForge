use std::collections::HashMap;

use crate::{FormatVersion, Module};

/// Naming convention for generated code identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NamingStyle {
    #[default]
    CamelCase,
    PascalCase,
    SnakeCase,
    KebabCase,
}

impl NamingStyle {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "camelCase" | "camel" => Some(Self::CamelCase),
            "PascalCase" | "pascal" => Some(Self::PascalCase),
            "snake_case" | "snake" => Some(Self::SnakeCase),
            "kebab-case" | "kebab" => Some(Self::KebabCase),
            _ => None,
        }
    }
}

/// How `Result<T, E>` types are rendered in generated TypeScript code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResultStyle {
    /// Import `Result` from hex-di (`import { Result } from 'hex-di'`).
    #[default]
    HexDi,
    /// Inline tagged union: `{ ok: T } | { err: E }`.
    Plain,
    /// Strip the Result wrapper, emit `T` directly.
    Never,
    /// Custom result type import path (e.g., `"@org/result"`).
    Custom(String),
}

impl ResultStyle {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "hex-di" | "hexdi" => Some(Self::HexDi),
            "plain" => Some(Self::Plain),
            "never" => Some(Self::Never),
            other => Some(Self::Custom(other.to_string())),
        }
    }
}

/// Configuration for a single code generator, extracted from a `gen <name> { ... }` block.
#[derive(Debug, Clone)]
pub struct GenConfig {
    /// Generator name (e.g., "typescript", "json-schema").
    pub name: String,
    /// Output directory.
    pub out: String,
    /// How to render Result types.
    pub result_style: ResultStyle,
    /// Whether to add readonly modifiers to fields with @readonly.
    pub readonly: bool,
    /// Naming convention for generated identifiers.
    pub naming: NamingStyle,
    /// Test framework adapter (e.g., "@specforge/vitest").
    pub tests: Option<String>,
    /// Pass-through fields for external generators.
    pub extra: HashMap<String, String>,
}

/// Compiler configuration derived from the `spec` root block.
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Project name from `spec "name"`.
    pub name: String,
    /// Format version from `version "1.0"`.
    pub version: FormatVersion,
    /// Optional namespace for cross-project references (e.g., `"@auth-service"`).
    pub namespace: Option<String>,
    /// Optional display prefix for reports (e.g., `"MS"`). Does not affect entity IDs.
    pub display_prefix: Option<String>,
    /// Installed plugins.
    pub plugins: Vec<Module>,
    /// Registered provider schemes.
    pub provider_schemes: Vec<String>,
    /// Valid kinds per provider scheme (e.g., "gh" → ["issue", "pr"]).
    pub provider_kinds: HashMap<String, Vec<String>>,
    /// Identifier regex patterns per scheme/kind (e.g., "gh" → {"issue" → r"^\d+$"}).
    /// Stored as raw strings; compiled to `Regex` in the resolver.
    pub provider_id_patterns: HashMap<String, HashMap<String, String>>,
    /// Defined personas (id -> display name).
    pub personas: Vec<(String, String)>,
    /// Defined surfaces (id -> display name).
    pub surfaces: Vec<(String, String)>,
    /// Whether --strict mode is enabled (warnings become errors).
    pub strict: bool,
    /// Code generation configurations from `gen <name> { ... }` blocks.
    pub gen_configs: Vec<GenConfig>,
}

impl CompilerConfig {
    /// Create a minimal config with only core module.
    pub fn core_only(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: FormatVersion::CURRENT,
            namespace: None,
            display_prefix: None,
            plugins: Vec::new(),
            provider_schemes: Vec::new(),
            provider_kinds: HashMap::new(),
            provider_id_patterns: HashMap::new(),
            personas: Vec::new(),
            surfaces: Vec::new(),
            strict: false,
            gen_configs: Vec::new(),
        }
    }

    /// Check if a plugin is installed.
    pub fn has_plugin(&self, module: Module) -> bool {
        module == Module::Core || self.plugins.contains(&module)
    }

    /// Check if a persona ID is defined.
    pub fn has_persona(&self, id: &str) -> bool {
        self.personas.iter().any(|(pid, _)| pid == id)
    }

    /// Check if a surface ID is defined.
    pub fn has_surface(&self, id: &str) -> bool {
        self.surfaces.iter().any(|(sid, _)| sid == id)
    }

    /// Check if a provider scheme is registered.
    pub fn has_provider_scheme(&self, scheme: &str) -> bool {
        self.provider_schemes.iter().any(|s| s == scheme)
    }

    /// Check if a provider kind is valid for a given scheme.
    /// Returns true if the scheme has no declared kinds (skip check) or the kind is in the list.
    pub fn has_provider_kind(&self, scheme: &str, kind: &str) -> bool {
        match self.provider_kinds.get(scheme) {
            None => true,          // No kinds registered → skip check
            Some(kinds) if kinds.is_empty() => true, // Empty kinds list → skip check
            Some(kinds) => kinds.iter().any(|k| k == kind),
        }
    }

    /// Get the identifier regex pattern for a scheme/kind pair, if any.
    pub fn get_id_pattern(&self, scheme: &str, kind: &str) -> Option<&str> {
        self.provider_id_patterns
            .get(scheme)
            .and_then(|kinds| kinds.get(kind))
            .map(|s| s.as_str())
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self::core_only("unnamed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_only_config() {
        let config = CompilerConfig::core_only("myproject");
        assert_eq!(config.name, "myproject");
        assert!(config.namespace.is_none());
        assert!(config.display_prefix.is_none());
        assert!(config.has_plugin(Module::Core));
        assert!(!config.has_plugin(Module::Product));
        assert!(!config.has_plugin(Module::Governance));
    }

    #[test]
    fn with_plugins() {
        let mut config = CompilerConfig::core_only("test");
        config.plugins.push(Module::Product);

        assert!(config.has_plugin(Module::Product));
        assert!(!config.has_plugin(Module::Governance));
    }

    #[test]
    fn with_namespace() {
        let mut config = CompilerConfig::core_only("auth-service");
        config.namespace = Some("@auth-service".to_string());
        config.display_prefix = Some("AUTH".to_string());

        assert_eq!(config.namespace.as_deref(), Some("@auth-service"));
        assert_eq!(config.display_prefix.as_deref(), Some("AUTH"));
    }

    #[test]
    fn id_pattern_lookup() {
        let mut config = CompilerConfig::default();
        let mut gh_patterns = HashMap::new();
        gh_patterns.insert("issue".to_string(), r"^\d+$".to_string());
        gh_patterns.insert("pr".to_string(), r"^\d+$".to_string());
        config.provider_id_patterns.insert("gh".to_string(), gh_patterns);

        assert_eq!(config.get_id_pattern("gh", "issue"), Some(r"^\d+$"));
        assert_eq!(config.get_id_pattern("gh", "pr"), Some(r"^\d+$"));
        assert_eq!(config.get_id_pattern("gh", "release"), None);
        assert_eq!(config.get_id_pattern("jira", "issue"), None);
    }

    #[test]
    fn persona_and_surface_lookup() {
        let mut config = CompilerConfig::default();
        config.personas.push(("dev".to_string(), "Developer".to_string()));
        config.surfaces.push(("cli".to_string(), "CLI".to_string()));

        assert!(config.has_persona("dev"));
        assert!(!config.has_persona("admin"));
        assert!(config.has_surface("cli"));
        assert!(!config.has_surface("web"));
    }
}
