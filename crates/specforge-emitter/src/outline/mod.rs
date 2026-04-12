mod build;
mod dot;
mod json;
mod markdown;
mod mermaid;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineFormat {
    #[default]
    Markdown,
    Mermaid,
    Dot,
    Json,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineDetail {
    None,
    #[default]
    Keys,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    /// Explicitly declared in peerDependencies
    Direct,
    /// Transitive AND the extension references kinds from the target
    Effective,
    /// Pure transitive — no direct kind references
    Transitive,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyDepth {
    /// Only explicitly declared peer_dependencies
    #[default]
    Direct,
    /// Direct + transitive deps that reference kinds from the target extension
    Effective,
    /// All transitive deps, including unused
    Full,
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct OutlineOptions {
    pub format: OutlineFormat,
    pub detail: OutlineDetail,
    pub deps: DependencyDepth,
}

// ---------------------------------------------------------------------------
// Core IR types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineIntermediate {
    pub extensions: Vec<OutlineExtension>,
    pub dependencies: Vec<OutlineDependency>,
    pub enhancements: Vec<OutlineEnhancement>,
    pub cross_edges: Vec<OutlineCrossEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineExtension {
    pub name: String,
    pub version: String,
    pub entity_kinds: Vec<OutlineEntityKind>,
    pub edge_types: Vec<OutlineEdgeType>,
    pub validation_rules: Vec<OutlineValidationRule>,
    pub contributes: OutlineContributes,
    pub verify_kinds: Vec<String>,
    pub surface_counts: OutlineSurfaceCounts,
    pub shared_fields: Vec<OutlineSharedField>,
    pub collector_count: usize,
    pub grammar_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineEntityKind {
    pub name: String,
    pub keyword: String,
    pub testable: bool,
    pub field_count: usize,
    pub fields: Vec<OutlineField>,
    pub enhanced_by: Vec<OutlineFieldAttribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub source_extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineFieldAttribution {
    pub source_extension: String,
    pub field_count: usize,
    pub field_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineEdgeType {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineValidationRule {
    pub code: String,
    pub severity: String,
    pub check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kind: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutlineContributes {
    pub entities: bool,
    pub validators: bool,
    pub renderers: bool,
    pub providers: bool,
    pub collectors: bool,
    pub prompts: bool,
    pub parsers: bool,
    pub grammars: bool,
    pub body_parsers: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutlineSurfaceCounts {
    pub cli_commands: usize,
    pub mcp_tools: usize,
    pub mcp_resources: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineSharedField {
    pub name: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineDependency {
    pub from: String,
    pub to: String,
    pub version: String,
    pub optional: bool,
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineEnhancement {
    pub enhancer: String,
    pub owner: String,
    pub target_kind: String,
    pub field_count: usize,
    pub field_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutlineCrossEdge {
    pub edge_label: String,
    pub owner_extension: String,
    pub source_kind: String,
    pub target_kind: String,
    pub target_extension: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub use build::OutlineIntermediate_from_manifests;

pub fn filter_dependencies(deps: &[OutlineDependency], depth: DependencyDepth) -> Vec<&OutlineDependency> {
    match depth {
        DependencyDepth::Direct => deps.iter().filter(|d| d.kind == DependencyKind::Direct).collect(),
        DependencyDepth::Effective => deps.iter().filter(|d| d.kind != DependencyKind::Transitive).collect(),
        DependencyDepth::Full => deps.iter().collect(),
    }
}

pub fn render(outline: &OutlineIntermediate, options: &OutlineOptions) -> String {
    match options.format {
        OutlineFormat::Markdown => markdown::render_markdown(outline, options),
        OutlineFormat::Mermaid => mermaid::render_mermaid(outline, options),
        OutlineFormat::Dot => dot::render_dot(outline, options),
        OutlineFormat::Json => json::render_json(outline, options),
    }
}
