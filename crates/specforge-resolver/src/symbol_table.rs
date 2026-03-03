use specforge_common::{EntityId, EntityKind, SourceSpan};
use std::collections::HashMap;

/// A declaration entry in the symbol table.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub id: EntityId,
    pub kind: EntityKind,
    pub file: String,
    pub span: SourceSpan,
}

/// Global symbol table mapping entity ID strings to their declarations.
///
/// Used for:
/// - E002: duplicate ID detection
/// - E001: dangling reference detection
/// - Reference resolution
#[derive(Debug, Default)]
pub struct SymbolTable {
    entries: HashMap<String, Declaration>,
    /// Track duplicates: id -> list of all declaration sites
    all_sites: HashMap<String, Vec<Declaration>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a declaration. Returns `Some(existing)` if duplicate (E002).
    pub fn insert(&mut self, decl: Declaration) -> Option<&Declaration> {
        let raw = decl.id.raw().to_string();

        self.all_sites
            .entry(raw.clone())
            .or_default()
            .push(decl.clone());

        use std::collections::hash_map::Entry;
        match self.entries.entry(raw) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(e) => {
                e.insert(decl);
                None
            }
        }
    }

    /// Look up a declaration by raw ID string.
    pub fn get(&self, raw: &str) -> Option<&Declaration> {
        self.entries.get(raw)
    }

    /// Check if an ID is declared.
    pub fn contains(&self, raw: &str) -> bool {
        self.entries.contains_key(raw)
    }

    /// All declarations.
    pub fn declarations(&self) -> impl Iterator<Item = &Declaration> {
        self.entries.values()
    }

    /// Number of declared entities.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all raw ID strings (for did-you-mean suggestions).
    pub fn all_ids(&self) -> Vec<&str> {
        self.entries.keys().map(|k| k.as_str()).collect()
    }

    /// Remove all declarations that belong to a file (for watch mode re-parse).
    pub fn remove_file(&mut self, file: &str) {
        let ids_to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, decl)| decl.file == file)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids_to_remove {
            self.entries.remove(id);
            self.all_sites.remove(id);
        }
    }

    /// Get declarations of a specific kind.
    pub fn declarations_of_kind(&self, kind: EntityKind) -> Vec<&Declaration> {
        self.entries
            .values()
            .filter(|d| d.kind == kind)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_decl(id: &str, kind: EntityKind, file: &str) -> Declaration {
        Declaration {
            id: EntityId::parse(id),
            kind,
            file: file.to_string(),
            span: SourceSpan::new(file, 1, 1, 1, 1),
        }
    }

    #[test]
    fn insert_unique() {
        let mut table = SymbolTable::new();
        let result = table.insert(make_decl("data_integrity", EntityKind::Invariant, "a.spec"));
        assert!(result.is_none());
        assert!(table.contains("data_integrity"));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn insert_duplicate() {
        let mut table = SymbolTable::new();
        table.insert(make_decl("data_integrity", EntityKind::Invariant, "a.spec"));
        let dup = table.insert(make_decl("data_integrity", EntityKind::Invariant, "b.spec"));
        assert!(dup.is_some());
        assert_eq!(dup.unwrap().file, "a.spec");
    }

    #[test]
    fn lookup() {
        let mut table = SymbolTable::new();
        table.insert(make_decl("create_user", EntityKind::Behavior, "beh.spec"));
        let found = table.get("create_user");
        assert!(found.is_some());
        assert_eq!(found.unwrap().kind, EntityKind::Behavior);
        assert!(table.get("nonexistent_beh").is_none());
    }

    #[test]
    fn all_ids() {
        let mut table = SymbolTable::new();
        table.insert(make_decl("data_integrity", EntityKind::Invariant, "a.spec"));
        table.insert(make_decl("validate_input", EntityKind::Behavior, "b.spec"));
        let ids = table.all_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"data_integrity"));
        assert!(ids.contains(&"validate_input"));
    }

    #[test]
    fn remove_file_clears_declarations() {
        let mut table = SymbolTable::new();
        table.insert(make_decl("data_integrity", EntityKind::Invariant, "a.spec"));
        table.insert(make_decl("email_uniqueness", EntityKind::Invariant, "a.spec"));
        table.insert(make_decl("validate_input", EntityKind::Behavior, "b.spec"));
        assert_eq!(table.len(), 3);

        table.remove_file("a.spec");
        assert_eq!(table.len(), 1);
        assert!(!table.contains("data_integrity"));
        assert!(!table.contains("email_uniqueness"));
        assert!(table.contains("validate_input"));
    }

    #[test]
    fn remove_file_noop_for_unknown() {
        let mut table = SymbolTable::new();
        table.insert(make_decl("data_integrity", EntityKind::Invariant, "a.spec"));
        table.remove_file("nonexistent.spec");
        assert_eq!(table.len(), 1);
    }
}
