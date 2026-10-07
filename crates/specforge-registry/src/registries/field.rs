use std::collections::HashMap;

use specforge_protocol_types::{FieldDescriptor, FieldType, ProofRole};

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
impl From<FieldType> for ManifestFieldType {
    fn from(t: FieldType) -> Self {
        use FieldType as T;
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

/// A declared field type this host does not read. The registry build
/// reports it as W019 and registers nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFieldType(pub String);

/// One registered field of one kind: the descriptor its extension declared,
/// the kind it is registered on and the extension it is the kind's through
/// (its own, or the owner an enhancement names).
///
/// [`FieldRegistryEntry::new`] is the only way to make one, so an entry's
/// type, enum values and proof role are always its declaration's, read once.
/// There is no second copy to disagree with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRegistryEntry {
    kind_name: String,
    source_extension: String,
    field_type: ManifestFieldType,
    proof_role: Option<ProofRole>,
    declared: FieldDescriptor,
}

impl FieldRegistryEntry {
    /// `declared`, registered on `kind_name` through `source_extension`.
    ///
    /// The declared type must be a name the host reads (canonical or an
    /// accepted older spelling), else `Err` (W019 at the registry build). The
    /// entry's descriptor names the type canonically (`"boolean"` becomes
    /// `"bool"`), so every reader of `declared()` sees the one name. A
    /// `proof_role` that is not `bound` or `claim` is read as none; the
    /// registry build reports it (W021).
    pub fn new(
        kind_name: &str,
        source_extension: &str,
        mut declared: FieldDescriptor,
    ) -> Result<Self, UnknownFieldType> {
        let parsed = FieldType::parse(&declared.field_type)
            .ok_or_else(|| UnknownFieldType(declared.field_type.clone()))?;
        declared.field_type = parsed.as_str().to_string();
        let field_type = match ManifestFieldType::from(parsed) {
            ManifestFieldType::Enum(_) => ManifestFieldType::Enum(declared.enum_values.clone()),
            other => other,
        };
        let proof_role = declared.proof_role.as_deref().and_then(ProofRole::parse);
        Ok(Self {
            kind_name: kind_name.to_string(),
            source_extension: source_extension.to_string(),
            field_type,
            proof_role,
            declared,
        })
    }

    /// The kind the field is registered on.
    pub fn kind_name(&self) -> &str {
        &self.kind_name
    }

    /// The field's name, as declared.
    pub fn name(&self) -> &str {
        &self.declared.name
    }

    /// The extension the field is the kind's through.
    pub fn source_extension(&self) -> &str {
        &self.source_extension
    }

    /// How the host reads the field's value.
    pub fn field_type(&self) -> &ManifestFieldType {
        &self.field_type
    }

    /// What the prove pass reads the field as, when it declares a known role.
    pub fn proof_role(&self) -> Option<ProofRole> {
        self.proof_role
    }

    /// The field as its extension declared it (its type named canonically).
    pub fn declared(&self) -> &FieldDescriptor {
        &self.declared
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
        let kind_map = self
            .entries
            .entry(entry.kind_name().to_string())
            .or_default();
        if !kind_map.contains_key(entry.name()) {
            self.count += 1;
        }
        kind_map.insert(entry.name().to_string(), entry);
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

    /// Reference fields whose target kind `kinds` doesn't declare, as
    /// (kind, field) -> target kind: the kind's extension isn't enabled
    /// (e.g. software's `behavior.features` without @specforge/product).
    pub fn absent_reference_targets(
        &self,
        kinds: &crate::KindRegistry,
    ) -> HashMap<(String, String), String> {
        self.iter()
            .filter_map(|(kind, field, entry)| {
                let target = entry.declared().target_kind.as_ref()?;
                (!kinds.contains(target))
                    .then(|| ((kind.to_string(), field.to_string()), target.clone()))
            })
            .collect()
    }

    pub fn bidirectional_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (_kind, field_name, entry) in self.iter() {
            if let Some(ref inverse) = entry.declared().inverse_of {
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

    /// `kind.name` of the declared type `field_type`, registered through
    /// `@specforge/software`.
    fn entry(kind: &str, name: &str, field_type: FieldType) -> FieldRegistryEntry {
        FieldRegistryEntry::new(
            kind,
            "@specforge/software",
            FieldDescriptor {
                name: name.to_string(),
                field_type: field_type.as_str().to_string(),
                ..Default::default()
            },
        )
        .unwrap()
    }

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
        registry.register(entry("behavior", "contract", FieldType::Block));
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
        registry.register(entry("behavior", "contract", FieldType::String));

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
        registry.register(entry("behavior", "contract", FieldType::String));
        assert_eq!(registry.len(), 1);

        // Re-registering the same (kind, field) should not increase count
        let mut updated = entry("behavior", "contract", FieldType::Block)
            .declared()
            .clone();
        updated.description = Some("updated".to_string());
        registry
            .register(FieldRegistryEntry::new("behavior", "@specforge/software", updated).unwrap());
        assert_eq!(registry.len(), 1);

        // Different field, same kind
        registry.register(entry("behavior", "status", FieldType::String));
        assert_eq!(registry.len(), 2);
    }

    // Verify iter yields all registered entries
    #[test]
    fn test_iter_yields_all_entries() {
        let mut registry = FieldRegistry::new();
        registry.register(entry("behavior", "contract", FieldType::String));
        registry.register(entry("event", "payload", FieldType::Block));

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

    #[test]
    fn an_entry_reads_its_type_and_role_from_its_declaration() {
        let declared = FieldDescriptor {
            name: "level".to_string(),
            field_type: "enum_type".to_string(),
            enum_values: vec!["low".to_string(), "high".to_string()],
            proof_role: Some("assumed".to_string()),
            ..Default::default()
        };
        let entry = FieldRegistryEntry::new("ticket", "@t/x", declared).unwrap();
        assert_eq!(
            entry.field_type(),
            &ManifestFieldType::Enum(vec!["low".to_string(), "high".to_string()])
        );
        assert_eq!(entry.declared().field_type, "enum");
        assert_eq!(entry.proof_role(), None);
        assert_eq!(entry.kind_name(), "ticket");
        assert_eq!(entry.name(), "level");
        assert_eq!(entry.source_extension(), "@t/x");
    }
}
