//! The vocabulary of the author's prose (ADR 0357 D16-D18): the one table of prose fields the vocabulary reads, where
//! an entity's prose holds a word, and the `vocabulary` pass, gsd/W534. `term.used_in` (`passes::derive::product`)
//! reads the same table; the terms' own words are gsd/W538's (`passes::glossary`).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, rows, strings, text};
use super::words::{Boundary, Segments, folded, has_word};

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

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("vocabulary", |p| {
        p.phase("check").run(check);
    });
}

/// A word a term avoids, as its first term wrote it, and each term that avoids it with the word that term asks for.
struct Avoided<'a> {
    word: String,
    terms: Vec<(&'a str, &'a str)>,
}

/// The glossary as W534 reads it: every name and alias the terms accept, and the words they avoid.
struct Glossary<'a> {
    accepted: Vec<Segments>,
    /// By folded spelling, each with every term that avoids it; a word the glossary also accepts (a term's `name` or
    /// alias) is gsd/W538's, never W534's, and is left out.
    avoided: BTreeMap<String, Avoided<'a>>,
}

fn glossary(input: &PassInput) -> Glossary<'_> {
    let mut terms: Vec<&PassEntity> = of_kind(input, "term").collect();
    terms.sort_by(|a, b| a.id.cmp(&b.id));
    let accepted: BTreeSet<String> = terms
        .iter()
        .flat_map(|t| {
            text(t, "name")
                .map(str::to_string)
                .into_iter()
                .chain(strings(t, "aliases"))
        })
        .map(|word| folded(&word))
        .filter(|word| !word.is_empty())
        .collect();
    let mut avoided: BTreeMap<String, Avoided> = BTreeMap::new();
    for term in terms {
        let asks = text(term, "name").unwrap_or(term.id.as_str());
        for word in strings(term, "avoid") {
            let key = folded(&word);
            if key.is_empty() || accepted.contains(&key) {
                continue;
            }
            let entry = avoided.entry(key).or_insert_with(|| Avoided {
                word,
                terms: Vec::new(),
            });
            if !entry.terms.iter().any(|(id, _)| *id == term.id) {
                entry.terms.push((term.id.as_str(), asks));
            }
        }
    }
    Glossary {
        accepted: accepted.iter().map(|word| Segments::of(word)).collect(),
        avoided,
    }
}

/// A place of an entity's prose, read for W534: the place, its text's segments, and where in them it writes an
/// accepted name or alias.
struct Place<'p> {
    place: &'p str,
    text: Segments,
    names: Vec<Range<usize>>,
}

impl<'p> Place<'p> {
    fn read(place: &'p str, said: &str, accepted: &[Segments]) -> Self {
        let text = Segments::of(said);
        let names = accepted.iter().flat_map(|name| text.find(name)).collect();
        Place { place, text, names }
    }

    /// Whether the place writes `word` outside every accepted name it writes: a "ticket" inside "the ticket system",
    /// a term's name, is that name.
    fn writes(&self, word: &Segments) -> bool {
        self.text.find(word).any(|at| {
            !self
                .names
                .iter()
                .any(|name| name.start <= at.start && at.end <= name.end)
        })
    }
}

/// W534: prose (`PROSE`) that holds a word or phrase a term avoids, read at the Unicode boundary, outside every
/// accepted name or alias it writes. One warning per entity and avoided word, on the entity, naming the first place
/// that holds it, every term that avoids it and the word each asks for.
fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let glossary = glossary(input);
    if glossary.avoided.is_empty() {
        return Vec::new();
    }
    let needles: Vec<(&Avoided, Segments)> = glossary
        .avoided
        .values()
        .map(|avoided| (avoided, Segments::of(&avoided.word)))
        .collect();
    let mut entities: Vec<&PassEntity> = writers(input).collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    let mut found = Vec::new();
    for entity in entities {
        let prose = prose(entity);
        let places: Vec<Place> = prose
            .iter()
            .map(|(place, said)| Place::read(place, said, &glossary.accepted))
            .collect();
        for (avoided, needle) in &needles {
            let Some(place) = places.iter().find(|p| p.writes(needle)).map(|p| p.place) else {
                continue;
            };
            let terms: Vec<String> = avoided
                .terms
                .iter()
                .map(|(id, _)| format!("term '{id}'"))
                .collect();
            let asks: Vec<String> = avoided
                .terms
                .iter()
                .map(|(id, asks)| format!("\"{asks}\" (term '{id}')"))
                .collect();
            found.push(
                PassDiagnostic::new(
                    "W534",
                    PassSeverity::Warning,
                    format!(
                        "{} '{}' writes \"{}\" in {place}, a word avoided by {}",
                        entity.kind,
                        entity.id,
                        avoided.word,
                        terms.join(" and ")
                    ),
                )
                .with_suggestion(format!("write {} instead", asks.join(" or ")))
                .with_entity(entity.id.as_str()),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
