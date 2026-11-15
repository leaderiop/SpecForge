//! The vocabulary of the author's prose (ADR 0357 D16-D18): the one table of prose fields the vocabulary reads, and
//! where an entity's prose holds a word. `term.used_in` (`passes::derive::product`) reads it.

use specforge_extension_sdk::prelude::*;

use super::plans::{rows, text};
use super::words::{Boundary, has_word};

/// The authored prose fields, by kind, read in the author's language (`text_en` is not): a field's name, or
/// `<list>.<member>` for a member of each row of a block list.
pub(crate) const PROSE: &[(&str, &[&str])] = &[
    ("requirement", &["text"]),
    ("decision", &["text", "rationale", "context"]),
    ("truth", &["text"]),
    ("journey", &["steps.action"]),
    ("risk", &["cause", "effect"]),
    ("invariant", &["statement"]),
];

/// The prose fields of `kind`; none for a kind that writes no prose.
fn fields(kind: &str) -> &'static [&'static str] {
    PROSE
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(&[], |(_, fields)| fields)
}

/// The entities of the input that write prose, in input order.
pub(crate) fn writers(input: &PassInput) -> impl Iterator<Item = &PassEntity> {
    input
        .entities
        .iter()
        .filter(|e| !fields(&e.kind).is_empty())
}

/// `(where, text)` of each written, non-blank prose value of `entity`, in the table's order: `where` is the field, or
/// a row's place for a member of a block list (`step 2`, the list's name made singular and the row's 1-based number).
pub(crate) fn prose(entity: &PassEntity) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in fields(&entity.kind) {
        match path.split_once('.') {
            Some((list, member)) => {
                let row = list.strip_suffix('s').unwrap_or(list);
                for (at, values) in rows(entity, list).iter().enumerate() {
                    if let Some(said) = values
                        .get(member)
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                    {
                        out.push((format!("{row} {}", at + 1), said.to_string()));
                    }
                }
            }
            None => out.extend(text(entity, path).map(|said| (path.to_string(), said.to_string()))),
        }
    }
    out
}

/// The first place of `prose` (as [`prose`] reads it) that holds `phrase` as a whole word or phrase, read at the
/// Unicode boundary.
pub(crate) fn uses<'p>(prose: &'p [(String, String)], phrase: &str) -> Option<&'p str> {
    prose
        .iter()
        .find(|(_, said)| has_word(said, phrase, Boundary::Unicode))
        .map(|(place, _)| place.as_str())
}

#[cfg(test)]
mod tests;
