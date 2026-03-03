use crate::intern::InternedStr;
use crate::EntityId;

/// An entity ID with its raw string interned for cheap comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternedEntityId {
    key: InternedStr,
}

impl InternedEntityId {
    /// Intern an entity ID's raw string.
    pub fn new(id: &EntityId) -> Self {
        Self {
            key: InternedStr::new(id.raw()),
        }
    }

    /// Intern from a raw string.
    pub fn from_raw(raw: &str) -> Self {
        Self {
            key: InternedStr::new(raw),
        }
    }

    /// Get the interned key.
    pub fn key(&self) -> InternedStr {
        self.key
    }

    /// Resolve back to the raw string.
    pub fn as_str(&self) -> &str {
        self.key.as_str()
    }
}

impl std::fmt::Display for InternedEntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_named() {
        let id = EntityId::parse("data_persistence");
        let interned = InternedEntityId::new(&id);
        assert_eq!(interned.as_str(), "data_persistence");
    }

    #[test]
    fn roundtrip_identifier() {
        let id = EntityId::parse("FileSystem");
        let interned = InternedEntityId::new(&id);
        assert_eq!(interned.as_str(), "FileSystem");
    }

    #[test]
    fn roundtrip_scheme_ref() {
        let id = EntityId::parse("gh.issue:42");
        let interned = InternedEntityId::new(&id);
        assert_eq!(interned.as_str(), "gh.issue:42");
    }

    #[test]
    fn same_id_same_key() {
        let a = InternedEntityId::from_raw("create_user");
        let b = InternedEntityId::from_raw("create_user");
        assert_eq!(a, b);
        assert_eq!(a.key(), b.key());
    }

    #[test]
    fn different_id_different_key() {
        let a = InternedEntityId::from_raw("create_user");
        let b = InternedEntityId::from_raw("update_user");
        assert_ne!(a, b);
    }

    #[test]
    fn copy_semantics() {
        let a = InternedEntityId::from_raw("copy_test");
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn interned_file_paths() {
        let a = InternedStr::new("invariants/core.spec");
        let b = InternedStr::new("invariants/core.spec");
        let c = InternedStr::new("types/errors.spec");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
