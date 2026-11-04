//! What a checker's local probes read: the check's disk, and the Git port. Each is local and needs no journal: a missing
//! file is checked again by its stamp, not by a time to live.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead as _, BufReader};
use std::path::{Component, Path};
use std::time::Instant;

use specforge_common::path_glob;
use specforge_effects::contained::{Contained, Resolved, withheld};
use specforge_effects::git::{GitPlace, Located};
use specforge_effects::{FsOp, OutcomeClass};
use specforge_protocol_types::{OutcomeData, StatEntry};
use specforge_registry::Disk;

use crate::Environment;

/// The most entries `fs.walk` answers, and the most directories it opens: a loop of links ends here.
pub(super) const WALK_CAP: usize = 10_000;

/// A missing path's answer, with its contained nearest sibling; none when it is there.
fn missing(
    disk: &dyn Disk,
    contained: &Contained<'_>,
    resolved: &Resolved,
) -> Option<(OutcomeClass, OutcomeData)> {
    // Asked as written too: a session records the path itself, so a file that appears re-runs the check.
    let there = disk.exists(&resolved.at);
    (!resolved.exists || !there).then(|| {
        let near = contained.sibling(resolved);
        (OutcomeClass::Missing, stat_of(None, 0, near))
    })
}

/// The digest of a whole file: `Deadline` when the run's time ran out first, `Denied` for what is no regular file.
pub(super) fn digest_disk(
    disk: &dyn Disk,
    contained: &Contained<'_>,
    resolved: &Resolved,
    deadline: Instant,
) -> (OutcomeClass, OutcomeData) {
    if let Some(missing) = missing(disk, contained, resolved) {
        return missing;
    }
    match disk.digest(&resolved.at, deadline) {
        Ok((sha256, length)) => (OutcomeClass::Ok, OutcomeData::Digest { sha256, length }),
        Err(error) if error.kind() == io::ErrorKind::TimedOut => {
            (OutcomeClass::Deadline, OutcomeData::None)
        }
        Err(_) => (OutcomeClass::Denied, OutcomeData::None),
    }
}

