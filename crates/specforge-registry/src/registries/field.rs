use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestFieldType {
    String,
    Integer,
    Bool,
    Enum(Vec<String>),
    StringList,
    Reference,
    ReferenceList,
    Block,
}

#[derive(Debug, Clone)]
pub struct FieldRegistryEntry {
    pub kind_name: String,
    pub field_name: String,
    pub field_type: ManifestFieldType,
    pub source_extension: String,
    pub edge: Option<String>,
    pub target_kind: Option<String>,
    pub file_reference: bool,
    pub required: bool,
}

#[derive(Debug, Default)]
pub struct FieldRegistry {
    /// Key: (kind_name, field_name)
    entries: HashMap<(String, String), FieldRegistryEntry>,
}

impl FieldRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, kind_name: &str, field_name: &str) -> Option<&FieldRegistryEntry> {
        self.entries
            .get(&(kind_name.to_string(), field_name.to_string()))
    }

    pub fn contains(&self, kind_name: &str, field_name: &str) -> bool {
        self.entries
            .contains_key(&(kind_name.to_string(), field_name.to_string()))
    }

    pub fn register(&mut self, entry: FieldRegistryEntry) {
        self.entries.insert(
            (entry.kind_name.clone(), entry.field_name.clone()),
            entry,
        );
    }

    pub fn fields_for_kind(&self, kind_name: &str) -> Vec<&FieldRegistryEntry> {
        self.entries
            .values()
            .filter(|e| e.kind_name == kind_name)
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&(String, String), &FieldRegistryEntry)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // B:boot_empty_field_registry — verify unit "FieldRegistry::new() has zero entries"
    #[test]
    fn test_field_registry_new_has_zero_entries() {
        let registry = FieldRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    // B:boot_empty_field_registry — verify unit "no field names recognized before extension loading"
    #[test]
    fn test_no_field_names_recognized_before_extension_loading() {
        let registry = FieldRegistry::new();
        assert!(registry.get("behavior", "contract").is_none());
        assert!(!registry.contains("behavior", "contract"));
        assert!(registry.fields_for_kind("behavior").is_empty());
    }

    // B:boot_empty_field_registry — verify unit "entity title parsed by grammar, not FieldRegistry"
    #[test]
    fn test_entity_title_parsed_by_grammar_not_field_registry() {
        // Title is a grammar-level construct (parsed by tree-sitter), not a field.
        // Even after registering fields for a kind, "title" should not appear
        // as a registered field — it lives in the AST, not the FieldRegistry.
        let mut registry = FieldRegistry::new();
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "contract".to_string(),
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        });
        assert!(registry.get("behavior", "title").is_none());
    }

    // B:boot_empty_field_registry — verify contract "requires/ensures consistency for empty field registry boot"
    #[test]
    fn test_boot_empty_field_registry_contract() {
        // requires: no extensions loaded yet
        let registry = FieldRegistry::new();
        // ensures: empty
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        // ensures: get returns None for arbitrary keys
        assert!(registry.get("behavior", "contract").is_none());
        // ensures: fields_for_kind returns empty
        assert!(registry.fields_for_kind("behavior").is_empty());
        // ensures: iter yields nothing
        assert_eq!(registry.iter().count(), 0);
    }
}
