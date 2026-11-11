//! A fact store a rename marked stale (ADR 0346 D7): for each name a rename rewrote where the store's facts hold it,
//! the ledger holds `<collector> stale:<key>:<field>:<written>:<now> <digest>` for the store's bytes, `written` the
//! name the store holds and `now` the name the last rename wrote. Until the collector collects again, exactly those
//! names are read as now written and the store is W197.
//!
//! What the renames wrote in a source file is derived from every stale store the ledger serves ([`rewritten_by`]):
//! each place a store's facts name a held name its stale lines rename, in the text that store collected (by its
//! digest of the file). Every store placed in that file, renamed or not, collected from that same text, is served
//! with its places shifted by all of it ([`follow`]), so a rename writes exactly what the serve shifts by: it plans
//! its own edits from the served places and the stale lines it writes.
//!
//! The renames reach the serve on the store's served text: the collector's bytes, then what the ledger says of it as
//! one more JSON value. The session compares served texts, so a ledger change re-serves the store; the store file
//! keeps the collector's bytes.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::FactLocation;

use super::{Respelled, Store};
use crate::harness::{Ledger, Line};

/// The renames a stale store is read through: field → the name its facts hold → the name a rename wrote.
pub type Renames = BTreeMap<String, BTreeMap<String, String>>;

/// The names a store's stale lines rename more than one way: field → the name its facts hold → each name written.
/// Two branches each renaming one name, merged (`merge=union`), leave two lines for it.
pub type Conflicts = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

/// What the ledger says of a store beside its bytes, carried on its served text after them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Carried {
    /// The renames it is read through; none when any name is renamed two ways.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub renames: Renames,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub conflicts: Conflicts,
    /// What every stale store's renames wrote, file by file ([`Rewrites`]), as places in order.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rewritten: BTreeMap<String, BTreeMap<String, Vec<Wrote>>>,
}

impl Carried {
    fn is_empty(&self) -> bool {
        self.renames.is_empty() && self.conflicts.is_empty() && self.rewritten.is_empty()
    }

    /// `rewrites` carried.
    pub fn rewriting(mut self, rewrites: &Rewrites) -> Carried {
        self.rewritten = rewrites
            .iter()
            .map(|(file, texts)| {
                let texts = texts
                    .iter()
                    .map(|(digest, at)| {
                        let at = at
                            .iter()
                            .map(|(span, (held, now))| (*span, held.clone(), now.clone()))
                            .collect();
                        (digest.clone(), at)
                    })
                    .collect();
                (file.clone(), texts)
            })
            .collect();
        self
    }

    /// The edits of the file `file` the text whose digest is `digest` was rewritten by.
    fn edits(&self, file: &str, digest: &str) -> Option<Edits> {
        let at = self.rewritten.get(file)?.get(digest)?;
        Some(
            at.iter()
                .map(|(span, held, now)| (*span, (held.clone(), now.clone())))
                .collect(),
        )
    }
}

/// One place a rename wrote, as carried: the span, the name held there, the name written now.
type Wrote = (Span, String, String);

/// What the renames wrote in one text: each place, a span of the text collected, and the name held there and the name
/// written now.
pub(super) type Edits = BTreeMap<Span, (String, String)>;

/// What the renames wrote in source files, derived from every stale store the ledger serves: file → the digest of the
/// text collected → its [`Edits`].
pub type Rewrites = BTreeMap<String, BTreeMap<String, Edits>>;

/// Add to `rewrites` what `renames`, the renames of `store`, wrote: every place a fact of it names a held name a
/// rename lists in that field, in the text of the file the store collected (by its digest). A text one place of which
/// two stores say was written two ways goes in `torn`.
pub(super) fn rewritten_by(
    store: &Store,
    renames: &Renames,
    rewrites: &mut Rewrites,
    torn: &mut BTreeSet<(String, String)>,
) {
    for fact in &store.facts {
        for (field, names) in renames {
            let Some(places) = fact.named_at.get(field) else {
                continue;
            };
            for (held, now) in names {
                for place in places.get(held).into_iter().flatten() {
                    let Some(digest) = store.sources.get(&place.file) else {
                        continue;
                    };
                    let edits = rewrites
                        .entry(place.file.clone())
                        .or_default()
                        .entry(digest.clone())
                        .or_default();
                    let wrote = (held.clone(), now.clone());
                    if edits
                        .insert(span(place), wrote.clone())
                        .is_some_and(|before| before != wrote)
                    {
                        torn.insert((place.file.clone(), digest.clone()));
                    }
                }
            }
        }
    }
}

