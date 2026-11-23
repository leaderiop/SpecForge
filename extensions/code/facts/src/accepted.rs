//! The accepted side of an API snapshot (ADR 0390 D3): each public entry of a run judged against the committed
//! accepted report of its crate or package, and the `change` its `api_item` fact carries.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::Fact;

use crate::api::{api_facts, fact, insert, ApiEntry, Served};
use crate::report::Repeats;

/// What an entry is against the accepted report: `@code/core`'s vocabulary of `api_item.change`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiChange {
    /// In the run, not in the accepted report.
    Added,
    /// The same comparable signature, as wide as accepted.
    Unchanged,
    /// Another signature.
    Changed,
    /// The same comparable signature with a lower release tag or a narrower visibility.
    Narrowed,
    /// In the accepted report, not in the run.
    Removed,
}

/// Every change, in vocabulary order.
const CHANGES: [ApiChange; 5] = [
    ApiChange::Added,
    ApiChange::Unchanged,
    ApiChange::Changed,
    ApiChange::Narrowed,
    ApiChange::Removed,
];

impl ApiChange {
    /// Every change's name: `added`, `unchanged`, `changed`, `narrowed`, `removed`.
    pub const NAMES: [&'static str; 5] = {
        let mut names = [""; 5];
        let mut at = 0;
        while at < names.len() {
            names[at] = CHANGES[at].as_str();
            at += 1;
        }
        names
    };

    /// Its name: an exhaustive match, so a new change names itself or fails to compile.
    pub const fn as_str(self) -> &'static str {
        match self {
            ApiChange::Added => "added",
            ApiChange::Unchanged => "unchanged",
            ApiChange::Changed => "changed",
            ApiChange::Narrowed => "narrowed",
            ApiChange::Removed => "removed",
        }
    }
}

/// The api-extractor release tags, lowest first.
const TAGS: [&str; 4] = ["Internal", "Alpha", "Beta", "Public"];

/// The tag the accepted side skips: the doc model never lists an `@internal` entry, so its absence is no break.
const INTERNAL: &str = "Internal";

/// The leading words [`comparable`] drops, at most one of each row, in row order.
const PREFIXES: [&[&str]; 3] = [&["export "], &["declare "], &["const ", "let ", "var "]];

/// A signature as both spellings of one declaration compare: whitespace collapsed to one space, a leading `export `,
/// then `declare `, then `const `, `let ` or `var ` dropped, and trailing `;`, `,` and `{` dropped. api-extractor's
/// doc model writes `export declare function f(): void;` and `LIMIT = 3` where its report writes
/// `export function f(): void;` and `const LIMIT = 3;`. The first Rust visibility word (`pub`, `pub(crate)`, …) is
/// dropped too: cargo public-api writes it in the line, and the entry's `visibility` judges it.
pub fn comparable(signature: &str) -> String {
    let collapsed = signature.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut rest = collapsed.as_str();
    for words in PREFIXES {
        if let Some(word) = words.iter().find(|word| rest.starts_with(**word)) {
            rest = &rest[word.len()..];
        }
    }
    without_visibility(rest)
        .trim_end_matches([';', ',', '{', ' '])
        .to_string()
}

/// `text` without its first word `pub` or `pub(…)`, the space after it included.
fn without_visibility(text: &str) -> String {
    let word = text.match_indices("pub").map(|(at, _)| at).find(|&at| {
        (at == 0 || text[..at].ends_with(' '))
            && matches!(text[at + 3..].chars().next(), Some(' ' | '('))
    });
    let Some(at) = word else {
        return text.to_string();
    };
    let mut end = at + 3;
    if text[end..].starts_with('(') {
        end = text[end..]
            .find(')')
            .map_or(text.len(), |close| end + close + 1);
    }
    format!("{}{}", &text[..at], text[end..].trim_start())
}

