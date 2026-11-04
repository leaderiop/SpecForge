//! What a checker's local probes read: the check's disk, and the Git port. Each is local and needs no journal: a missing
//! file is checked again by its stamp, not by a time to live (O12).

use std::collections::VecDeque;

use specforge_common::path_glob;
use specforge_effects::contained::{Contained, Resolved};
use specforge_effects::git::Located;
use specforge_effects::{FsOp, OutcomeClass};
use specforge_protocol_types::{OutcomeData, Probe, StatEntry};
use specforge_registry::Disk;

use crate::Environment;

/// The most entries `fs.walk` answers, and the most directories it opens: a loop of links ends here.
pub(super) const WALK_CAP: usize = 10_000;

pub(super) fn read_disk(
    disk: &dyn Disk,
    contained: &Contained<'_>,
    resolved: &Resolved,
    op: &FsOp,
) -> (OutcomeClass, OutcomeData) {
    // Asked as written too: a session records the path itself, so a file that appears re-runs the check (O12).
    let there = disk.exists(&resolved.at);
    if !resolved.exists || !there {
        let near = contained.sibling(resolved);
        return (OutcomeClass::Missing, stat_of(None, 0, near));
    }
    match op {
        FsOp::Stat if disk.is_dir(&resolved.at) => {
            (OutcomeClass::Ok, stat_of(Some(StatEntry::Dir), 0, None))
        }
        FsOp::Stat => match disk.stamp(&resolved.at) {
            Ok(stamp) => (
                OutcomeClass::Ok,
                stat_of(Some(StatEntry::File), stamp.size(), None),
            ),
            Err(_) => (OutcomeClass::Denied, OutcomeData::None),
        },
        FsOp::Read { max_bytes } => match disk.read(&resolved.at) {
            Ok(mut bytes) => {
                let length = bytes.len() as u64;
                let truncated = length > *max_bytes;
                bytes.truncate(*max_bytes as usize);
                (
                    OutcomeClass::Ok,
                    OutcomeData::Bytes {
                        bytes,
                        length,
                        truncated,
                    },
                )
            }
            Err(_) => (OutcomeClass::Denied, OutcomeData::None),
        },
        FsOp::Walk { glob, .. } => {
            let (entries, truncated) =
                walk(disk, contained, resolved, glob.as_deref().unwrap_or("**"));
            (OutcomeClass::Ok, OutcomeData::Walk { entries, truncated })
        }
    }
}

/// The files under `resolved` whose path from it matches `glob`, each held by the root; sorted, at most
/// [`WALK_CAP`].
fn walk(
    disk: &dyn Disk,
    contained: &Contained<'_>,
    resolved: &Resolved,
    glob: &str,
) -> (Vec<String>, bool) {
    let mut entries = Vec::new();
    let mut truncated = false;
    let mut opened = 0;
    let mut pending = VecDeque::from([String::new()]);
    while let Some(under) = pending.pop_front() {
        let dir = resolved.at.join(&under);
        let Some(names) = disk.list(&dir) else {
            continue;
        };
        opened += 1;
        if opened > WALK_CAP {
            truncated = true;
            break;
        }
        for name in names {
            let relative = if under.is_empty() {
                name
            } else {
                format!("{under}/{name}")
            };
            let written = format!("{}/{relative}", resolved.rel);
            let Ok(entry) = contained.resolve(&written, resolved.root.clone()) else {
                continue;
            };
            if entry.withheld {
                continue;
            }
            if disk.is_dir(&entry.at) {
                pending.push_back(relative);
            } else if path_glob::matches(glob, &relative) {
                if entries.len() == WALK_CAP {
                    truncated = true;
                } else {
                    entries.push(relative);
                }
            }
        }
    }
    entries.sort();
    (entries, truncated)
}

/// What git answers; none when it cannot be asked.
pub(super) fn ask_git(
    env: &Environment,
    probe: &Probe,
    tracked: Option<&Resolved>,
) -> Option<(OutcomeClass, OutcomeData)> {
    let port = env.git()?;
    let place = match port.locate(env.root()) {
        Located::Repo(place) => place,
        Located::NotARepository => return Some((OutcomeClass::Missing, git_of(false, None))),
        Located::Unavailable(_) => return None,
    };
    let revision = match probe {
        Probe::GitObject { rev } => rev.as_str(),
        Probe::GitRef { name } => name.as_str(),
        _ => "HEAD",
    };
    // A value is never an option of the program behind the port.
    if revision.starts_with('-') {
        return Some((OutcomeClass::Missing, git_of(false, None)));
    }
    let commit = port.commit(&place, revision).ok()?;
    let Some(commit) = commit else {
        return Some((OutcomeClass::Missing, git_of(false, None)));
    };
    let Some(resolved) = tracked else {
        let oid = Some(commit.as_str().to_string());
        return Some((OutcomeClass::Ok, git_of(true, oid)));
    };
    let at = resolved.real.as_ref().unwrap_or(&resolved.at);
    let Ok(from_top) = at.strip_prefix(&place.toplevel) else {
        return Some((OutcomeClass::Missing, git_of(false, None)));
    };
    let sizes = port
        .sizes(&place, &commit, &[from_top.to_path_buf()])
        .ok()?;
    let found = sizes.first().is_some_and(|(_, size)| size.is_some());
    let class = if found {
        OutcomeClass::Ok
    } else {
        OutcomeClass::Missing
    };
    Some((class, git_of(found, None)))
}

pub(super) fn stat_of(entry: Option<StatEntry>, size: u64, near: Option<String>) -> OutcomeData {
    OutcomeData::Stat { entry, size, near }
}

pub(super) fn git_of(found: bool, oid: Option<String>) -> OutcomeData {
    OutcomeData::Git { found, oid }
}
