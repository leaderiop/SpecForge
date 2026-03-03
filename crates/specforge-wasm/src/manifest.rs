use serde::{Deserialize, Serialize};
use specforge_common::{DynamicEdgeType, FieldEnhancement};
use std::collections::HashMap;
use std::path::PathBuf;

/// What capabilities a package contributes to the SpecForge compiler.
///
/// Replaces the single `PluginKind` enum with a multi-capability model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageContributions {
    #[serde(default)]
    pub entities: bool,
    #[serde(default)]
    pub validators: bool,
    #[serde(default)]
    pub generators: bool,
    #[serde(default)]
    pub providers: bool,
}

impl PackageContributions {
    /// Create contributions from a legacy `PluginKind` value (v1 manifest compat).
    pub fn from_kind(kind: PluginKind) -> Self {
        match kind {
            PluginKind::Plugin => Self {
                entities: true,
                validators: true,
                ..Default::default()
            },
            PluginKind::Provider => Self {
                providers: true,
                ..Default::default()
            },
            PluginKind::Generator => Self {
                generators: true,
                ..Default::default()
            },
        }
    }
}

/// The sidecar manifest for a Wasm package (`manifest.json`).
///
/// Contains metadata, capabilities, sandbox policy, and peer dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package name (e.g., `@specforge/hexagonal`).
    pub package: String,
    /// Manifest schema version.
    #[serde(default = "default_manifest_version")]
    pub manifest_version: String,
    /// Plugin kind: plugin, provider, or generator (v1 manifest backward compat).
    #[serde(default)]
    pub kind: PluginKind,
    /// What capabilities this package contributes.
    #[serde(default)]
    pub contributes: PackageContributions,
    /// Path to the `.wasm` binary, relative to the manifest file.
    pub wasm: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Plugin version (semver).
    #[serde(default = "default_version")]
    pub version: String,

    /// Field enhancements contributed to existing entity kinds.
    #[serde(default)]
    pub enhancements: Vec<FieldEnhancement>,
    /// Custom edge types contributed by this plugin.
    #[serde(default)]
    pub dynamic_edge_types: Vec<DynamicEdgeType>,

    /// Entity kinds registered by this plugin (for plugin kind).
    #[serde(default)]
    pub entity_kinds: Vec<WasmEntityKind>,

    /// Provider configuration (for provider kind).
    #[serde(default)]
    pub provider: Option<WasmProviderConfig>,

    /// Generator configuration (for generator kind).
    #[serde(default)]
    pub generator: Option<WasmGeneratorConfig>,

    /// Sandbox policy.
    #[serde(default)]
    pub sandbox: SandboxPolicy,

    /// Peer dependencies on other plugins (package → semver range).
    #[serde(default)]
    pub peer_dependencies: HashMap<String, String>,

    /// Query extension files bundled with this plugin.
    #[serde(default)]
    pub query_extensions: Vec<QueryExtension>,

    /// Resolved absolute path to the manifest file (set at load time, not serialized).
    #[serde(skip)]
    pub manifest_path: PathBuf,

    /// Resolved absolute path to the `.wasm` binary (set at load time, not serialized).
    #[serde(skip)]
    pub wasm_path: PathBuf,
}

/// The kind of Wasm extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    #[default]
    Plugin,
    Provider,
    Generator,
}

/// An entity kind registered by a Wasm plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmEntityKind {
    /// Entity kind name (e.g., `"microservice"`).
    pub name: String,
    /// Whether this entity kind is testable.
    #[serde(default)]
    pub testable: bool,
    /// Expected fields for this entity kind.
    #[serde(default)]
    pub fields: Vec<WasmEntityField>,
    /// Reference target kinds for fields.
    #[serde(default)]
    pub reference_targets: HashMap<String, String>,
}

/// A field definition for a Wasm-registered entity kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmEntityField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
}

/// Provider configuration within a Wasm manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmProviderConfig {
    /// Ref scheme(s) this provider handles (e.g., `["gh"]`).
    pub schemes: Vec<String>,
    /// Valid ref kinds per scheme.
    #[serde(default)]
    pub kinds: HashMap<String, Vec<String>>,
}

/// Generator configuration within a Wasm manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmGeneratorConfig {
    /// Generator name as referenced in `gen` blocks (e.g., `"rust"`).
    pub name: String,
}

/// Sandbox policy for a Wasm plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Maximum memory in bytes (default: 256 MB).
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: u64,
    /// Maximum fuel (instruction budget). 0 = unlimited.
    #[serde(default)]
    pub max_fuel: u64,
    /// Allowed domains for `http_get` host function.
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    /// Whether file emission is allowed.
    #[serde(default = "default_true")]
    pub allow_emit_file: bool,
    /// Whether HTTP access is allowed.
    #[serde(default)]
    pub allow_http: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            max_memory_bytes: default_max_memory(),
            max_fuel: 0,
            allowed_domains: Vec::new(),
            allow_emit_file: true,
            allow_http: false,
        }
    }
}