/// The `api_item` facts of `entries` judged against `accepted`, the entries the committed accepted report holds, each
/// citing the one `served` item its path names, sorted by id (ADR 0390 D3):
///
/// - Only a crate or package on both sides is judged; the facts of any other are [`api_facts`]'s, with no `change`,
///   and an accepted one the run did not cover mints nothing.
/// - An accepted entry tagged `Internal` is skipped.
/// - Both sides are grouped by base id. Inside a group, entries of equal [`comparable`] signature pair first, then the
///   leftovers in document order, never by their `~N`.
/// - A pair of equal comparable signatures is `narrowed` when the run's tag is lower (Public, Beta, Alpha, Internal)
///   or its visibility narrower, else `unchanged`; a pair of different ones is `changed`. A run's entry left alone is
///   `added`; an accepted one is `removed`, minted from the accepted entry with no item and the next repeat suffix
///   after the run's.
/// - `changed`, `narrowed` and `removed` carry `accepted_signature`, `accepted_tag` when the accepted entry has a tag,
///   and `accepted_visibility` when its visibility differs.
///
/// A member with no tag of its own has its container's: the report reader resolves it, the doc model states it.
pub fn api_facts_against(
    entries: &[ApiEntry],
    accepted: &[ApiEntry],
    served: &[Fact],
) -> Vec<Fact> {
    let side = |entries: &[ApiEntry]| -> BTreeSet<_> {
        entries
            .iter()
            .map(|e| (e.scheme, e.container.clone()))
            .collect()
    };
    let (fresh_side, accepted_side) = (side(entries), side(accepted));
    let judged = |e: &ApiEntry| {
        let container = (e.scheme, e.container.clone());
        fresh_side.contains(&container) && accepted_side.contains(&container)
    };
    let alone: Vec<ApiEntry> = entries.iter().filter(|e| !judged(e)).cloned().collect();
    let mut facts: BTreeMap<String, Fact> = api_facts(&alone, served)
        .into_iter()
        .map(|fact| (fact.id.clone(), fact))
        .collect();

    let served = Served::new(served);
    let mut repeats = Repeats::default();
    let mut groups: BTreeMap<String, Group> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for entry in entries.iter().filter(|e| judged(e)) {
        let (base, item, written) = served.minted(entry);
        if seen.insert((base.clone(), entry.signature.as_str())) {
            let id = repeats.id(base.clone());
            let group = groups.entry(base).or_default();
            group.fresh.push(Minted {
                entry,
                id,
                item,
                written,
            });
        }
    }
    let mut seen = BTreeSet::new();
    for entry in accepted
        .iter()
        .filter(|e| judged(e) && e.tag.as_deref() != Some(INTERNAL))
    {
        let (base, _, written) = served.minted(entry);
        if seen.insert((base.clone(), entry.signature.as_str())) {
            let group = groups.entry(base).or_default();
            group.accepted.push((entry, written));
        }
    }

    for (base, group) in groups {
        let (pairs, removed) = pair(&group);
        for (minted, paired) in group.fresh.into_iter().zip(pairs) {
            let fresh = minted.entry;
            let mut judged = fact(minted.id, fresh, minted.item, minted.written);
            judged = match paired.map(|at| group.accepted[at].0) {
                None => judged.field("change", ApiChange::Added.as_str()),
                Some(old) if comparable(&old.signature) != comparable(&fresh.signature) => {
                    against(judged, ApiChange::Changed, old, Some(fresh))
                }
                Some(old) if narrower(fresh, old) => {
                    against(judged, ApiChange::Narrowed, old, Some(fresh))
                }
                Some(_) => judged.field("change", ApiChange::Unchanged.as_str()),
            };
            insert(&mut facts, judged);
        }
        for at in removed {
            let (old, written) = &group.accepted[at];
            let id = repeats.id(base.clone());
            let gone = fact(id, old, None, written.clone());
            insert(&mut facts, against(gone, ApiChange::Removed, old, None));
        }
    }
    facts.into_values().collect()
}

/// One base id's entries: the run's, minted, and the accepted ones with their paths as written, in document order.
#[derive(Default)]
struct Group<'e> {
    fresh: Vec<Minted<'e>>,
    accepted: Vec<(&'e ApiEntry, String)>,
}

/// A run's entry, its id taken, the item it cites and its path as written.
struct Minted<'e> {
    entry: &'e ApiEntry,
    id: String,
    item: Option<String>,
    written: String,
}

/// For each of `group`'s run entries, the accepted entry it pairs with, and the accepted entries left alone: equal
/// comparable signatures first, then the leftovers of both sides in document order.
fn pair(group: &Group) -> (Vec<Option<usize>>, Vec<usize>) {
    let keys: Vec<String> = group
        .accepted
        .iter()
        .map(|(e, _)| comparable(&e.signature))
        .collect();
    let mut taken = vec![false; keys.len()];
    let mut pairs: Vec<Option<usize>> = group
        .fresh
        .iter()
        .map(|minted| {
            let key = comparable(&minted.entry.signature);
            let at = (0..keys.len()).find(|&at| !taken[at] && keys[at] == key)?;
            taken[at] = true;
            Some(at)
        })
        .collect();
    let mut left = (0..keys.len()).filter(|&at| !taken[at]);
    for slot in pairs.iter_mut().filter(|slot| slot.is_none()) {
        *slot = left.next();
    }
    (pairs, left.collect())
}

/// Whether `fresh` is narrower than `accepted`: a lower release tag, or a visibility later in `Visibility::NAMES`.
fn narrower(fresh: &ApiEntry, accepted: &ApiEntry) -> bool {
    let rank = |tag: &Option<String>| TAGS.iter().position(|t| tag.as_deref() == Some(*t));
    let lower =
        matches!((rank(&fresh.tag), rank(&accepted.tag)), (Some(new), Some(old)) if new < old);
    lower || fresh.visibility < accepted.visibility
}

/// `fact` marked `change` with what `accepted` said: its signature, its tag when it has one, its visibility when it
/// differs from `fresh`'s.
fn against(
    mut fact: Fact,
    change: ApiChange,
    accepted: &ApiEntry,
    fresh: Option<&ApiEntry>,
) -> Fact {
    fact = fact
        .field("change", change.as_str())
        .field("accepted_signature", accepted.signature.clone());
    if let Some(tag) = &accepted.tag {
        fact = fact.field("accepted_tag", tag.clone());
    }
    if fresh.is_some_and(|fresh| fresh.visibility != accepted.visibility) {
        fact = fact.field("accepted_visibility", accepted.visibility.as_str());
    }
    fact
}