/// `rewrites` without a text it cannot say exactly: one in `torn`, or one whose places overlap. Its stores' places are
/// served as collected there, and the file is not the text they are in.
pub(super) fn settled(mut rewrites: Rewrites, torn: &BTreeSet<(String, String)>) -> Rewrites {
    for (file, texts) in &mut rewrites {
        texts.retain(|digest, edits| {
            let spans: Vec<&Span> = edits.keys().collect();
            let overlap = spans
                .windows(2)
                .any(|pair| (pair[0].2, pair[0].3) > (pair[1].0, pair[1].1));
            !overlap && !torn.contains(&(file.clone(), digest.clone()))
        });
    }
    rewrites.retain(|_, texts| !texts.is_empty());
    rewrites
}

/// What the ledger `recorded` says of the store of `collector` filed under `key`, for its `bytes`: the renames of each
/// stale line of that store whose digest is of exactly those bytes; when one name held has lines to two names, those
/// in `conflicts`, and no rename at all.
pub(super) fn carried(recorded: &Ledger, collector: &str, key: &str, bytes: &[u8]) -> Carried {
    let mut written: Conflicts = Conflicts::new();
    for line in recorded.lines() {
        let Some(stale) = line.stale_of() else {
            continue;
        };
        if stale.collector == collector
            && stale.key == key
            && *line == Line::stale(collector, key, bytes, stale.field, stale.written, stale.now)
        {
            written
                .entry(stale.field.to_string())
                .or_default()
                .entry(stale.written.to_string())
                .or_default()
                .insert(stale.now.to_string());
        }
    }
    let mut conflicts = written.clone();
    for names in conflicts.values_mut() {
        names.retain(|_, now| now.len() > 1);
    }
    conflicts.retain(|_, names| !names.is_empty());
    if !conflicts.is_empty() {
        return Carried {
            conflicts,
            ..Carried::default()
        };
    }
    let renames = written
        .into_iter()
        .map(|(field, names)| {
            let names = names
                .into_iter()
                .filter_map(|(held, now)| Some((held, now.into_iter().next()?)))
                .collect();
            (field, names)
        })
        .collect();
    Carried {
        renames,
        ..Carried::default()
    }
}

/// The served text of a store whose bytes are `text`: those bytes, then what the ledger says of it when it says
/// anything.
pub(super) fn served(mut text: String, carried: &Carried) -> String {
    if !carried.is_empty() {
        text.push_str(&serde_json::to_string(carried).expect("a store's renames serialize"));
        text.push('\n');
    }
    text
}

/// The served text `text` read: the store, the collector's bytes it was read from, and what the ledger says of it.
pub(super) fn split(text: &str) -> Result<(Store, &str, Carried), serde_json::Error> {
    let mut values = serde_json::Deserializer::from_str(text).into_iter::<Store>();
    let store = match values.next() {
        Some(store) => store?,
        None => serde_json::from_str::<Store>(text)?,
    };
    let carried = text[values.byte_offset()..].trim_start();
    let bytes = &text[..text.len() - carried.len()];
    let carried = if carried.is_empty() {
        Carried::default()
    } else {
        serde_json::from_str(carried)?
    };
    Ok((store, bytes, carried))
}

/// W202: the store `key`'s stale lines rename a name it holds two ways (`conflicts`); none of its renames is served.
pub(super) fn w202(key: &str, conflicts: &Conflicts) -> Diagnostic {
    let named: Vec<String> = conflicts
        .values()
        .flat_map(|names| names.iter())
        .map(|(held, now)| {
            let now: Vec<String> = now.iter().map(|n| format!("'{n}'")).collect();
            format!("'{held}' ({})", now.join(", "))
        })
        .collect();
    Diagnostic::new(
        codes::W202,
        format!(
            "fact store '{key}': specforge-harness holds two renames of {}; none of its renames is served, until \
             `specforge collect`",
            named.join(" and of ")
        ),
    )
    .with_suggestion("run `specforge collect`")
}

/// A span of the collected text, by its start and end (line, byte column): the key the edits of a file are sorted by.
pub(super) type Span = (usize, usize, usize, usize);

