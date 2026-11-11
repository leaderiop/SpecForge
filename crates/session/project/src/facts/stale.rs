//! A fact store a rename marked stale (ADR 0346 D7): for each name a rename rewrote where the store's facts hold it,
//! the ledger holds `<collector> stale:<key>:<field>:<written>:<now> <digest>` for the store's bytes, `written` the
//! name the store holds and `now` the name the last rename wrote. Until the collector collects again, exactly those
//! names are read as now written, every place the store records in a file a rename rewrote is shifted by what the
//! renames wrote there, and the store is W197.
//!
//! The renames reach the serve on the store's served text: the collector's bytes, then the renames as one more JSON
//! value. The session compares served texts, so a ledger change re-serves the store; the store file keeps the
//! collector's bytes. What the renames wrote in a file is derived from the store and the renames alone ([`follow`]),
//! so a rename writes exactly what the serve shifts by: it plans its own edits from the stale lines it writes.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::FactLocation;

use super::{Respelled, Store};
use crate::harness::{Ledger, Line};

/// The renames a stale store is read through: field → the name its facts hold → the name a rename wrote.
pub type Renames = BTreeMap<String, BTreeMap<String, String>>;

/// The renames the ledger `recorded` marks the store of `collector` filed under `key` stale with, for its `bytes`:
/// each stale line of that store whose digest is of exactly those bytes.
pub(super) fn renames(recorded: &Ledger, collector: &str, key: &str, bytes: &[u8]) -> Renames {
    let mut renames = Renames::new();
    for line in recorded.lines() {
        let Some(stale) = line.stale_of() else {
            continue;
        };
        if stale.collector == collector
            && stale.key == key
            && *line == Line::stale(collector, key, bytes, stale.field, stale.written, stale.now)
        {
            renames
                .entry(stale.field.to_string())
                .or_default()
                .insert(stale.written.to_string(), stale.now.to_string());
        }
    }
    renames
}

/// The served text of a store whose bytes are `text`: those bytes, then `renames` when there are any.
pub(super) fn carried(mut text: String, renames: &Renames) -> String {
    if !renames.is_empty() {
        text.push_str(&serde_json::to_string(renames).expect("renames serialize"));
        text.push('\n');
    }
    text
}

/// The served text `text` read: the store, the collector's bytes it was read from, and the renames it carries.
pub(super) fn split(text: &str) -> Result<(Store, &str, Renames), serde_json::Error> {
    let mut values = serde_json::Deserializer::from_str(text).into_iter::<Store>();
    let store = match values.next() {
        Some(store) => store?,
        None => serde_json::from_str::<Store>(text)?,
    };
    let carried = text[values.byte_offset()..].trim_start();
    let bytes = &text[..text.len() - carried.len()];
    let renames = if carried.is_empty() {
        Renames::new()
    } else {
        serde_json::from_str(carried)?
    };
    Ok((store, bytes, renames))
}

/// A span of the collected text, by its start and end (line, byte column): the key the edits of a file are sorted by.
type Span = (usize, usize, usize, usize);

/// `store` read through `renames`: every place a fact names a renamed name in a field `renames` lists is a place a
/// rename rewrote (the collector's name there replaced by the name written now); every place the store records in a
/// file so rewritten is shifted by those rewrites, exactly; and each such name is read as now written. Then the W197
/// of the store `key` when there are renames, and, by file, each place rewritten as it is in the text the renames
/// left.
pub(super) fn follow(
    key: &str,
    store: &mut Store,
    renames: &Renames,
) -> (Option<Diagnostic>, BTreeMap<String, Vec<Respelled>>) {
    if renames.is_empty() {
        return (None, BTreeMap::new());
    }
    // What the renames wrote, by file, in the collected text: (written, now) at each span.
    let mut edits: BTreeMap<String, BTreeMap<Span, (String, String)>> = BTreeMap::new();
    let mut renamed: BTreeSet<(String, String)> = BTreeSet::new();
    for fact in &store.facts {
        for (field, names) in renames {
            let Some(places) = fact.named_at.get(field) else {
                continue;
            };
            for (written, now) in names {
                for place in places.get(written).into_iter().flatten() {
                    edits
                        .entry(place.file.clone())
                        .or_default()
                        .insert(span(place), (written.clone(), now.clone()));
                    renamed.insert((written.clone(), now.clone()));
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
    (Some(w197(key, &renamed, &files)), respelled)
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
fn shift(place: &mut FactLocation, edits: &BTreeMap<Span, (String, String)>) {
    (place.start_line, place.start_col) = moved((place.start_line, place.start_col), edits);
    (place.end_line, place.end_col) = moved((place.end_line, place.end_col), edits);
}

/// The point `at` of the collected text in the text `edits` leave: unchanged before an edit, carried by the length an
/// edit changes after it, and at the end of what an edit wrote inside it. The edits are applied from the last: each
/// moves only what lies after it, so the ones before it keep their positions.
fn moved(at: (usize, usize), edits: &BTreeMap<Span, (String, String)>) -> (usize, usize) {
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
    fn a_served_text_gives_back_the_collectors_bytes_and_its_renames() {
        let bytes = Store::new("node", "@t/node", "k", Vec::new(), BTreeMap::new()).bytes();
        let bytes = String::from_utf8(bytes).unwrap();
        let renames: Renames = [(
            "covers".to_string(),
            [("T1".to_string(), "T2".to_string())].into(),
        )]
        .into();

        let served = carried(bytes.clone(), &renames);
        let (store, read, carried_back) = split(&served).unwrap();

        assert_eq!((store.key.as_str(), read), ("k", bytes.as_str()));
        assert_eq!(carried_back, renames);
        let (_, plain, none) = split(&bytes).unwrap();
        assert_eq!((plain, none), (bytes.as_str(), Renames::new()));
    }
}
