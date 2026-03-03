use serde::{Deserialize, Serialize};
use std::fmt;

/// A source location span within a `.spec` file.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl SourceSpan {
    pub fn new(file: impl Into<String>, start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            file: file.into(),
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// A zero-span at the beginning of a file (used for file-level diagnostics).
    pub fn file_start(file: impl Into<String>) -> Self {
        Self::new(file, 1, 1, 1, 1)
    }
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.start_line, self.start_col)
    }
}

impl Ord for SourceSpan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.file
            .cmp(&other.file)
            .then(self.start_line.cmp(&other.start_line))
            .then(self.start_col.cmp(&other.start_col))
            .then(self.end_line.cmp(&other.end_line))
            .then(self.end_col.cmp(&other.end_col))
    }
}

impl PartialOrd for SourceSpan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format() {
        let span = SourceSpan::new("foo.spec", 10, 5, 10, 20);
        assert_eq!(span.to_string(), "foo.spec:10:5");
    }

    #[test]
    fn ordering() {
        let a = SourceSpan::new("a.spec", 1, 1, 1, 10);
        let b = SourceSpan::new("a.spec", 2, 1, 2, 10);
        let c = SourceSpan::new("b.spec", 1, 1, 1, 10);
        assert!(a < b);
        assert!(b < c);
    }
}