/// `store` read through `carried`: every place it records in a file the renames rewrote, in the text it collected,
/// shifted by what they wrote there ([`Carried::edits`]), exactly; each name its own renames list read as now
/// written. Then the W197 of the store `key` when it has renames, and, by file, each place rewritten as it is in the
/// text the renames left.
pub(super) fn follow(
    key: &str,
    store: &mut Store,
    carried: &Carried,
) -> (Option<Diagnostic>, BTreeMap<String, Vec<Respelled>>) {
    let renames = &carried.renames;
    let edits: BTreeMap<String, Edits> = store
        .sources
        .iter()
        .filter_map(|(file, digest)| Some((file.clone(), carried.edits(file, digest)?)))
        .collect();
    if renames.is_empty() && edits.is_empty() {
        return (None, BTreeMap::new());
    }
    // What its own renames moved: each (held, now) a fact writes at a place.
    let mut renamed: BTreeSet<(String, String)> = BTreeSet::new();
    for fact in &store.facts {
        for (field, names) in renames {
            let Some(places) = fact.named_at.get(field) else {
                continue;
            };
            for (held, now) in names {
                if places.get(held).is_some_and(|at| !at.is_empty()) {
                    renamed.insert((held.clone(), now.clone()));
                }
            }
        }
    }
    for fact in &mut store.facts {
        let named = fact.named_at.values_mut().flat_map(|ids| ids.values_mut());
        for place in fact.at.iter_mut().chain(named.flatten()) {
            if let Some(edits) = edits.get(&place.file) {
                shift(place, edits);
            }
        }
        for (field, names) in renames {
            let Some(places) = fact.named_at.get_mut(field) else {
                continue;
            };
            // A rename rewrites only a name written at a place: the name the fact holds there. All at once, so a
            // name one rename freed and another took is read for each.
            let held: BTreeSet<String> = places.keys().cloned().collect();
            for (written, at) in std::mem::take(places) {
                let now = names.get(&written).cloned().unwrap_or(written);
                places.entry(now).or_default().extend(at);
            }
            if let Some(value) = fact.fields.get_mut(field) {
                respell(value, names, &held);
            }
        }
    }
    let respelled = edits
        .iter()
        .map(|(file, at)| {
            let respelled = at
                .iter()
                .map(
                    |(&(start_line, start_col, end_line, end_col), (written, now))| {
                        let mut place = FactLocation {
                            file: file.clone(),
                            start_line,
                            start_col,
                            end_line,
                            end_col,
                        };
                        shift(&mut place, at);
                        Respelled {
                            at: place,
                            written: written.clone(),
                            now: now.clone(),
                        }
                    },
                )
                .collect();
            (file.clone(), respelled)
        })
        .collect();
    let files: BTreeSet<&String> = edits.keys().collect();
    let stale = (!renames.is_empty()).then(|| w197(key, &renamed, &files));
    (stale, respelled)
}

fn span(place: &FactLocation) -> Span {
    (
        place.start_line,
        place.start_col,
        place.end_line,
        place.end_col,
    )
}

/// `place`, a span of the collected text, as a span of the text `edits` (spans of the collected text, each replaced
/// by its `now`) leave.
fn shift(place: &mut FactLocation, edits: &Edits) {
    (place.start_line, place.start_col) = moved((place.start_line, place.start_col), edits);
    (place.end_line, place.end_col) = moved((place.end_line, place.end_col), edits);
}

/// The point `at` of the collected text in the text `edits` leave: unchanged before an edit, carried by the length an
/// edit changes after it, and at the end of what an edit wrote inside it. The edits are applied from the last: each
/// moves only what lies after it, so the ones before it keep their positions.
fn moved(at: (usize, usize), edits: &Edits) -> (usize, usize) {
    let (mut line, mut col) = at;
    for (&(start_line, start_col, end_line, end_col), (_, now)) in edits.iter().rev() {
        if (line, col) <= (start_line, start_col) {
            continue;
        }
        if (line, col) < (end_line, end_col) {
            (line, col) = (start_line, start_col + now.len());
        } else if line == end_line {
            (line, col) = (start_line, start_col + now.len() + (col - end_col));
        } else {
            line -= end_line - start_line;
        }
    }
    (line, col)
}

/// `value`, or each item of it, that is one of the names `held` lists a rename of in `names`, as the rename wrote it.
fn respell(
    value: &mut serde_json::Value,
    names: &BTreeMap<String, String>,
    held: &BTreeSet<String>,
) {
    match value {
        serde_json::Value::Array(items) => {
            items.iter_mut().for_each(|item| respell(item, names, held))
        }
        serde_json::Value::String(text) if held.contains(text.as_str()) => {
            if let Some(now) = names.get(text.as_str()) {
                *text = now.clone();
            }
        }
        _ => {}
    }
}

