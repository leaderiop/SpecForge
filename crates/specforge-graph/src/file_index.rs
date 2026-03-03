use std::collections::HashMap;

/// Tracks which entities belong to which files.
///
/// Used for:
/// - Mapping entity IDs back to their source files
/// - Finding all entities in a specific file (for watch mode invalidation)
#[derive(Debug, Default)]
pub struct FileIndex {
    /// entity raw ID -> file path
    entity_to_file: HashMap<String, String>,
    /// file path -> list of entity raw IDs
    file_to_entities: HashMap<String, Vec<String>>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an entity as belonging to a file.
    pub fn register(&mut self, entity_id: &str, file: &str) {
        self.entity_to_file
            .insert(entity_id.to_string(), file.to_string());
        self.file_to_entities
            .entry(file.to_string())
            .or_default()
            .push(entity_id.to_string());
    }

    /// Get the file that owns a given entity.
    pub fn file_of(&self, entity_id: &str) -> Option<&str> {
        self.entity_to_file.get(entity_id).map(|s| s.as_str())
    }

    /// Get all entity IDs declared in a file.
    pub fn entities_in(&self, file: &str) -> &[String] {
        self.file_to_entities
            .get(file)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Remove all entities associated with a file (for watch mode re-parse).
    pub fn remove_file(&mut self, file: &str) {
        if let Some(ids) = self.file_to_entities.remove(file) {
            for id in ids {
                self.entity_to_file.remove(&id);
            }
        }
    }

    /// Number of tracked entities.
    pub fn entity_count(&self) -> usize {
        self.entity_to_file.len()
    }

    /// Number of tracked files.
    pub fn file_count(&self) -> usize {
        self.file_to_entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup() {
        let mut idx = FileIndex::new();
        idx.register("INV-SF-1", "invariants.spec");
        idx.register("INV-SF-2", "invariants.spec");
        idx.register("BEH-SF-1", "behaviors.spec");

        assert_eq!(idx.file_of("INV-SF-1"), Some("invariants.spec"));
        assert_eq!(idx.file_of("BEH-SF-1"), Some("behaviors.spec"));
        assert_eq!(idx.file_of("UNKNOWN"), None);

        assert_eq!(idx.entities_in("invariants.spec").len(), 2);
        assert_eq!(idx.entities_in("behaviors.spec").len(), 1);
        assert_eq!(idx.entities_in("unknown.spec").len(), 0);
    }

    #[test]
    fn remove_file() {
        let mut idx = FileIndex::new();
        idx.register("INV-SF-1", "invariants.spec");
        idx.register("INV-SF-2", "invariants.spec");
        assert_eq!(idx.entity_count(), 2);

        idx.remove_file("invariants.spec");
        assert_eq!(idx.entity_count(), 0);
        assert_eq!(idx.file_count(), 0);
        assert_eq!(idx.file_of("INV-SF-1"), None);
    }

    #[test]
    fn counts() {
        let mut idx = FileIndex::new();
        idx.register("INV-SF-1", "invariants.spec");
        idx.register("BEH-SF-1", "behaviors.spec");
        assert_eq!(idx.entity_count(), 2);
        assert_eq!(idx.file_count(), 2);
    }
}
