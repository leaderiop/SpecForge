//! The word matcher and the fold: whether a text holds a word or a phrase, and the one reading of a word or phrase.
//!
//! - [`has_word`] is JavaScript's ASCII `\b`, as upstream's planner rule reads its phrases: `gsd/E312`
//!   (`passes::scope`) reads a task's action and a decision's text so.
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

/// Whether `haystack` holds `needle` as a whole word or phrase: `\b<needle>\b`, JavaScript's `\b` on bytes, where a
/// word character is an ASCII letter, digit or `_`, case-insensitively for ASCII letters. The needle starts and ends
/// with a word character (an id, a phrase). An empty needle is in nothing.
pub fn has_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let (haystack, needle) = (haystack.to_ascii_lowercase(), needle.to_ascii_lowercase());
    let bytes = haystack.as_bytes();
    haystack.match_indices(&needle).any(|(at, found)| {
        let before = at.checked_sub(1).map(|i| bytes[i]);
        let after = bytes.get(at + found.len()).copied();
        !before.is_some_and(is_js_word) && !after.is_some_and(is_js_word)
    })
}

#[cfg(test)]
mod tests;
