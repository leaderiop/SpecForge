//! A fact store a rename marked stale (ADR 0346 D7): the ledger holds `<collector> stale:<key> <digest>` for its
//! bytes, so the store was collected before a rename moved what its facts name. Until the collector collects again,
//! its references are read through the ledger's moves, and the store is W197.

use std::collections::{BTreeMap, BTreeSet};

use specforge_common::{Diagnostic, codes};
use specforge_graph::{FieldForms, Reading};
use specforge_protocol_types::FieldType;

use super::Store;
use crate::harness::Ledger;

/// The store `text` as a session serves it when the ledger marks it stale: its `moved` set to the ledger's moves.
/// The text unchanged when it is no store (the serve says W192) or the ledger records no move.
pub(super) fn marked(text: String, ledger: &Ledger) -> String {
    let moved = moves(ledger);
    match serde_json::from_str::<Store>(&text) {
        Ok(mut store) if !moved.is_empty() => {
            store.moved = moved;
            serde_json::to_string(&store).expect("a store serializes")
        }
        _ => text,
    }
}

/// Each moved path and the path it is now, from the ledger's `new moved:old` records, a chain followed to its end.
fn moves(ledger: &Ledger) -> BTreeMap<String, String> {
    let direct: BTreeMap<&str, &str> = ledger
        .lines()
        .filter_map(|line| Some((line.moved_from()?, line.subject.as_str())))
        .collect();
    direct
        .iter()
        .map(|(old, new)| {
            let mut now = *new;
            // A chain is no longer than the records; a cycle (a rename and back) stops there.
            for _ in 0..direct.len() {
                match direct.get(now) {
                    Some(next) if next != old => now = next,
                    _ => break,
                }
            }
            (old.to_string(), now.to_string())
        })
        .collect()
}

/// Every reference value of `store`'s facts that spells a moved path, or a suffix of one, read as the same suffix of
/// the path it moved to, with its name places moved along; and the W197 of the store `key` when it holds `moved`.
pub(super) fn followed(key: &str, store: &mut Store, forms: &FieldForms) -> Option<Diagnostic> {
    if store.moved.is_empty() {
        return None;
    }
    let mut renamed: Vec<(String, String)> = Vec::new();
    let mut files: BTreeSet<String> = BTreeSet::new();
    for fact in &mut store.facts {
        for (name, value) in &mut fact.fields {
            let referenced = matches!(
                forms.reading(&fact.kind, name),
                Reading::Declared(form)
                    if matches!(form.field_type(), FieldType::Reference | FieldType::ReferenceList)
            );
            if !referenced {
                continue;
            }
            let texts: Vec<&mut serde_json::Value> = match value {
                serde_json::Value::Array(items) => items.iter_mut().collect(),
                single => vec![single],
            };
            for text in texts {
                let Some(old) = text.as_str().map(str::to_string) else {
                    continue;
                };
                let Some(new) = follow(&old, &store.moved) else {
                    continue;
                };
                if let Some(places) = fact.named_at.get_mut(name.as_str())
                    && let Some(at) = places.remove(&old)
                {
                    files.extend(at.iter().map(|place| place.file.clone()));
                    places.entry(new.clone()).or_default().extend(at);
                }
                *text = serde_json::Value::String(new.clone());
                renamed.push((old, new));
            }
        }
    }
    Some(w197(key, &renamed, &files))
}

/// `value` read through `moved`: the moved path it spells, or ends a suffix of at a `/` (a short name a fact holds,
/// or one under it), as the same number of trailing segments of the path it moved to, what follows kept.
fn follow(value: &str, moved: &BTreeMap<String, String>) -> Option<String> {
    let segments: Vec<&str> = value.split('/').collect();
    // The longest written head first: `02/TR1` is matched as `02/TR1`, not as `02`.
    for written in (1..=segments.len()).rev() {
        let head = segments[..written].join("/");
        let Some((_, new)) = moved
            .iter()
            .find(|(old, _)| **old == head || old.ends_with(&format!("/{head}")))
        else {
            continue;
        };
        let new: Vec<&str> = new.split('/').collect();
        let mut read = new[new.len().saturating_sub(written)..].to_vec();
        read.extend(&segments[written..]);
        return Some(read.join("/"));
    }
    None
}

/// W197: the store `key` is stale, and what a rename moved in it.
fn w197(key: &str, renamed: &[(String, String)], files: &BTreeSet<String>) -> Diagnostic {
    let moved = match renamed {
        [] => String::new(),
        [(old, new)] => format!(" ('{old}' → '{new}')"),
        [(old, new), rest @ ..] => format!(" ('{old}' → '{new}', and {} more)", rest.len()),
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
    fn a_value_is_followed_by_its_written_suffix() {
        let moved: BTreeMap<String, String> = [
            ("v1/02/TR1".to_string(), "v1/02/TR2".to_string()),
            ("a/old".to_string(), "b/c/new".to_string()),
        ]
        .into();
        let read = |value: &str| follow(value, &moved);
        assert_eq!(read("v1/02/TR1").as_deref(), Some("v1/02/TR2"));
        assert_eq!(read("TR1").as_deref(), Some("TR2"));
        assert_eq!(read("02/TR1").as_deref(), Some("02/TR2"));
        assert_eq!(read("TR1/step").as_deref(), Some("TR2/step"));
        assert_eq!(read("old").as_deref(), Some("new"));
        assert_eq!(read("a/old").as_deref(), Some("c/new"));
        assert_eq!(read("XTR1"), None);
        assert_eq!(read("TR3"), None);
    }
}
