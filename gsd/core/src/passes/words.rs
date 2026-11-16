//! The one word matcher (ADR 0357 D15): whether a text holds a word or a phrase, read at one of two boundaries.
//!
//! - [`Boundary::Js`] is JavaScript's ASCII `\b`, as upstream's planner rule reads its phrases: `gsd/E312`
//!   (`passes::scope`) reads a task's action and a decision's text so.
//! - [`Boundary::Unicode`] is the vocabulary's reading of the author's prose: Unicode word segmentation (UAX #29), an
//!   apostrophe splitting a word as French elision does ([`Segments`]), a path or a code span in the prose read as
//!   no word ([`Segments::of_prose`]). `term.used_in` and `gsd/W534` (`passes::vocabulary`) read it so.
//!
//! [`folded`] is the one reading of a word the glossary compares: `gsd/W538` compares the terms' words with it, the
//! Unicode boundary folds each segment the same way, and the git collector compares a task's title with a commit's
//! task description through it. No stemming, no accent stripping, no normalisation.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

/// Where a word starts and ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    /// JavaScript's `\b`, on bytes: a word character is an ASCII letter, digit or `_`, and ASCII letters fold.
    Js,
    /// Unicode word segmentation, an apostrophe splitting a word, each segment [`folded`]: see [`Segments`].
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
        Boundary::Unicode => Segments::of_prose(haystack)
            .find(&Segments::of(needle))
            .next()
            .is_some(),
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

/// The apostrophes that split a word, so an elided article is a word of its own ("l'Élève" reads "l", "'",
/// "Élève"): UAX #29 joins two letters across them, and allows this tailoring for languages that elide.
const APOSTROPHES: [char; 2] = ['\'', '\u{2019}'];

/// A text as [`Boundary::Unicode`] reads it: its word-boundary segments (UAX #29, `split_word_bound_indices`), each
/// split at its apostrophes and [`folded`], a run of whitespace one segment `" "`, the text's edges trimmed; each keeps
/// where it sits in the text, so an occurrence reads back as written ([`Segments::written`]).
///
/// [`Segments::of_prose`] reads what prose holds that is not words as one opaque segment each, so no word is found in
/// it and no phrase across it.
///
/// A word is one segment, so a combining mark, a joiner (U+200C, U+200D), a virama or a vowel sign stays in the word
/// it marks, in every script, and a full stop or a colon between two letters joins them (`e.g.`, `example.com`).
/// Punctuation is a segment of its own. A run of Katakana is one word; each Han ideograph and each Hiragana letter is
/// a segment, so a word of them is found wherever its characters run in the text. Thai, Lao, Khmer and Myanmar, which
/// UAX #29 splits into words only with a dictionary, read as one segment per letter and its marks: a word of them is
/// also found inside a longer word.
pub struct Segments<'t> {
    text: &'t str,
    folded: Vec<String>,
    /// Where each segment sits in `text`: a run of whitespace spans the whole run.
    spans: Vec<Range<usize>>,
}

impl<'t> Segments<'t> {
    /// A word, a phrase or a name, as written.
    pub fn of(text: &'t str) -> Self {
        Self::read(text, &[])
    }

    /// Prose: [`Segments::of`] with each path and code span ([`not_words`]) one opaque segment.
    pub fn of_prose(text: &'t str) -> Self {
        Self::read(text, &not_words(text))
    }

    /// The segments of `text` between `masks` (sorted, disjoint byte ranges), each mask one opaque segment.
    fn read(text: &'t str, masks: &[Range<usize>]) -> Self {
        let mut segments = Segments {
            text,
            folded: Vec::new(),
            spans: Vec::new(),
        };
        let mut from = 0;
        for mask in masks {
            segments.words(from..mask.start);
            segments.push(OPAQUE.to_string(), mask.clone());
            from = mask.end;
        }
        segments.words(from..text.len());
        if segments.folded.last().is_some_and(|last| last == " ") {
            segments.folded.pop();
            segments.spans.pop();
        }
        if segments.folded.first().is_some_and(|first| first == " ") {
            segments.folded.remove(0);
            segments.spans.remove(0);
        }
        segments
    }

