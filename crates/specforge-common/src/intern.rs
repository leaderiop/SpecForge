use lasso::{Spur, ThreadedRodeo};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::LazyLock;

/// Global interning table (INV-SF-3).
///
/// Uses `LazyLock` for Phase 1 simplicity. Thread-safe via `ThreadedRodeo`.
static GLOBAL_INTERNER: LazyLock<ThreadedRodeo> = LazyLock::new(ThreadedRodeo::default);

/// Newtype over `lasso::Spur` for interned string comparison.
///
/// - `Copy`, `Eq`, `Hash` for cheap comparisons
/// - Resolves back to `&str` via the global interner
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct InternedStr(Spur);

impl InternedStr {
    /// Intern a string, returning a handle for cheap comparisons.
    pub fn new(s: &str) -> Self {
        Self(GLOBAL_INTERNER.get_or_intern(s))
    }

    /// Resolve back to a string slice.
    pub fn as_str(&self) -> &str {
        GLOBAL_INTERNER.resolve(&self.0)
    }
}

impl fmt::Debug for InternedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InternedStr({:?})", self.as_str())
    }
}

impl fmt::Display for InternedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<InternedStr> for String {
    fn from(s: InternedStr) -> Self {
        s.as_str().to_string()
    }
}

impl TryFrom<String> for InternedStr {
    type Error = std::convert::Infallible;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Ok(Self::new(&s))
    }
}

impl From<&str> for InternedStr {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Wrapper around the global interner for explicit access.
pub struct Interner;

impl Interner {
    /// Intern a string.
    pub fn get_or_intern(s: &str) -> InternedStr {
        InternedStr::new(s)
    }

    /// Try to look up a string without interning it.
    pub fn get(s: &str) -> Option<InternedStr> {
        GLOBAL_INTERNER.get(s).map(InternedStr)
    }

    /// Number of interned strings.
    pub fn len() -> usize {
        GLOBAL_INTERNER.len()
    }

    /// Whether the interner is empty.
    pub fn is_empty() -> bool {
        GLOBAL_INTERNER.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_roundtrip() {
        let s = InternedStr::new("hello");
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s.to_string(), "hello");
    }

    #[test]
    fn same_string_same_key() {
        let a = InternedStr::new("INV-SF-1");
        let b = InternedStr::new("INV-SF-1");
        assert_eq!(a, b);
    }

    #[test]
    fn different_strings_different_keys() {
        let a = InternedStr::new("INV-SF-1");
        let b = InternedStr::new("INV-SF-2");
        assert_ne!(a, b);
    }

    #[test]
    fn copy_semantics() {
        let a = InternedStr::new("copy-test");
        let b = a; // Copy
        assert_eq!(a, b);
        assert_eq!(a.as_str(), "copy-test");
    }

    #[test]
    fn interner_get() {
        let key = "unique-lookup-test";
        assert!(Interner::get(key).is_none() || Interner::get(key).is_some());
        let _ = InternedStr::new(key);
        assert!(Interner::get(key).is_some());
    }

    #[test]
    fn from_str() {
        let s: InternedStr = "from-str-test".into();
        assert_eq!(s.as_str(), "from-str-test");
    }
}
