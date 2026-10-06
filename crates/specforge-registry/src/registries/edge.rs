use std::collections::HashMap;

use specforge_protocol_types::EdgeTypeDescriptor;

/// One registered edge type, as its extension declared it (an edge a
/// field maps to without declaring it is registered with the field's kind
/// as its source and its target kind, nothing else).
#[derive(Debug, Clone, Default)]
pub struct EdgeRegistryEntry {
    pub source_extension: String,
    pub declared: EdgeTypeDescriptor,
}

#[derive(Debug, Default)]
pub struct EdgeRegistry {
    entries: HashMap<String, EdgeRegistryEntry>,
}

impl EdgeRegistry {
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

    pub fn get(&self, label: &str) -> Option<&EdgeRegistryEntry> {
        self.entries.get(label)
    }

    pub fn contains(&self, label: &str) -> bool {
        self.entries.contains_key(label)
    }

    pub fn register(&mut self, entry: EdgeRegistryEntry) -> Option<EdgeRegistryEntry> {
        self.entries.insert(entry.declared.label.clone(), entry)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &EdgeRegistryEntry)> {
        self.entries.iter()
    }

    pub fn labels(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // B:boot_empty_edge_registry — verify unit "edge type set starts with zero entries"
    #[test]
    fn test_edge_registry_new_has_zero_entries() {
        let registry = EdgeRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    // B:boot_empty_edge_registry — verify unit "no edge labels recognized before extension loading"
    #[test]
    fn test_no_edge_labels_recognized_before_extension_loading() {
        let registry = EdgeRegistry::new();
        assert!(registry.get("invariants").is_none());
        assert!(!registry.contains("invariants"));
    }

    // B:boot_empty_edge_registry — verify contract "requires/ensures consistency for empty edge registry boot"
    #[test]
    fn test_boot_empty_edge_registry_contract() {
        // requires: no extensions loaded yet
        let registry = EdgeRegistry::new();
        // ensures: empty
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        // ensures: get returns None
        assert!(registry.get("enforces").is_none());
        // ensures: labels yields nothing
        assert_eq!(registry.labels().count(), 0);
        // ensures: iter yields nothing
        assert_eq!(registry.iter().count(), 0);
    }
}
