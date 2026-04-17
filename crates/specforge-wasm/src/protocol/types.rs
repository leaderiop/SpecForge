use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::ProtocolError;

// ── Handshake ──

/// Sent by the host to initiate the protocol handshake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub host_version: String,
    pub supported_categories: Vec<String>,
}

/// Returned by the extension's `__handshake` export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeResponse {
    pub protocol_version: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub contribution_flags: ContributionFlags,
    #[serde(default)]
    pub peer_dependencies: Vec<PeerDependency>,
    #[serde(default)]
    pub sandbox_policy: Option<SandboxPolicy>,
}

/// Declares which contribution categories an extension provides.
/// Controls which `__describe` categories the host will request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionFlags {
    #[serde(default)]
    pub entities: bool,
    #[serde(default)]
    pub validators: bool,
    #[serde(default)]
    pub renderers: bool,
    #[serde(default)]
    pub providers: bool,
    #[serde(default)]
    pub collectors: bool,
    #[serde(default)]
    pub prompts: bool,
    #[serde(default)]
    pub parsers: bool,
    #[serde(default)]
    pub grammars: bool,
    #[serde(default)]
    pub body_parsers: bool,
}

/// Declares a dependency on another extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerDependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub optional: bool,
}

/// Sandbox constraints for extension execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxPolicy {
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    #[serde(default)]
    pub max_execution_ms: Option<u32>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default)]
    pub allowed_output_extensions: Vec<String>,
    #[serde(default)]
    pub network_access: Option<bool>,
    #[serde(default)]
    pub file_system_access: Option<bool>,
}

// ── Describe ──

/// Sent by the host to request a specific contribution category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DescribeRequest {
    pub category: String,
}

/// Returned by the extension's `__describe` export.
/// Items are stored as raw JSON and parsed into typed descriptors via `parse_items`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DescribeResponse {
    pub category: String,
    pub items: serde_json::Value,
}

impl DescribeResponse {
    /// Parse the raw `items` array into a typed `Vec<T>`.
    pub fn parse_items<T: DeserializeOwned>(&self) -> Result<Vec<T>, ProtocolError> {
        serde_json::from_value(self.items.clone()).map_err(ProtocolError::from)
    }
}

// ── Entity Kind Descriptor ──

/// Describes an entity kind contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityKindDescriptor {
    pub name: String,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDescriptor>,
    #[serde(default)]
    pub testable: bool,
    #[serde(default)]
    pub singleton: bool,
    #[serde(default)]
    pub supports_verify: bool,
    #[serde(default)]
    pub incremental: Option<bool>,
    #[serde(default)]
    pub has_body_parser: bool,
    #[serde(default)]
    pub open_fields: bool,
    #[serde(default)]
    pub semantic_token: Option<String>,
    #[serde(default)]
    pub lsp_icon: Option<String>,
    #[serde(default)]
    pub dot_shape: Option<String>,
    #[serde(default)]
    pub dot_color: Option<String>,
    #[serde(default)]
    pub dot_fillcolor: Option<String>,
    #[serde(default)]
    pub verify_kinds: Vec<String>,
}

// ── Field Descriptor ──

/// Describes a field on an entity kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub edge: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub file_reference: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default)]
    pub inverse_of: Option<String>,
}

// ── Edge Type Descriptor ──

/// Describes an edge type (relationship) between entity kinds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeTypeDescriptor {
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub edge_style: Option<String>,
    #[serde(default)]
    pub edge_color: Option<String>,
    #[serde(default)]
    pub edge_arrowhead: Option<String>,
}

// ── Shared Field Descriptor ──

/// A field applied globally to all entity kinds. Structurally identical to FieldDescriptor.
pub type SharedFieldDescriptor = FieldDescriptor;

// ── Entity Enhancement Descriptor ──

/// Describes fields and edge types added to a foreign entity kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityEnhancementDescriptor {
    pub target_kind: String,
    pub source_extension: String,
    #[serde(default)]
    pub fields: Vec<FieldDescriptor>,
    #[serde(default)]
    pub edge_types: Vec<EdgeTypeDescriptor>,
}

// ── Validation Rule Descriptor ──

/// Severity level for validation diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Constraint on a field value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldConstraintDescriptor {
    pub kind: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
}

/// Describes a validation rule contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationRuleDescriptor {
    pub code: String,
    pub severity: ValidationSeverity,
    pub message_template: String,
    pub check: String,
    #[serde(default)]
    pub target_kind: Option<String>,
    #[serde(default)]
    pub edge_type: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub constraint: Option<FieldConstraintDescriptor>,
    #[serde(default)]
    pub wasm_function: Option<String>,
}

// ── Surface Descriptors ──

/// Describes all surface contributions (CLI commands, MCP tools, MCP resources).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceDescriptor {
    #[serde(default)]
    pub commands: Vec<CommandDescriptor>,
    #[serde(default)]
    pub mcp_tools: Vec<McpToolDescriptor>,
    #[serde(default)]
    pub mcp_resources: Vec<McpResourceDescriptor>,
}

/// Describes a CLI command contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandDescriptor {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    pub export: String,
    #[serde(default)]
    pub args: Vec<CommandArgDescriptor>,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

/// Describes an argument to a CLI command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandArgDescriptor {
    pub name: String,
    pub arg_type: CommandArgType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Type of a command argument.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CommandArgType {
    String,
    Path,
    Bool,
    #[serde(rename = "enum")]
    Enum { values: Vec<String> },
    Integer,
}

/// Describes an MCP tool contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub category: Option<String>,
    pub export: String,
    pub input_schema: serde_json::Value,
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

/// Describes an MCP resource contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpResourceDescriptor {
    pub uri_template: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub export: String,
    pub mime_type: String,
    #[serde(default)]
    pub sandbox: Option<SurfaceSandboxOverride>,
}

/// Per-surface sandbox override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceSandboxOverride {
    #[serde(default)]
    pub fs_read: Option<bool>,
    #[serde(default)]
    pub fs_write: Option<bool>,
    #[serde(default)]
    pub network: Option<bool>,
}

// ── Grammar Descriptor ──

/// Describes a grammar contribution for an entity kind's body content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrammarDescriptor {
    pub entity_kind: String,
    pub grammar_wasm_path: String,
    #[serde(default)]
    pub export_name: Option<String>,
}

// ── Body Parser Descriptor ──

/// Describes a body parser contribution for an entity kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BodyParserDescriptor {
    pub entity_kind: String,
    pub export_name: String,
}

// ── Collector Descriptor ──

/// Auto-detection configuration for a collector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoDetectConfig {
    pub file_patterns: Vec<String>,
    #[serde(default)]
    pub env_vars: Vec<String>,
}

/// Describes a test result collector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectorDescriptor {
    pub name: String,
    pub input_formats: Vec<String>,
    pub export: String,
    #[serde(default)]
    pub auto_detect: Option<AutoDetectConfig>,
}

// ── Compiler Pass Descriptor ──

/// Describes a compiler pass contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompilerPassDescriptor {
    pub name: String,
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
}

// ── Feature Flag Descriptor ──

/// Describes a feature flag contributed by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureFlagDescriptor {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_enabled: bool,
}
