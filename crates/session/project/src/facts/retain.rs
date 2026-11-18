//! The archive of the fact stores a collect supersedes (ADR 0376 D4):
//! `.specforge/facts-history/<seq>-<key>/<collector>.json`, outside `.specforge/facts/`, so a compile never lists,
//! serves or warns about an archived store. `<seq>` is six decimal digits, one more than the largest of the archive,
//! since a store has no clock; each collector keeps its newest `facts.retain`. Every path is built from a collector's
//! name, a store key and a number, never from what a listing or a ledger line wrote, so nothing lands outside the two
//! directories.

use std::path::Path;

use specforge_common::facts::{FACTS_HISTORY_DIR, filed, is_collector_name, is_store_key};
use specforge_registry::Disk;

use super::{Store, superseded};
use crate::harness::Line;

/// The digits of an archive's sequence number.
const SEQ_DIGITS: usize = 6;
/// The largest sequence number six digits write.
const MAX_SEQ: u32 = 999_999;

/// One archived store of a collector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archived {
    /// Its place in the archive: the larger, the later it was superseded.
    pub seq: u32,
    /// The key it was filed under while it was live.
    pub key: String,
    /// The store file, from the project root.
    pub path: String,
}

/// The sequence number and the key an archive directory `name` is named by: `<seq>-<key>`, `<seq>` six ASCII digits
/// and `<key>` a store key. `None` for any other name.
pub fn parse_name(name: &str) -> Option<(u32, &str)> {
    let (seq, key) = name.split_once('-')?;
    if seq.len() != SEQ_DIGITS || !seq.bytes().all(|b| b.is_ascii_digit()) || !is_store_key(key) {
        return None;
    }
    Some((seq.parse().ok()?, key))
}

/// The archive directory of the store superseded at `seq` that was filed under `key`.
fn dir_name(seq: u32, key: &str) -> String {
    format!("{seq:0SEQ_DIGITS$}-{key}")
}

/// The store file of `collector` in the archive directory of `seq` and `key`, from the project root.
pub fn path(collector: &str, seq: u32, key: &str) -> String {
    format!(
        "{FACTS_HISTORY_DIR}/{}/{collector}.json",
        dir_name(seq, key)
    )
}

/// Every archive directory under the project root `root`, by sequence number and key, listed through `disk`: a
/// directory whose name is no archive's is skipped.
fn directories(disk: &dyn Disk, root: &Path) -> Vec<(u32, String)> {
    disk.list(&root.join(FACTS_HISTORY_DIR))
        .unwrap_or_default()
        .iter()
        .filter_map(|name| parse_name(name).map(|(seq, key)| (seq, key.to_string())))
        .collect()
}

/// The archived stores of `collector` under the project root `root`, listed through `disk`, newest first: each
/// archive directory holding `<collector>.json`. A directory that holds none (one an interrupted change left empty)
/// is skipped.
pub fn archived(disk: &dyn Disk, root: &Path, collector: &str) -> Vec<Archived> {
    if !is_collector_name(collector) {
        return Vec::new();
    }
    let file = format!("{collector}.json");
    let history = root.join(FACTS_HISTORY_DIR);
    let mut found: Vec<Archived> = directories(disk, root)
        .into_iter()
        .filter(|(seq, key)| {
            disk.list(&history.join(dir_name(*seq, key)))
                .is_some_and(|names| names.contains(&file))
        })
        .map(|(seq, key)| Archived {
            path: path(collector, seq, &key),
            seq,
            key,
        })
        .collect();
    found.sort_by(|a, b| b.seq.cmp(&a.seq).then_with(|| a.key.cmp(&b.key)));
    found
}

/// What a collect that files `collector`'s facts under `key` does to the stores it supersedes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Retirement {
    /// Each superseded store, from its live path to its archive path, both from the project root.
    pub moves: Vec<(String, String)>,
    /// Each store removed, from the project root: an archived store beyond the bound, oldest first, and a superseded
    /// store the bound leaves no room for.
    pub pruned: Vec<String>,
}

/// What a collect filing `collector`'s facts under `key` does to its stores under the project root `root`, listed
/// through `disk`, keeping `retain` archived (ADR 0376 D4): each live store under another key is archived under a
/// fresh sequence number, one more than the largest of the archive, in key order; then the collector's archives
/// beyond the newest `retain`, the new ones counted, are pruned, oldest first. A collector or key that names no
/// store retires nothing.
pub fn retire(disk: &dyn Disk, root: &Path, collector: &str, key: &str, retain: u8) -> Retirement {
    let mut retirement = Retirement::default();
    if !is_collector_name(collector) || !is_store_key(key) {
        return retirement;
    }
    let next = directories(disk, root)
        .iter()
        .map(|(seq, _)| *seq)
        .max()
        .unwrap_or(0);
    // The newest first, as `archived` lists them: the new ones, the latest seq first, then those already there.
    let mut kept: Vec<Archived> = Vec::new();
    for (offset, store) in superseded(disk, root, collector, key).iter().enumerate() {
        let Some((_, filed_under)) = filed(store) else {
            continue;
        };
        let seq = next + 1 + offset as u32;
        if seq > MAX_SEQ {
            // shortcut: an archive past a million supersedes archives nothing more; renumbering it is a migration.
            retirement.pruned.push(store.clone());
            continue;
        }
        let to = path(collector, seq, filed_under);
        retirement.moves.push((store.clone(), to.clone()));
        kept.push(Archived {
            seq,
            key: filed_under.to_string(),
            path: to,
        });
    }
    kept.reverse();
    kept.extend(archived(disk, root, collector));
    let bound = usize::from(retain.max(1));
    for beyond in kept.iter().skip(bound).rev() {
        // A superseded store the bound leaves no room for is removed where it is, never archived first.
        match retirement
            .moves
            .iter()
            .position(|(_, to)| *to == beyond.path)
        {
            Some(at) => {
                let (from, _) = retirement.moves.remove(at);
                retirement.pruned.push(from);
            }
            None => retirement.pruned.push(beyond.path.clone()),
        }
    }
    retirement
}

/// Where `collector`'s store filed under `key` may be under the project root `root`, listed through `disk`: its live
/// path, then each archived one under that key, newest first. Empty for a name that is no store's.
pub fn stores(disk: &dyn Disk, root: &Path, collector: &str, key: &str) -> Vec<String> {
    if !is_collector_name(collector) || !is_store_key(key) {
        return Vec::new();
    }
    let archives = archived(disk, root, collector)
        .into_iter()
        .filter(|a| a.key == key)
        .map(|a| a.path);
    std::iter::once(Store::path(collector, key))
        .chain(archives)
        .collect()
}

/// The bytes of `collector`'s store filed under `key` that a `facts:` ledger line of `digest` records (the digest of
/// its bytes, [`Line::facts`]), read through `disk` under the project root `root`: the first of [`stores`] that holds
/// them. `None` when none of them is there with those bytes.
pub fn find(
    disk: &dyn Disk,
    root: &Path,
    collector: &str,
    key: &str,
    digest: &str,
) -> Option<Vec<u8>> {
    stores(disk, root, collector, key)
        .into_iter()
        .filter_map(|path| disk.read(&root.join(path)).ok())
        .find(|bytes| Line::facts(collector, key, bytes).digest == digest)
}
