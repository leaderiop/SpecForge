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
    pub description: Option<String>,
    pub field_type: ManifestFieldType,
    pub source_extension: String,
    pub edge: Option<String>,
    pub target_kind: Option<String>,
    pub file_reference: bool,
    pub required: bool,
}

#[derive(Debug, Default)]
pub struct FieldRegistry {
    /// Two-level map: kind_name -> field_name -> entry.
    /// Using nested `HashMap<String, _>` instead of `HashMap<(String, String), _>`
    /// so lookups can accept `&str` directly (via `Borrow<str>` on `String`),
    /// avoiding per-call `to_string()` allocations.
    entries: HashMap<String, HashMap<String, FieldRegistryEntry>>,
    /// Total number of registered fields (cached for O(1) len).
    count: usize,
}

impl FieldRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            count: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Look up a field entry by kind and field name.
    /// Zero-allocation: accepts `&str` without converting to `String`.
    pub fn get(&self, kind_name: &str, field_name: &str) -> Option<&FieldRegistryEntry> {
        self.entries.get(kind_name)?.get(field_name)
    }

    /// Check if a field is registered for the given kind.
    /// Zero-allocation: accepts `&str` without converting to `String`.
    pub fn contains(&self, kind_name: &str, field_name: &str) -> bool {
        self.entries
            .get(kind_name)
            .is_some_and(|fields| fields.contains_key(field_name))
    }

    pub fn register(&mut self, entry: FieldRegistryEntry) {
        let kind_map = self.entries
            .entry(entry.kind_name.clone())
            .or_default();
        if !kind_map.contains_key(&entry.field_name) {
            self.count += 1;
        }
        kind_map.insert(entry.field_name.clone(), entry);
    }

    pub fn fields_for_kind(&self, kind_name: &str) -> Vec<&FieldRegistryEntry> {
        self.entries
            .get(kind_name)
            .map(|fields| fields.values().collect())
            .unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &FieldRegistryEntry)> {
        self.entries.iter().flat_map(|(kind, fields)| {
            fields.iter().map(move |(field, entry)| {
                (kind.as_str(), field.as_str(), entry)
            })
        })
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
            description: None,
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

    // Zero-allocation lookup: get() and contains() accept &str without String allocation
    #[test]
    fn test_get_and_contains_accept_str_refs() {
        let mut registry = FieldRegistry::new();
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "contract".to_string(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        });

        // These calls should not allocate — they take &str and use HashMap<String,_>::get(&str)
        let kind: &str = "behavior";
        let field: &str = "contract";
        assert!(registry.get(kind, field).is_some());
        assert!(registry.contains(kind, field));
        assert!(registry.get(kind, "nonexistent").is_none());
        assert!(!registry.contains("other_kind", field));
    }

    // Verify len() is correctly maintained across register and re-register
    #[test]
    fn test_len_tracks_unique_entries() {
        let mut registry = FieldRegistry::new();
        let entry = FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "contract".to_string(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        };
        registry.register(entry.clone());
        assert_eq!(registry.len(), 1);

        // Re-registering the same (kind, field) should not increase count
        let entry2 = FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "contract".to_string(),
            description: Some("updated".to_string()),
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        };
        registry.register(entry2);
        assert_eq!(registry.len(), 1);

        // Different field, same kind
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "status".to_string(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        });
        assert_eq!(registry.len(), 2);
    }

    // Verify iter yields all registered entries
    #[test]
    fn test_iter_yields_all_entries() {
        let mut registry = FieldRegistry::new();
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_name: "contract".to_string(),
            description: None,
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        });
        registry.register(FieldRegistryEntry {
            kind_name: "event".to_string(),
            field_name: "payload".to_string(),
            description: None,
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            edge: None,
            target_kind: None,
            file_reference: false,
            required: false,
        });

        let items: Vec<_> = registry.iter().collect();
        assert_eq!(items.len(), 2);
        // Each item is (&str, &str, &FieldRegistryEntry)
        assert!(items.iter().any(|(k, f, _)| *k == "behavior" && *f == "contract"));
        assert!(items.iter().any(|(k, f, _)| *k == "event" && *f == "payload"));
    }
}
