//! The vocabulary of the author's prose (ADR 0357 D16-D18): the one table of prose fields the vocabulary reads,
//! derived from the kinds' tables, where an entity's prose holds a word, and the `vocabulary` pass, gsd/W534.
//! `term.used_in` (`passes::derive::product`) reads the same table; the terms' own words are gsd/W538's
//! (`passes::glossary`).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::LazyLock;

use gsd_kit::dsl::{Field, Kind, Ty, Writer};
use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, strings, text};
use super::words::{Boundary, Segments, folded, has_word};

/// Where a kind writes prose (ADR 0357 D16): an authored field its table marks prose (`Field::prose`), or such a
/// member of an authored block or block list, at any depth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Prose {
    /// The blocks down to the text, outermost first: each block's name, and whether it is a block list (each of its
    /// rows holds the next). Empty for a field of the kind.
    pub blocks: Vec<(&'static str, bool)>,
    /// The text's field or member name.
    pub name: &'static str,
    /// A list of texts (`strs`), not one text.
    pub list: bool,
    /// The one prose member of its block, so a row's place need not name it (`step 2`, not `step 2 action`).
    pub alone: bool,
    /// The author cannot change it once it is committed: the field, a block on its way or its kind is immutable or
    /// append-only (K4). It counts for `term.used_in`, and W534 skips it (ADR 0357 D18).
    pub fixed: bool,
}

/// The prose of each kind of `kinds` that writes some, in their order: a field or member another writer fills
/// (computed, harness, verdict, context), or one inside such a block, is never prose.
pub(crate) fn table(kinds: &[&Kind]) -> Vec<(&'static str, Vec<Prose>)> {
    kinds
        .iter()
        .filter_map(|kind| {
            let mut prose = Vec::new();
            collect(kind.fields, &mut Vec::new(), kind.append_only, &mut prose);
            (!prose.is_empty()).then_some((kind.name, prose))
        })
        .collect()
}

