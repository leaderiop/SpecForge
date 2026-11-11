//! A fact store a rename marked stale (ADR 0346 D7): for each name a rename rewrote where the store's facts hold it,
//! the ledger holds `<collector> stale:<key>:<field>:<written>:<now> <digest>` for the store's bytes. Until the
//! collector collects again, exactly those names are read as the rename wrote them, and the store is W197.
//!
//! The renames reach the serve on the store's served text: the collector's bytes, then the renames as one more JSON
//! value. The session compares served texts, so a ledger change re-serves the store; the store file keeps the
//! collector's bytes.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};

use super::Store;
use crate::harness::{Ledger, Line};

/// The renames a stale store is read through: field → the name its facts hold → the name a rename wrote.
pub(super) type Renames = BTreeMap<String, BTreeMap<String, String>>;

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

/// Every name `store`'s facts hold in a field `renames` lists, read as the rename wrote it, its places moved along;
/// the W197 of the store `key` when there are renames, and the source files a rename rewrote a name in.
pub(super) fn follow(
    key: &str,
    store: &mut Store,
    renames: &Renames,
) -> (Option<Diagnostic>, BTreeSet<String>) {
    if renames.is_empty() {
        return (None, BTreeSet::new());
    }
    let mut renamed: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut files: BTreeSet<String> = BTreeSet::new();
    for fact in &mut store.facts {
        for (field, names) in renames {
            let Some(places) = fact.named_at.get_mut(field) else {
                continue;
            };
            for (written, now) in names {
                // A rename rewrites only a name written at a place: the name the fact holds there.
                let Some(at) = places.remove(written) else {
                    continue;
                };
                files.extend(at.iter().map(|place| place.file.clone()));
                places.entry(now.clone()).or_default().extend(at);
                if let Some(value) = fact.fields.get_mut(field) {
                    respell(value, written, now);
                }
                renamed.insert((written, now));
            }
        }
    }
    (Some(w197(key, &renamed, &files)), files)
}

/// `value`, or each item of it, that is the string `written`, as `now`.
fn respell(value: &mut serde_json::Value, written: &str, now: &str) {
    match value {
        serde_json::Value::Array(items) => items
            .iter_mut()
            .for_each(|item| respell(item, written, now)),
        serde_json::Value::String(text) if text == written => *text = now.to_string(),
        _ => {}
    }
}

/// W197: the store `key` is stale, and what a rename moved in it.
fn w197(key: &str, renamed: &BTreeSet<(&str, &str)>, files: &BTreeSet<String>) -> Diagnostic {
    let moved = match renamed.iter().next() {
        None => String::new(),
        Some((old, new)) if renamed.len() == 1 => format!(" ('{old}' → '{new}')"),
        Some((old, new)) => format!(" ('{old}' → '{new}', and {} more)", renamed.len() - 1),
    };
    let places = if files.is_empty() {
        "the files its collector read".to_string()
    } else {
        files.iter().cloned().collect::<Vec<_>>().join(", ")
    };
    Diagnostic::new(
        codes::W197,
        format!(
            "fact store '{key}' is stale: a rename moved what its facts name{moved} after the collect; its places in \
             {places} may be off until `specforge collect`"
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
