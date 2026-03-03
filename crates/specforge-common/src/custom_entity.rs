use serde::{Deserialize, Serialize};

/// A custom entity type defined via a `define` block in a `.spec` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEntityDef {
    /// Name of the custom entity type (e.g., `risk_register`).
    pub name: String,
    /// Whether instances of this entity type are testable.
    pub testable: bool,
    /// Field definitions for this custom entity type.
    pub fields: Vec<CustomFieldDef>,
}

/// A field definition within a custom entity type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFieldDef {
    /// Field name (e.g., `severity`).
    pub name: String,
    /// Field type.
    pub field_type: CustomFieldType,
    /// Whether this field is required.
    pub required: bool,
}

/// Supported field types for custom entity definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomFieldType {
    /// A string value.
    String,
    /// An integer value.
    Integer,
    /// A boolean value.
    Bool,
    /// An enum with allowed values.
    Enum(Vec<String>),
    /// A reference to another entity.
    Reference,
    /// A list of references.
    ReferenceList,
    /// A list of strings.
    StringList,
}

impl CustomFieldType {
    /// Parse a field type string from the DSL.
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "string" => Some(Self::String),
            "integer" | "int" => Some(Self::Integer),
            "bool" | "boolean" => Some(Self::Bool),
            "reference" | "ref" => Some(Self::Reference),
            "reference_list" | "ref_list" => Some(Self::ReferenceList),
            "string_list" => Some(Self::StringList),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_field_type_parsing() {
        assert_eq!(CustomFieldType::from_str_opt("string"), Some(CustomFieldType::String));
        assert_eq!(CustomFieldType::from_str_opt("integer"), Some(CustomFieldType::Integer));
        assert_eq!(CustomFieldType::from_str_opt("int"), Some(CustomFieldType::Integer));
        assert_eq!(CustomFieldType::from_str_opt("bool"), Some(CustomFieldType::Bool));
        assert_eq!(CustomFieldType::from_str_opt("reference"), Some(CustomFieldType::Reference));
        assert_eq!(CustomFieldType::from_str_opt("ref"), Some(CustomFieldType::Reference));
        assert_eq!(CustomFieldType::from_str_opt("unknown"), None);
    }
}