pub(super) fn read_disk(
    disk: &dyn Disk,
    contained: &Contained<'_>,
    resolved: &Resolved,
    op: &FsOp,
) -> (OutcomeClass, OutcomeData) {
    if let Some(missing) = missing(disk, contained, resolved) {
        return missing;
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
        FsOp::Read { max_bytes } => match disk.read_prefix(&resolved.at, *max_bytes) {
            Ok((bytes, length)) => (
                OutcomeClass::Ok,
                OutcomeData::Bytes {
                    bytes,
                    length,
                    truncated: length > *max_bytes,
                },
            ),
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
            let written = match resolved.rel.as_str() {
                "" => format!("<{}>/{relative}", resolved.root),
                rel => format!("<{}>/{rel}/{relative}", resolved.root),
            };
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

/// The path a git question asks about.
pub(super) enum GitPath<'p> {
    /// Contained under a root.
    Under(&'p Resolved),
    /// From the repository's top, as a value wrote it.
    FromTop(&'p str),
}

/// What git answers about `revision`, and `path` at it; none when it cannot be asked. A withheld path is `Denied`
/// unasked, as a read of it is: whether it was ever committed says it exists.
pub(super) fn ask_git(
    env: &Environment,
    revision: &str,
    path: Option<GitPath<'_>>,
) -> Option<(OutcomeClass, OutcomeData)> {
    let denied = match &path {
        Some(GitPath::Under(resolved)) => resolved.withheld,
        Some(GitPath::FromTop(path)) => withheld(path),
        None => false,
    };
    if denied {
        return Some((OutcomeClass::Denied, OutcomeData::None));
    }
    let port = env.git()?;
    // A directory that is no repository (a source tarball) cannot say what one lacks.
    let Located::Repo(place) = port.locate(env.root()) else {
        return None;
    };
    // A value is never an option of the program behind the port.
    if revision.starts_with('-') {
        return Some((OutcomeClass::Missing, git_of(false, None)));
    }
    let Some(commit) = port.commit(&place, revision).ok()? else {
        return decides_a_miss(&place, revision)
            .then(|| (OutcomeClass::Missing, git_of(false, None)));
    };
    let from_top = match path {
        None => {
            let oid = Some(commit.as_str().to_string());
            return Some((OutcomeClass::Ok, git_of(true, oid)));
        }
        Some(GitPath::Under(resolved)) => {
            let at = resolved.real.as_ref().unwrap_or(&resolved.at);
            at.strip_prefix(&place.toplevel).ok().map(Path::to_path_buf)
        }
        // Only plain parts: a value never names a path above the top, or the top itself.
        Some(GitPath::FromTop(path)) => {
            let path = Path::new(path);
            let plain = path
                .components()
                .all(|part| matches!(part, Component::Normal(_)));
            (plain && !path.as_os_str().is_empty()).then(|| path.to_path_buf())
        }
    };
    let Some(from_top) = from_top else {
        return Some((OutcomeClass::Missing, git_of(false, None)));
    };
    let sizes = port.sizes(&place, &commit, &[from_top]).ok()?;
    let found = sizes.first().is_some_and(|(_, size)| size.is_some());
    let class = if found {
        OutcomeClass::Ok
    } else {
        OutcomeClass::Missing
    };
    Some((class, git_of(found, None)))
}

/// Whether `revision`, which the repository does not resolve, is not there: a shallow clone lacks older history and
/// the refs it did not fetch, and a clone that fetched no tag cannot say one is missing.
fn decides_a_miss(place: &GitPlace, revision: &str) -> bool {
    !place.shallow && (!revision.starts_with("refs/tags/") || holds_tags(place))
}

/// Whether the repository holds a tag, as git keeps refs on disk: loose under `refs/tags`, or in `packed-refs`. A store
/// read otherwise (reftable) holds none as far as this knows, so a tag it lacks is not decided.
fn holds_tags(place: &GitPlace) -> bool {
    let loose = std::fs::read_dir(place.common_dir.join("refs/tags"))
        .is_ok_and(|mut entries| entries.next().is_some());
    loose
        || File::open(place.common_dir.join("packed-refs")).is_ok_and(|packed| {
            BufReader::new(packed)
                .lines()
                .map_while(Result::ok)
                .any(|line| line.contains(" refs/tags/"))
        })
}

pub(super) fn stat_of(entry: Option<StatEntry>, size: u64, near: Option<String>) -> OutcomeData {
    OutcomeData::Stat { entry, size, near }
}

pub(super) fn git_of(found: bool, oid: Option<String>) -> OutcomeData {
    OutcomeData::Git { found, oid }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::Path;

    use specforge_effects::contained::RealFs;
    use specforge_effects::{Root, Roots};
    use specforge_registry::Disk;

    use super::*;

    /// A disk on which only a prefix can be read.
    struct OnlyPrefix;

    impl Disk for OnlyPrefix {
        fn exists(&self, _: &Path) -> bool {
            true
        }
        fn list(&self, _: &Path) -> Option<Vec<String>> {
            None
        }
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            panic!("read all of {}", path.display())
        }
        fn read_prefix(&self, _: &Path, max: u64) -> io::Result<(Vec<u8>, u64)> {
            assert_eq!(max, 2);
            Ok((b"he".to_vec(), 5))
        }
    }

    #[test]
    fn a_read_reads_only_what_it_answers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "hello").unwrap();
        let roots = Roots::new(dir.path(), dir.path());
        let contained = Contained::new(&roots, &RealFs);
        let resolved = contained.resolve("a.md", Root::Project).unwrap();
        let op = FsOp::Read { max_bytes: 2 };
        let (class, data) = read_disk(&OnlyPrefix, &contained, &resolved, &op);
        assert_eq!(class, OutcomeClass::Ok);
        assert_eq!(
            data,
            OutcomeData::Bytes {
                bytes: b"he".to_vec(),
                length: 5,
                truncated: true,
            }
        );
    }
}