/// Pushes the prose of `fields`, the members of `blocks`' innermost block (or a kind's fields), in their order; `fixed`
/// when the block on their way, or their kind, cannot change once committed.
fn collect(
    fields: &[Field],
    blocks: &mut Vec<(&'static str, bool)>,
    fixed: bool,
    out: &mut Vec<Prose>,
) {
    let authored = || fields.iter().filter(|f| f.writer == Writer::Author);
    let alone = !blocks.is_empty() && authored().filter(|f| f.prose).count() == 1;
    for field in authored() {
        let fixed = fixed || field.immutable || field.append;
        match field.ty {
            Ty::Str | Ty::Strs if field.prose => out.push(Prose {
                blocks: blocks.clone(),
                name: field.name,
                list: matches!(field.ty, Ty::Strs),
                alone,
                fixed,
            }),
            Ty::Block(members) | Ty::Blocks(members) => {
                blocks.push((field.name, matches!(field.ty, Ty::Blocks(_))));
                collect(members, blocks, fixed, out);
                blocks.pop();
            }
            _ => {}
        }
    }
}

/// The vocabulary's table: [`table`] of `@gsd/core`'s kinds, derived once.
static PROSE: LazyLock<Vec<(&'static str, Vec<Prose>)>> =
    LazyLock::new(|| table(crate::kinds::KINDS));

/// The prose of `kind`; none for a kind that writes no prose.
fn fields(kind: &str) -> &'static [Prose] {
    PROSE
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(&[], |(_, prose)| prose.as_slice())
}

/// The entities of the input that write prose, in input order.
pub(crate) fn writers(input: &PassInput) -> impl Iterator<Item = &PassEntity> {
    input
        .entities
        .iter()
        .filter(|e| !fields(&e.kind).is_empty())
}

/// The written, non-blank texts of a member's value: a text, or each text of a list.
fn texts_of(value: Option<&Value>) -> Vec<&str> {
    let said: Vec<&str> = match value {
        Some(Value::String(text)) => vec![text.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    said.into_iter()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect()
}

/// `(where, text)` of each written, non-blank prose value of `entity`, in its kind's field order. `where` is the field;
/// inside a block, the block's name, or for a row of a block list its place (`step 2`: the list's name made singular
/// and the row's 1-based number), then the member's name unless it is the one prose member of a row.
pub(crate) fn prose(entity: &PassEntity) -> Vec<(String, String)> {
    read(entity, |_| true)
}

/// [`prose`] without what the author cannot change once committed ([`Prose::fixed`]): what W534 reads, since
/// rewriting a committed immutable value is E084.
fn changeable(entity: &PassEntity) -> Vec<(String, String)> {
    read(entity, |path| !path.fixed)
}

fn read(entity: &PassEntity, keep: impl Fn(&Prose) -> bool) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in fields(&entity.kind).iter().filter(|path| keep(path)) {
        match path.blocks.first() {
            None if path.list => out.extend(
                strings(entity, path.name)
                    .iter()
                    .map(|said| said.trim())
                    .filter(|said| !said.is_empty())
                    .map(|said| (path.name.to_string(), said.to_string())),
            ),
            None => out.extend(
                text(entity, path.name).map(|said| (path.name.to_string(), said.to_string())),
            ),
            Some((block, _)) => visit(entity.values.get(*block), &path.blocks, path, "", &mut out),
        }
    }
    out
}

/// A block list's name made singular, for its row's place: `steps` is `step`, `entries` is `entry`.
fn singular(list: &str) -> String {
    match list.strip_suffix("ies") {
        Some(stem) => format!("{stem}y"),
        None => list.strip_suffix('s').unwrap_or(list).to_string(),
    }
}

/// Pushes `(where, text)` of each text `prose` names under `value`, the value of `blocks`' first block, which sits at
/// `place` (empty for a field of the kind).
fn visit(
    value: Option<&Value>,
    blocks: &[(&'static str, bool)],
    prose: &Prose,
    place: &str,
    out: &mut Vec<(String, String)>,
) {
    let Some(((name, rows), rest)) = blocks.split_first() else {
        out.extend(
            texts_of(value)
                .into_iter()
                .map(|said| (place.to_string(), said.to_string())),
        );
        return;
    };
    let at = |part: String| {
        if place.is_empty() {
            part
        } else {
            format!("{place} {part}")
        }
    };
    let records: Vec<(String, &serde_json::Map<String, Value>)> = if *rows {
        let row = singular(name);
        value
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
            .filter_map(|(n, record)| Some((at(format!("{row} {}", n + 1)), record.as_object()?)))
            .collect()
    } else {
        value
            .and_then(Value::as_object)
            .map(|record| (at(name.to_string()), record))
            .into_iter()
            .collect()
    };
    let next = rest.first().map_or(prose.name, |(block, _)| *block);
    for (place, record) in records {
        let place = if rest.is_empty() && !(*rows && prose.alone) {
            format!("{place} {}", prose.name)
        } else {
            place
        };
        visit(record.get(next), rest, prose, &place, out);
    }
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

/// The glossary as W534 reads it: every name and alias the terms accept, folded, and the words they avoid.
struct Glossary<'a> {
    accepted: Vec<String>,
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
        accepted: accepted.into_iter().collect(),
        avoided,
    }
}

/// A place of an entity's prose, read for W534: the place, its text's segments, and where in them it writes an
/// accepted name or alias.
struct Place<'p> {
    place: &'p str,
    text: Segments<'p>,
    names: Vec<Range<usize>>,
}

impl<'p> Place<'p> {
    fn read(place: &'p str, said: &'p str, accepted: &[Segments]) -> Self {
        let text = Segments::of_prose(said);
        let names = accepted.iter().flat_map(|name| text.find(name)).collect();
        Place { place, text, names }
    }

    /// What the place writes for `word`, as it writes it, at its first occurrence outside every accepted name the
    /// place writes: a "ticket" inside "the ticket system", a term's name, is that name.
    fn writes(&self, word: &Segments) -> Option<&'p str> {
        self.text
            .find(word)
            .find(|at| {
                !self
                    .names
                    .iter()
                    .any(|name| name.start <= at.start && at.end <= name.end)
            })
            .map(|at| self.text.written(&at))
    }
}

/// W534: prose (`PROSE`, every field the kinds mark prose) that holds a word or phrase a term avoids, read at the Unicode boundary, outside every
/// accepted name or alias it writes. One warning per entity and avoided word, on the entity, naming the first place
/// that holds it and what it writes there, the avoided word as the avoid list spells it, every term that avoids it and
/// the word each asks for.
fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let glossary = glossary(input);
    if glossary.avoided.is_empty() {
        return Vec::new();
    }
    let accepted: Vec<Segments> = glossary.accepted.iter().map(|w| Segments::of(w)).collect();
    let needles: Vec<(&Avoided, Segments)> = glossary
        .avoided
        .values()
        .map(|avoided| (avoided, Segments::of(&avoided.word)))
        .collect();
    let mut entities: Vec<&PassEntity> = writers(input).collect();
    entities.sort_by(|a, b| a.id.cmp(&b.id));
    let mut found = Vec::new();
    for entity in entities {
        let prose = changeable(entity);
        let places: Vec<Place> = prose
            .iter()
            .map(|(place, said)| Place::read(place, said, &accepted))
            .collect();
        for (avoided, needle) in &needles {
            let Some((place, written)) = places
                .iter()
                .find_map(|p| p.writes(needle).map(|written| (p.place, written)))
            else {
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
                        "{} '{}' writes \"{written}\" in {place}, \"{}\" avoided by {}",
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
