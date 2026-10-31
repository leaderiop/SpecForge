//! The header of a commit subject, as GSD's `/gsd-undo` reads it: `{type}({phase}-{plan}): {task}`.
//!
//! A port of `src/undo-commit-selection.cts:95-111` (one `fixup! `, `squash! ` or `Revert "…"` wrapper is
//! unwrapped, never two) and of `scripts/release-notes/conventional-title.cjs:28`, `HEADER_RE =
//! /^([a-z]+)(\([^)]*\))?(!)?:/i` (gsd-core v1.19.0, MIT, `LICENSE-GSD`), hand-written: the crate has no regex
//! dependency.

/// A parsed header.
#[derive(Debug, PartialEq, Eq)]
pub struct Header<'a> {
    /// The type, lower-cased: `feat`, `fix`, `test`, `docs` ...
    pub kind: String,
    /// The text inside the parentheses: the plan's GSD id, when the subject names one.
    pub scope: Option<&'a str>,
    /// What follows the colon, trimmed: GSD's "task description".
    pub description: &'a str,
}

/// The subject with one `fixup! `, `squash! ` or `Revert "…"` layer removed.
fn unwrap_one(subject: &str) -> &str {
    for prefix in ["fixup! ", "squash! "] {
        if let Some(rest) = subject.strip_prefix(prefix) {
            return rest;
        }
    }
    subject
        .strip_prefix("Revert \"")
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(subject)
}

/// The header of `subject`, or `None` when it has no `type[(scope)][!]:` at its start.
pub fn parse(subject: &str) -> Option<Header<'_>> {
    let subject = unwrap_one(subject);
    let letters = subject
        .bytes()
        .take_while(|b| b.is_ascii_alphabetic())
        .count();
    if letters == 0 {
        return None;
    }
    let (kind, mut rest) = subject.split_at(letters);
    let mut scope = None;
    if let Some(inner) = rest.strip_prefix('(') {
        let close = inner.find(')')?;
        scope = Some(&inner[..close]);
        rest = &inner[close + 1..];
    }
    rest = rest.strip_prefix('!').unwrap_or(rest);
    let description = rest.strip_prefix(':')?;
    Some(Header {
        kind: kind.to_ascii_lowercase(),
        scope,
        description: description.trim(),
    })
}