    /// Pushes the segments of `text[piece]`.
    fn words(&mut self, piece: Range<usize>) {
        let text = self.text;
        for (at, segment) in text[piece.clone()].split_word_bound_indices() {
            let at = piece.start + at;
            if segment.chars().all(char::is_whitespace) {
                self.push(" ".to_string(), at..at + segment.len());
                continue;
            }
            let mut from = 0;
            for (i, apostrophe) in segment.match_indices(APOSTROPHES) {
                if i > from {
                    self.push(segment[from..i].to_lowercase(), at + from..at + i);
                }
                self.push(apostrophe.to_string(), at + i..at + i + apostrophe.len());
                from = i + apostrophe.len();
            }
            if from < segment.len() {
                self.push(
                    segment[from..].to_lowercase(),
                    at + from..at + segment.len(),
                );
            }
        }
    }

    /// Adds a segment; whitespace beside whitespace (a line break and the indent after it) widens the space before.
    fn push(&mut self, folded: String, span: Range<usize>) {
        if folded == " "
            && self.folded.last().is_some_and(|last| last == " ")
            && let Some(last) = self.spans.last_mut()
        {
            last.end = span.end;
            return;
        }
        self.folded.push(folded);
        self.spans.push(span);
    }

    /// Every occurrence of `needle`'s segments as a run of these, overlapping ones too, as a range of segment indices;
    /// none for an empty needle. Each window is compared once: linear in the text for a given needle, and no pattern
    /// is compiled.
    pub fn find<'s>(&'s self, needle: &'s Segments<'_>) -> impl Iterator<Item = Range<usize>> + 's {
        let width = needle.folded.len();
        self.folded
            .windows(width.max(1))
            .enumerate()
            .filter(move |(_, window)| width > 0 && *window == needle.folded.as_slice())
            .map(move |(at, _)| at..at + width)
    }

    /// The text an occurrence [`Segments::find`] gave covers, as the text writes it.
    pub fn written(&self, found: &Range<usize>) -> &'t str {
        &self.text[self.spans[found.start].start..self.spans[found.end - 1].end]
    }
}

/// The segment that stands for what prose holds that is not words: U+FFFC, which no word is.
const OPAQUE: &str = "\u{FFFC}";

/// What prose holds that is not words (ADR 0357 D16), as sorted, disjoint byte ranges of `text`: a code span (a run
/// of backticks to the next run of as many), and a whitespace-delimited token that holds a `/` or a `\` (a path, a
/// URL). shortcut: a word written with a slash ("CI/CD", "and/or") reads as a path too, which no rule here can tell
/// apart; a dictionary of such words would.
fn not_words(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let run = |at: usize| bytes[at..].iter().take_while(|b| **b == b'`').count();
    let mut masks = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let open = run(at);
        let mut close = at + open;
        let end = loop {
            match bytes[close..].iter().position(|b| *b == b'`') {
                None => break None,
                Some(skip) if run(close + skip) == open => break Some(close + skip + open),
                Some(skip) => close += skip + run(close + skip),
            }
        };
        if let Some(end) = end {
            masks.push(at..end);
        }
        at = end.unwrap_or(at + open);
    }
    let mut start = None;
    for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(from)) => {
                if text[from..i].contains(['/', '\\']) {
                    masks.push(from..i);
                }
                start = None;
            }
            _ => {}
        }
    }
    masks.sort_by_key(|mask| mask.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for mask in masks {
        match merged.last_mut() {
            Some(last) if mask.start <= last.end => last.end = last.end.max(mask.end),
            _ => merged.push(mask),
        }
    }
    merged
}

#[cfg(test)]
mod tests;
