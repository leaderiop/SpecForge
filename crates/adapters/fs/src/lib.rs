//! The adapter behind the effect plane's `Fs` port (ADR 0335): `std::fs` behind [`Contained`] over the real file
//! system. A path is resolved in its root first (`outside`, `missing`); a read answers at most its `max_bytes` (held
//! to 1 MiB) of a file no larger than 64 MiB, never a withheld one or one with more than one name; a walk visits at most 20 000 entries, skips
//! tool and dependency directories the glob does not name, never enters the host's own state, and does not descend
//! through links.
//!
//! [`Contained`]: specforge_effects::Contained

mod change;

use std::io::Read as _;
use std::path::Path;
use std::time::{Duration, Instant, UNIX_EPOCH};

use specforge_effects::contained::in_host_state;
use specforge_effects::fs::{
    FILE_CEILING, FileKind, FsStat, LIST_VISITS, READ_CEILING, SKIPPED, WALK_VISITS, held,
};
use specforge_effects::{
    Fs, FsChange, FsChanged, FsDigest, FsListing, FsOp, FsOutcome, FsRequest, ListRequest, Listed,
    OutcomeClass, Place, RealFs, Roots,
};

/// The most time a digest of one file takes; past it the answer is `Denied`, as for a file that cannot be read.
const DIGEST_TIME: Duration = Duration::from_secs(60);

/// The `Fs` port over the local file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

impl Fs for LocalFs {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
        Self::answer(roots, request, READ_CEILING)
    }

    fn read_whole(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
        Self::answer(roots, request, FILE_CEILING)
    }

    fn list(&self, roots: &Roots, request: &ListRequest) -> FsListing {
        Self::listing(roots, request)
    }

    fn change(&self, roots: &Roots, change: &FsChange) -> FsChanged {
        change::change(roots, change)
    }

    fn digest(&self, roots: &Roots, at: &Place) -> FsDigest {
        change::digest(roots, at)
    }
}

impl LocalFs {
    /// [`Fs::run`], a read held to `ceiling`.
    fn answer(roots: &Roots, request: &FsRequest, ceiling: u64) -> FsOutcome {
        let held = match held(roots, &RealFs, request) {
            Ok(held) => held,
            Err(class) => return FsOutcome::of(class),
        };
        let root = roots.get(&held.root).unwrap_or(&held.at).to_path_buf();
        let at = held.at.clone();
        let stat = match std::fs::metadata(&at) {
            Ok(meta) => stat_of(&meta),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return FsOutcome::of(OutcomeClass::Missing);
            }
            Err(_) => return FsOutcome::of(OutcomeClass::Denied),
        };
        let mut outcome = FsOutcome::of(OutcomeClass::Ok);
        outcome.stat = Some(stat);
        match &request.op {
            FsOp::Stat => {}
            // The written name or where it lands: `notes.txt -> .env` is not read.
            FsOp::Read { .. } | FsOp::Digest if held.withheld => {
                return FsOutcome::of(OutcomeClass::Denied);
            }
            FsOp::Read { .. } | FsOp::Digest if stat.kind != FileKind::File => {}
            FsOp::Read { .. } | FsOp::Digest if stat.size > FILE_CEILING => {
                outcome.class = OutcomeClass::Capped;
            }
            FsOp::Digest => {
                match change::digest_of_one_name(&at, Some(Instant::now() + DIGEST_TIME)) {
                    Ok((sha256, _)) => outcome.digest = Some(sha256),
                    Err(_) => return FsOutcome::of(OutcomeClass::Denied),
                }
            }
            FsOp::Read { max_bytes } => {
                let max = (*max_bytes).min(ceiling);
                // The handle checked is the handle read (ADR 0146 D2).
                let read = std::fs::File::open(&at).and_then(|file| {
                    specforge_common::regular::refuse_if_linked(&file)?;
                    file.take(max + 1).read_to_end(&mut outcome.bytes)
                });
                if read.is_err() {
                    return FsOutcome::of(OutcomeClass::Denied);
                }
                outcome.truncated = outcome.bytes.len() as u64 > max;
                outcome.bytes.truncate(max as usize);
            }
            FsOp::Walk { glob, max_entries } => {
                let mut walk = Walk {
                    root: &root,
                    glob: glob.as_deref(),
                    visits: 0,
                    entries: Vec::new(),
                };
                walk.dir(&at);
                let max = *max_entries as usize;
                outcome.truncated = walk.visits > WALK_VISITS || walk.entries.len() > max;
                walk.entries.sort();
                walk.entries.truncate(max);
                outcome.entries = walk.entries;
            }
        }
        outcome
    }

    /// [`Fs::list`]: the directory resolved like a stat, then walked with the `ignore` crate's rules, never through a
    /// link, never into the host's own state or `.git`, at most [`LIST_VISITS`] entries visited.
    fn listing(roots: &Roots, request: &ListRequest) -> FsListing {
        let stat = FsRequest {
            root: request.root.clone(),
            path: request.path.clone(),
            op: FsOp::Stat,
        };
        let held = match held(roots, &RealFs, &stat) {
            Ok(held) => held,
            Err(class) => return FsListing::of(class),
        };
        let start = held.at.clone();
        let Ok(wanted) = overrides(&start, &request.globs) else {
            return FsListing::of(OutcomeClass::Denied);
        };
        let globs = wanted.clone();
        let prefix = if held.rel.is_empty() {
            String::new()
        } else {
            format!("{}/", held.rel)
        };
        let mut walk = ignore::WalkBuilder::new(&start);
        walk.hidden(!request.hidden)
            .ignore(request.ignore)
            .git_ignore(request.ignore)
            .git_exclude(request.ignore)
            .git_global(request.ignore)
            .parents(request.ignore)
            .require_git(false)
            .follow_links(false)
            .max_depth(request.depth)
            // The globs judge what the walk reaches, never widen it past `hidden` and `ignore`: ripgrep's overrides
            // would whitelist a hidden or ignored name.
            .filter_entry(move |entry| {
                let name = entry.file_name().to_string_lossy();
                let dir = entry.file_type().is_some_and(|kind| kind.is_dir());
                entry.depth() == 0
                    || !(in_host_state(&name)
                        || name == ".git"
                        || globs.matched(entry.path(), dir).is_ignore())
            });
        let mut listing = FsListing::of(OutcomeClass::Ok);
        for (visits, entry) in walk.build().enumerate() {
            if visits >= LIST_VISITS {
                listing.truncated = true;
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    let at = failed_path(&error).unwrap_or(&start);
                    let below = at.strip_prefix(&start).unwrap_or(Path::new(""));
                    let below = below.to_string_lossy().replace('\\', "/");
                    listing.unread.push(if below.is_empty() {
                        held.rel.clone()
                    } else {
                        format!("{prefix}{below}")
                    });
                    continue;
                }
            };
            let Some(kind) = entry.file_type() else {
                continue;
            };
            // A link is neither entered nor listed.
            let kind = if kind.is_file() {
                FileKind::File
            } else if kind.is_dir() {
                FileKind::Dir
            } else {
                continue;
            };
            let Ok(below) = entry.path().strip_prefix(&start) else {
                continue;
            };
            if entry.depth() == 0 || request.kind.is_some_and(|only| only != kind) {
                continue;
            }
            // A directory no glob names is walked but, beside a whitelist glob, not listed.
            let globbed = match wanted.matched(entry.path(), kind == FileKind::Dir) {
                ignore::Match::Whitelist(_) => true,
                ignore::Match::Ignore(_) => false,
                ignore::Match::None => wanted.num_whitelists() == 0,
            };
            if !globbed {
                continue;
            }
            let below = below.to_string_lossy().replace('\\', "/");
            let size = match kind {
                FileKind::File => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
                _ => 0,
            };
            listing.entries.push(Listed {
                path: format!("{prefix}{below}"),
                kind,
                size,
            });
        }
        listing.entries.sort_by(|a, b| a.path.cmp(&b.path));
        if listing.entries.len() > request.max_entries {
            listing.truncated = true;
            listing.entries.truncate(request.max_entries);
        }
        listing
    }
}

