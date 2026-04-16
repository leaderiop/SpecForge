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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFieldType {
    String,
    Integer,
    Boolean,
    Enum,
    #[serde(rename = "string_list")]
    StringList,
    Reference,
    #[serde(rename = "reference_list")]
    ReferenceList,
    Block,
}

impl fmt::Display for ModelFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelFieldType::String => write!(f, "string"),
            ModelFieldType::Integer => write!(f, "integer"),
            ModelFieldType::Boolean => write!(f, "boolean"),
            ModelFieldType::Enum => write!(f, "enum"),
            ModelFieldType::StringList => write!(f, "string_list"),
            ModelFieldType::Reference => write!(f, "reference"),
            ModelFieldType::ReferenceList => write!(f, "reference_list"),
            ModelFieldType::Block => write!(f, "block"),
        }
    }
}

impl ModelFieldType {
    pub fn from_schema_str(s: &str) -> Self {
        match s {
            "string" => Self::String,
            "integer" => Self::Integer,
            "boolean" => Self::Boolean,
            "enum" => Self::Enum,
            "string_list" => Self::StringList,
            "reference" => Self::Reference,
            "reference_list" => Self::ReferenceList,
            "block" => Self::Block,
            _ => Self::String, // safe fallback
        }
    }
}

// ---------------------------------------------------------------------------
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
    pub fields: Vec<ModelField>,
    /// Extensions that contribute fields to this entity via entity enhancements.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enhanced_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelField {
    pub name: String,
    pub field_type: ModelFieldType,
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