/// W197: the store `key` is stale, what a rename moved in it, and the files its places are shifted in.
fn w197(key: &str, renamed: &BTreeSet<(String, String)>, files: &BTreeSet<&String>) -> Diagnostic {
    let moved = match renamed.iter().next() {
        None => String::new(),
        Some((old, new)) if renamed.len() == 1 => format!(" ('{old}' → '{new}')"),
        Some((old, new)) => format!(" ('{old}' → '{new}', and {} more)", renamed.len() - 1),
    };
    let places = if files.is_empty() {
        String::new()
    } else {
        format!(
            "; its places in {} are served as the rename left them",
            files
                .iter()
                .map(|f| f.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Diagnostic::new(
        codes::W197,
        format!(
            "fact store '{key}' is stale: a rename moved what its facts name{moved} after the collect{places}, until \
             `specforge collect`"
        ),
    )
    .with_suggestion("run `specforge collect`")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_served_text_gives_back_the_collectors_bytes_and_what_the_ledger_says() {
        let bytes = Store::new("node", "@t/node", "k", Vec::new(), BTreeMap::new()).bytes();
        let bytes = String::from_utf8(bytes).unwrap();
        let carried_there = Carried {
            renames: [(
                "covers".to_string(),
                [("T1".to_string(), "T2".to_string())].into(),
            )]
            .into(),
            ..Carried::default()
        };

        let text = served(bytes.clone(), &carried_there);
        let (store, read, carried_back) = split(&text).unwrap();

        assert_eq!((store.key.as_str(), read), ("k", bytes.as_str()));
        assert_eq!(carried_back, carried_there);
        let (_, plain, none) = split(&bytes).unwrap();
        assert_eq!((plain, none), (bytes.as_str(), Carried::default()));
    }

    #[test]
    fn a_name_held_with_lines_to_two_names_is_a_conflict_and_nothing_is_renamed() {
        let bytes = b"{}";
        let mut ledger = Ledger::default();
        for (held, now) in [("T1", "T2"), ("T1", "T3"), ("T5", "T6")] {
            ledger.insert(Line::stale("node", "k", bytes, "covers", held, now));
        }

        let carried = carried(&ledger, "node", "k", bytes);

        assert!(carried.renames.is_empty());
        assert_eq!(
            carried.conflicts,
            Conflicts::from([(
                "covers".to_string(),
                [(
                    "T1".to_string(),
                    BTreeSet::from(["T2".to_string(), "T3".to_string()])
                )]
                .into()
            )])
        );
    }

    #[test]
    fn a_text_two_stores_say_was_written_two_ways_or_overlapping_is_not_served() {
        let span = (1, 11, 1, 13);
        let mut rewrites = Rewrites::new();
        let edits = |at: &[(Span, &str)]| -> Edits {
            at.iter()
                .map(|(span, now)| (*span, ("T1".to_string(), now.to_string())))
                .collect()
        };
        rewrites.insert(
            "a.js".to_string(),
            [
                ("sha256:torn".to_string(), edits(&[(span, "T2")])),
                (
                    "sha256:overlap".to_string(),
                    edits(&[(span, "T2"), ((1, 12, 1, 14), "T3")]),
                ),
                (
                    "sha256:kept".to_string(),
                    edits(&[(span, "T2"), ((1, 14, 1, 16), "T3")]),
                ),
            ]
            .into(),
        );
        let torn = BTreeSet::from([("a.js".to_string(), "sha256:torn".to_string())]);

        let settled = settled(rewrites, &torn);

        let kept: Vec<&String> = settled["a.js"].keys().collect();
        assert_eq!(kept, ["sha256:kept"]);
    }

    #[test]
    fn two_stores_writing_one_place_two_ways_tear_its_text() {
        let place = FactLocation {
            file: "a.js".into(),
            start_line: 1,
            start_col: 11,
            end_line: 1,
            end_col: 13,
        };
        let store = |collector: &str| {
            let fact = specforge_protocol_types::Fact::new("test", format!("js:a#{collector}"))
                .name("covers", "T1", place.clone());
            Store::new(
                collector,
                "@t/node",
                "k",
                vec![fact],
                [("a.js".to_string(), "sha256:one".to_string())].into(),
            )
        };
        let renames = |now: &str| -> Renames {
            [(
                "covers".to_string(),
                [("T1".to_string(), now.to_string())].into(),
            )]
            .into()
        };
        let (mut rewrites, mut torn) = (Rewrites::new(), BTreeSet::new());

        rewritten_by(&store("node"), &renames("T2"), &mut rewrites, &mut torn);
        rewritten_by(&store("unit"), &renames("T2"), &mut rewrites, &mut torn);
        assert!(torn.is_empty(), "the same write twice agrees");
        rewritten_by(&store("other"), &renames("T3"), &mut rewrites, &mut torn);

        assert_eq!(
            torn,
            BTreeSet::from([("a.js".to_string(), "sha256:one".to_string())])
        );
        assert!(settled(rewrites, &torn).is_empty());
    }
}
