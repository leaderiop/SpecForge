//! The one word matcher (ADR 0357 D15): whether a text holds a word or a phrase, read at one of two boundaries.
//!
//! - [`Boundary::Js`] is JavaScript's ASCII `\b`, as upstream's planner rule reads its phrases: `gsd/E312`
//!   (`passes::scope`) reads a task's action and a decision's text so.
//! - [`Boundary::Unicode`] is the vocabulary's reading of the author's prose, in any language: `term.used_in`
//!   (`passes::vocabulary`) reads it so.
//!
//! [`folded`] is the one reading of a word the glossary compares: `gsd/W538` compares the terms' words with it, and
//! the Unicode boundary folds both sides with it. No stemming, no accent stripping, no normalisation.

/// Where a word starts and ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// JavaScript's `\b`, on bytes: a word character is an ASCII letter, digit or `_`, and ASCII letters fold.
    Js,
    /// A word character is a letter, a number, `_` or a combining mark, and both sides are [`folded`].
    Unicode,
}

/// A word or phrase as the glossary compares it: each run of whitespace one space, then Unicode lowercase
/// (`str::to_lowercase`, not full case folding: "STRASSE" is not "straße").
pub fn folded(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Whether `haystack` holds `needle` as a whole word or phrase, read at `boundary`. An empty needle is in nothing.
pub fn has_word(haystack: &str, needle: &str, boundary: Boundary) -> bool {
    match boundary {
        Boundary::Js => js(haystack, needle),
        Boundary::Unicode => unicode(haystack, needle),
    }
}

fn is_js_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `\b<needle>\b`, case-insensitively for ASCII letters. The needle starts and ends with a word character (an id, a
/// phrase).
fn js(haystack: &str, needle: &str) -> bool {
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

/// A letter or a number, `_`, or a combining mark, which stays in the word of the letter it marks.
fn is_word(c: char) -> bool {
    c.is_alphanumeric()
        || c == '_'
        || matches!(
            c,
            '\u{0300}'..='\u{036F}'
                | '\u{1AB0}'..='\u{1AFF}'
                | '\u{1DC0}'..='\u{1DFF}'
                | '\u{20D0}'..='\u{20FF}'
                | '\u{FE20}'..='\u{FE2F}'
        )
}

/// Every occurrence of the folded needle, overlapping ones too. A boundary is needed only on an edge where the
/// needle's own character is a word character, so "C++" is found before a full stop.
fn unicode(haystack: &str, needle: &str) -> bool {
    let needle: Vec<char> = folded(needle).chars().collect();
    let (Some(&first), Some(&last)) = (needle.first(), needle.last()) else {
        return false;
    };
    let haystack: Vec<char> = folded(haystack).chars().collect();
    let (opens, closes) = (is_word(first), is_word(last));
    haystack
        .windows(needle.len())
        .enumerate()
        .any(|(at, window)| {
            window == needle.as_slice()
                && !(opens && at.checked_sub(1).is_some_and(|i| is_word(haystack[i])))
                && !(closes && haystack.get(at + needle.len()).is_some_and(|&c| is_word(c)))
        })
}

#[cfg(test)]
mod tests;
