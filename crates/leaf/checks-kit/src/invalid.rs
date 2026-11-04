use std::ops::Range;

/// Why a text is not well formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub reason: String,
    /// The bytes of the text it is about, when it is about part of it.
    pub at: Option<Range<usize>>,
}

impl Invalid {
    /// About the whole text.
    pub fn new(reason: impl Into<String>) -> Self {
        Invalid {
            reason: reason.into(),
            at: None,
        }
    }

    /// About the bytes `at` of the text.
    pub fn at(reason: impl Into<String>, at: Range<usize>) -> Self {
        Invalid {
            reason: reason.into(),
            at: Some(at),
        }
    }

    /// About the one character of `text` that starts at byte `at`.
    pub fn char_at(reason: impl Into<String>, text: &str, at: usize) -> Self {
        let length = text[at..].chars().next().map_or(1, char::len_utf8);
        Invalid::at(reason, at..at + length)
    }
}
