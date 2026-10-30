//! The lexical part of the verifier's file checks (`agents/gsd-verifier.md:404-422`): debt markers, cleanup comments,
//! placeholder phrases and a key link's `pattern`. Hand-written: the matching is a deterministic subset of
//! upstream's `grep`, and says where it differs.
//!
//! gsd-core modules this file ports: pattern.cjs.

/// How serious an anti-pattern is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    Info,
    Warning,
    Blocker,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Blocker => "blocker",
        }
    }
}

/// One line that matches.
#[derive(Debug, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub text: String,
    pub severity: Severity,
    /// The marker word, for a blocker.
    pub marker: &'static str,
}

const BLOCKERS: &[&str] = &["TBD", "FIXME", "XXX"];
const WARNINGS: &[&str] = &["TODO", "HACK", "PLACEHOLDER"];
/// Matched case-insensitively.
const PHRASES: &[&str] = &[
    "placeholder",
    "coming soon",
    "will be here",
    "not yet implemented",
    "not available",
];

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn words(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !is_word(c)).filter(|w| !w.is_empty())
}

/// `#N`, `issue #N`, `PR #N` (all contain `#` and a number) or `DEF-N`: the line names formal follow-up work. The
/// number ends at a non-letter: a colour like `#1a2b` names nothing.
fn names_follow_up(line: &str) -> bool {
    let number = |rest: &str| {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        digits > 0 && !rest[digits..].starts_with(|c: char| c.is_alphanumeric())
    };
    let after = |needle: &str| {
        line.match_indices(needle)
            .any(|(at, _)| number(&line[at + needle.len()..]))
    };
    after("#") || after("DEF-")
}

/// The lines that carry an anti-pattern, the most serious kind per line. A blocker is a whole word (`XXXL` is not
/// one) on a line that names no follow-up.
pub fn anti_patterns(text: &str) -> Vec<Hit> {
    text.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let marker = |list: &[&'static str]| {
                words(line).find_map(|w| list.iter().find(|m| **m == w).copied())
            };
            let (severity, marker) =
                if let Some(m) = marker(BLOCKERS).filter(|_| !names_follow_up(line)) {
                    (Severity::Blocker, m)
                } else if let Some(m) = marker(WARNINGS) {
                    (Severity::Warning, m)
                } else if PHRASES.iter().any(|p| line.to_lowercase().contains(p)) {
                    (Severity::Info, "")
                } else {
                    return None;
                };
            let shown: String = line.trim().chars().take(160).collect();
            Some(Hit {
                line: i + 1,
                text: shown,
                severity,
                marker,
            })
        })
        .collect()
}

/// Whether a key link's `pattern` is plain text. Upstream compiles it as a regular expression, which this scan does
/// not: a pattern with a metacharacter is not checked.
pub fn is_literal(pattern: &str) -> bool {
    !pattern.contains([
        '\\', '.', '*', '+', '?', '(', ')', '[', ']', '{', '}', '|', '^', '$',
    ])
}

#[cfg(test)]
mod tests;
