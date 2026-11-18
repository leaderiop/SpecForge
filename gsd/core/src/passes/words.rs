//! The word matcher and the fold: whether a text holds a word or a phrase, and the one reading of a word or phrase.
//!
//! - [`word_starts`] is JavaScript's ASCII `\b` with the case as written, the one boundary rule. Its readers are
//!   decision citation (`passes::citations`, an id such as `D-01` in a plan's text) and [`has_word`].
//! - [`has_word`] is the same boundary with ASCII case folded, as upstream's planner rule reads its phrases:
//!   `gsd/E312` (`passes::scope`) reads a task's action and a decision's text so.
//! - [`folded`] is the reading the git collector compares a task's title with a commit's task description through.
//!   No stemming, no accent stripping, no normalisation.

/// A word or phrase as compared: each run of whitespace one space, then Unicode lowercase (`str::to_lowercase`, not
/// full case folding: "STRASSE" is not "straße").
pub fn folded(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn is_js_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The byte offset of every occurrence of `needle` in `haystack` as a whole word or phrase: `\b<needle>\b`,
/// JavaScript's `\b` on bytes, where a word character is an ASCII letter, digit or `_`, and the case as written. The
/// needle starts and ends with a word character (an id, a phrase). An empty needle is in nothing. Occurrences may
/// overlap, as a regex tries each start: `a-a` is a word of `xa-a-a` at 3. No regex: a substring search.
pub fn word_starts(haystack: &str, needle: &str) -> Vec<usize> {
    let Some(first) = needle.chars().next() else {
        return Vec::new();
    };
    let bytes = haystack.as_bytes();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let at = from + offset;
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + needle.len()).copied();
        if !before.is_some_and(is_js_word) && !after.is_some_and(is_js_word) {
            found.push(at);
        }
        from = at + first.len_utf8();
    }
    found
}

/// Whether `haystack` holds `needle` as a whole word or phrase: [`word_starts`] over both with ASCII letters folded.
pub fn has_word(haystack: &str, needle: &str) -> bool {
    !word_starts(&haystack.to_ascii_lowercase(), &needle.to_ascii_lowercase()).is_empty()
}

#[cfg(test)]
mod tests;