/// Query extension bundled with a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExtension {
    /// Relative path to the query file.
    pub path: String,
    /// Description of what this query provides.
    #[serde(default)]
    pub description: String,
}

/// Lifecycle states for a loaded Wasm plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLifecycleState {
    /// Manifest loaded, binary not yet loaded.
    Discovered,
    /// Binary loaded into memory, not yet initialized.
    Loading,
    /// Plugin has been initialized (register_entity/register_edge called).
    Initialized,
    /// Plugin is ready for validate/generate calls.
    Ready,
    /// Plugin encountered an error.
    Failed,
}

fn default_manifest_version() -> String {
    "1".to_string()
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_max_memory() -> u64 {
    256 * 1024 * 1024 // 256 MB
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_manifest() {
        let json = r#"{
            "package": "@specforge/test-plugin",
            "wasm": "plugin.wasm"
        }"#;
        let manifest: PackageManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.package, "@specforge/test-plugin");
        assert_eq!(manifest.wasm, "plugin.wasm");
        assert_eq!(manifest.manifest_version, "1");
        assert_eq!(manifest.kind, PluginKind::Plugin);
        assert!(manifest.enhancements.is_empty());
        assert!(manifest.peer_dependencies.is_empty());
        assert_eq!(manifest.sandbox.max_memory_bytes, 256 * 1024 * 1024);
    }

    #[test]
    fn deserialize_full_manifest() {
        let json = r#"{
            "package": "@specforge/hexagonal",
            "manifest_version": "1",
            "kind": "plugin",
            "wasm": "hexagonal.wasm",
            "version": "1.0.0",
            "description": "Hexagonal architecture plugin",
            "sandbox": {
                "max_memory_bytes": 134217728,
                "max_fuel": 1000000,
                "allowed_domains": ["api.example.com"],
                "allow_http": true
            },
            "peer_dependencies": {
                "@specforge/product": ">=0.1.0"
            },
            "entity_kinds": [
                {
                    "name": "adapter",
                    "testable": true,
                    "fields": [{"name": "port", "type": "reference", "required": true}]
                }
            ]
        }"#;
        let manifest: PackageManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.package, "@specforge/hexagonal");
        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.sandbox.allow_http);
        assert_eq!(manifest.sandbox.max_fuel, 1_000_000);
        assert_eq!(manifest.peer_dependencies.len(), 1);
        assert_eq!(manifest.entity_kinds.len(), 1);
        assert!(manifest.entity_kinds[0].testable);
    }

    #[test]
    fn deserialize_generator_manifest() {
        let json = r#"{
            "package": "@specforge/gen-rust",
            "kind": "generator",
            "wasm": "gen-rust.wasm",
            "generator": {
                "name": "rust"
            }
        }"#;
        let manifest: PackageManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.kind, PluginKind::Generator);
        assert_eq!(manifest.generator.as_ref().unwrap().name, "rust");
    }

    #[test]
    fn deserialize_provider_manifest() {
        let json = r#"{
            "package": "@specforge/gh",
            "kind": "provider",
            "wasm": "gh.wasm",
            "provider": {
                "schemes": ["gh"],
                "kinds": {"gh": ["issue", "pr"]}
            }
        }"#;
        let manifest: PackageManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.kind, PluginKind::Provider);
        let provider = manifest.provider.as_ref().unwrap();
        assert_eq!(provider.schemes, vec!["gh"]);
    }

    #[test]
    fn serialize_roundtrip() {
        let manifest = PackageManifest {
            package: "@specforge/test".to_string(),
            manifest_version: "1".to_string(),
            kind: PluginKind::Plugin,
            contributes: PackageContributions::default(),
            wasm: "test.wasm".to_string(),
            description: "Test plugin".to_string(),
            version: "0.1.0".to_string(),
            enhancements: vec![],
            dynamic_edge_types: vec![],
            entity_kinds: vec![],
            provider: None,
            generator: None,
            sandbox: SandboxPolicy::default(),
            peer_dependencies: HashMap::new(),
            query_extensions: vec![],
            manifest_path: PathBuf::new(),
            wasm_path: PathBuf::new(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: PackageManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.package, "@specforge/test");
    }

    #[test]
    fn default_sandbox_policy() {
        let policy = SandboxPolicy::default();
        assert_eq!(policy.max_memory_bytes, 256 * 1024 * 1024);
        assert_eq!(policy.max_fuel, 0);
        assert!(policy.allowed_domains.is_empty());
        assert!(policy.allow_emit_file);
        assert!(!policy.allow_http);
    }
}
