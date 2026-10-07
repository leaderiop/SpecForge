mod build;
mod cardinality;
mod dbml;
mod dot;
mod filter;
mod json;
mod markdown;
mod mermaid;

use std::fmt;

use serde::{Deserialize, Serialize};
use specforge_registry::FieldType;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    #[default]
    Markdown,
    Mermaid,
    Dot,
    Json,
    Dbml,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    #[default]
    Extension,
    None,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldLevel {
    None,
    #[default]
    Keys,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    #[serde(rename = "1:1")]
    OneToOne,
    #[serde(rename = "1:N")]
    OneToMany,
    #[serde(rename = "N:1")]
    ManyToOne,
    #[serde(rename = "N:M")]
    ManyToMany,
}

impl fmt::Display for Cardinality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cardinality::OneToOne => write!(f, "1:1"),
            Cardinality::OneToMany => write!(f, "1:N"),
            Cardinality::ManyToOne => write!(f, "N:1"),
            Cardinality::ManyToMany => write!(f, "N:M"),
        }
    }
}

// Options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ModelOptions {
    pub format: ModelFormat,
    pub group_by: GroupBy,
    pub fields: FieldLevel,
    pub extension_filter: Option<String>,
    pub kind_filter: Option<Vec<String>>,
    pub root: Option<String>,
    pub depth: Option<usize>,
}

// ---------------------------------------------------------------------------
// Core IR types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIntermediate {
    pub model_version: String,
    pub extensions: Vec<ModelExtension>,
    pub entities: Vec<ModelEntity>,
    pub relationships: Vec<ModelRelationship>,
    /// Maps edge label -> declaring extension name. Used for accurate
    /// extension edge counts after filtering. Not serialized to output.
    #[serde(skip)]
    pub edge_type_owners: Vec<(String, String)>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ModelEntity {
    pub name: String,
    pub extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Registry-declared DOT color; renderers fall back to the extension
    /// palette when absent (C13-03).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dot_color: Option<String>,
    pub fields: Vec<ModelField>,
    /// Extensions that contribute fields to this entity via entity enhancements.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enhanced_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    pub is_primary_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<String>,
    /// Internal: edge label from SchemaField.edge, used for cardinality inference.
    /// Skipped in serialization.
    #[serde(skip)]
    pub edge_label: Option<String>,
    /// Extension that contributed this field, set only when different from the entity's
    /// owning extension (i.e., the field comes from an entity enhancement).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributed_by: Option<String>,
    /// Contribution info: "EdgeLabel -> target_kind" for reference fields with edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contribution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRelationship {
    pub name: String,
    pub source: String,
    pub target: String,
    pub cardinality: Cardinality,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExtension {
    pub name: String,
    pub version: String,
    pub entity_count: usize,
    pub edge_count: usize,
    /// The colour the extension declares for diagrams (`theme_color`). The
    /// schema does not carry it: [`ModelIntermediate::with_theme_colors`]
    /// sets it from the declarations. Not serialized.
    #[serde(skip)]
    pub color: Option<String>,
}

impl ModelIntermediate {
    /// Each extension's declared `theme_color`, from its handshake.
    pub fn with_theme_colors(
        mut self,
        declarations: &[specforge_protocol_types::ExtensionDeclaration],
    ) -> Self {
        for ext in &mut self.extensions {
            ext.color = declarations
                .iter()
                .find(|d| d.name() == ext.name)
                .and_then(|d| d.handshake.theme_color.clone());
        }
        self
    }

    /// The colour `extension` is drawn in.
    pub fn extension_color(&self, extension: &str) -> &str {
        crate::diagram::theme_color(
            self.extensions
                .iter()
                .find(|e| e.name == extension)
                .and_then(|e| e.color.as_deref()),
        )
    }
}

// ---------------------------------------------------------------------------
// Public API: build + filter + render
// ---------------------------------------------------------------------------

pub use build::ModelIntermediate_from_schema;
pub use filter::{filter_entities, filter_fields};

pub fn render(model: &ModelIntermediate, options: &ModelOptions) -> String {
    match options.format {
        ModelFormat::Markdown => markdown::render_markdown(model, options),
        ModelFormat::Mermaid => mermaid::render_mermaid(model, options),
        ModelFormat::Dot => dot::render_dot(model, options),
        ModelFormat::Json => json::render_json(model),
        ModelFormat::Dbml => dbml::render_dbml(model, options),
    }
}
