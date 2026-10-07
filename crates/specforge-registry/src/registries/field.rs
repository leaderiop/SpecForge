use std::collections::HashMap;

use specforge_protocol_types::FieldDescriptor;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ManifestFieldType {
    #[default]
    String,
    Integer,
    Bool,
    Enum(Vec<String>),
    StringList,
    Reference,
    ReferenceList,
    Block,
}

/// An enum field's values come with the field, not its type name, so the
/// conversion leaves them empty.
impl From<specforge_protocol_types::FieldType> for ManifestFieldType {
    fn from(t: specforge_protocol_types::FieldType) -> Self {
        use specforge_protocol_types::FieldType as T;
        match t {
            T::String => Self::String,
            T::Integer => Self::Integer,
            T::Bool => Self::Bool,
            T::Enum => Self::Enum(Vec::new()),
            T::StringList => Self::StringList,
            T::Reference => Self::Reference,
            T::ReferenceList => Self::ReferenceList,
            T::Block => Self::Block,
        }
    }
}

/// One registered field of one kind: what its extension declared, and
/// what the registry build resolved of it. Everything else is read from
/// `declared` (`entry.declared.name`, `entry.declared.edge`,
/// `entry.declared.default_value`, ...).
#[derive(Debug, Clone, Default)]
pub struct FieldRegistryEntry {
    pub kind_name: String,
    /// The extension the field is the kind's through: its own, or the
    /// owner an enhancement names.
    pub source_extension: String,
    /// The declared type, parsed (W019 when unknown, and not registered);
    /// an enum's values are the declared `enum_values`.
    pub field_type: ManifestFieldType,
    /// What the prove pass reads the field as, when anything (ADR 0009, A):
    /// the declared role, parsed (W021 when unknown).
    pub proof_role: Option<ProofRole>,
    /// The field as its extension declared it.
    pub declared: FieldDescriptor,
}

/// A field's role in the prove pass, declared by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofRole {
    /// A fact the solver assumes; the bounds must be consistent (E046).
    Bound,
    /// A statement that must follow from the bounds (W139 when it does not).
    Claim,
}

impl ProofRole {
    /// The role a manifest names (`bound` or `claim`); None for any other.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "bound" => Some(Self::Bound),
            "claim" => Some(Self::Claim),
            _ => None,
        }
    }
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
        let kind_map = self.entries.entry(entry.kind_name.clone()).or_default();
        if !kind_map.contains_key(&entry.declared.name) {
            self.count += 1;
        }
        kind_map.insert(entry.declared.name.clone(), entry);
    }

    pub fn fields_for_kind(&self, kind_name: &str) -> Vec<&FieldRegistryEntry> {
        self.entries
            .get(kind_name)
            .map(|fields| fields.values().collect())
            .unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str, &FieldRegistryEntry)> {
        self.entries.iter().flat_map(|(kind, fields)| {
            fields
                .iter()
                .map(move |(field, entry)| (kind.as_str(), field.as_str(), entry))
        })
    }

    /// The names of the fields declared `file_reference`, sorted and
    /// unique: the fields whose values the checks read as files (E016).
    pub fn file_reference_fields(&self) -> std::collections::BTreeSet<&str> {
        self.iter()
            .filter(|(_, _, entry)| entry.declared.file_reference)
            .map(|(_, field, _)| field)
            .collect()
    }

    /// Reference fields whose target kind `kinds` doesn't declare, as
    /// (kind, field) -> target kind: the kind's extension isn't enabled
    /// (e.g. software's `behavior.features` without @specforge/product).
    pub fn absent_reference_targets(
        &self,
        kinds: &crate::KindRegistry,
    ) -> HashMap<(String, String), String> {
        self.iter()
            .filter_map(|(kind, field, entry)| {
                let target = entry.declared.target_kind.as_ref()?;
                (!kinds.contains(target))
                    .then(|| ((kind.to_string(), field.to_string()), target.clone()))
            })
            .collect()
    }

    pub fn bidirectional_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (_kind, field_name, entry) in self.iter() {
            if let Some(ref inverse) = entry.declared.inverse_of {
                let a = field_name.to_string();
                let b = inverse.clone();
                let key = if a < b {
                    (a.clone(), b.clone())
                } else {
                    (b.clone(), a.clone())
                };
                if seen.insert(key) {
                    pairs.push((a, b));
                }
            }
        }
        pairs
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
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "contract".to_string(),
                ..Default::default()
            },
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

    #[test]
    fn file_reference_fields_are_the_declared_ones_sorted_once() {
        let mut registry = FieldRegistry::new();
        for (kind, field, file_reference) in [
            ("gadget", "notes", true),
            ("gadget", "docs", true),
            ("gizmo", "docs", true),
            ("gizmo", "title_text", false),
        ] {
            registry.register(FieldRegistryEntry {
                kind_name: kind.to_string(),
                field_type: ManifestFieldType::StringList,
                source_extension: "@test/files".to_string(),
                proof_role: None,
                declared: specforge_protocol_types::FieldDescriptor {
                    name: field.to_string(),
                    file_reference,
                    ..Default::default()
                },
            });
        }
        let names: Vec<&str> = registry.file_reference_fields().into_iter().collect();
        assert_eq!(names, ["docs", "notes"]);
    }

    // Zero-allocation lookup: get() and contains() accept &str without String allocation
    #[test]
    fn test_get_and_contains_accept_str_refs() {
        let mut registry = FieldRegistry::new();
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "contract".to_string(),
                ..Default::default()
            },
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
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "contract".to_string(),
                ..Default::default()
            },
        };
        registry.register(entry.clone());
        assert_eq!(registry.len(), 1);

        // Re-registering the same (kind, field) should not increase count
        let entry2 = FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "contract".to_string(),
                description: Some("updated".to_string()),
                ..Default::default()
            },
        };
        registry.register(entry2);
        assert_eq!(registry.len(), 1);

        // Different field, same kind
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "status".to_string(),
                ..Default::default()
            },
        });
        assert_eq!(registry.len(), 2);
    }

    // Verify iter yields all registered entries
    #[test]
    fn test_iter_yields_all_entries() {
        let mut registry = FieldRegistry::new();
        registry.register(FieldRegistryEntry {
            kind_name: "behavior".to_string(),
            field_type: ManifestFieldType::String,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "contract".to_string(),
                ..Default::default()
            },
        });
        registry.register(FieldRegistryEntry {
            kind_name: "event".to_string(),
            field_type: ManifestFieldType::Block,
            source_extension: "@specforge/software".to_string(),
            proof_role: None,
            declared: specforge_protocol_types::FieldDescriptor {
                name: "payload".to_string(),
                ..Default::default()
            },
        });

        let items: Vec<_> = registry.iter().collect();
        assert_eq!(items.len(), 2);
        // Each item is (&str, &str, &FieldRegistryEntry)
        assert!(
            items
                .iter()
                .any(|(k, f, _)| *k == "behavior" && *f == "contract")
        );
        assert!(
            items
                .iter()
                .any(|(k, f, _)| *k == "event" && *f == "payload")
        );
    }
}