/// The path a walk's error names, under the depth and line the walker wraps it in.
fn failed_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::Loop { child, .. } => Some(child),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            failed_path(err)
        }
        ignore::Error::Partial(errors) => errors.iter().find_map(failed_path),
        _ => None,
    }
}

/// The globs as the `ignore` crate's overrides at `start`: gitignore semantics, so one with a `/` matches the path
/// below `start` and one without a name at any depth, and `!glob` leaves a file or a directory out; `Err` when one
/// does not parse.
fn overrides(start: &Path, globs: &[String]) -> Result<ignore::overrides::Override, ignore::Error> {
    let mut built = ignore::overrides::OverrideBuilder::new(start);
    for glob in globs {
        built.add(glob)?;
    }
    built.build()
}

fn stat_of(meta: &std::fs::Metadata) -> FsStat {
    FsStat {
        kind: if meta.is_file() {
            FileKind::File
        } else if meta.is_dir() {
            FileKind::Dir
        } else {
            FileKind::Other
        },
        size: meta.len(),
        modified_ms: meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64),
    }
}

/// One walk under a root: the root-relative files it found, and how many entries it visited.
struct Walk<'a> {
    root: &'a Path,
    glob: Option<&'a str>,
    visits: usize,
    entries: Vec<String>,
}

impl Walk<'_> {
    fn dir(&mut self, dir: &Path) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in read.flatten() {
            self.visits += 1;
            if self.visits > WALK_VISITS {
                return;
            }
            // The entry's own type: a link is neither entered nor listed.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if in_host_state(&name) {
                continue;
            }
            if kind.is_dir() {
                if !SKIPPED.contains(&name.as_str()) || self.names(&name) {
                    self.dir(&path);
                }
            } else if kind.is_file()
                && let Ok(rel) = path.strip_prefix(self.root)
            {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if self
                    .glob
                    .is_none_or(|glob| specforge_common::path_glob::matches(glob, &rel))
                {
                    self.entries.push(rel);
                }
            }
        }
    }

    /// The glob names `dir` in one of its literal segments.
    fn names(&self, dir: &str) -> bool {
        self.glob
            .is_some_and(|glob| glob.split('/').any(|segment| segment == dir))
    }
}
