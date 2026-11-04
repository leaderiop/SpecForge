//! The adapter behind the effect plane's `Fs` port (ADR 0335): `std::fs` behind [`Contained`] over the real file
//! system. A path is resolved in its root first (`outside`, `missing`); a read answers at most its `max_bytes` (held
//! to 1 MiB) of a file no larger than 64 MiB, never a withheld one; a walk visits at most 20 000 entries, skips
//! tool and dependency directories the glob does not name, never enters the host's own state, and does not descend
//! through links.
//!
//! [`Contained`]: specforge_effects::Contained

use std::io::Read as _;
use std::path::Path;
use std::time::UNIX_EPOCH;

use specforge_effects::contained::in_host_state;
use specforge_effects::fs::{
    FILE_CEILING, FileKind, FsStat, READ_CEILING, SKIPPED, WALK_VISITS, held,
};
use specforge_effects::{Fs, FsOp, FsOutcome, FsRequest, OutcomeClass, RealFs, Roots};

/// The `Fs` port over the local file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

impl Fs for LocalFs {
    fn run(&self, roots: &Roots, request: &FsRequest) -> FsOutcome {
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
            FsOp::Read { .. } if held.withheld => return FsOutcome::of(OutcomeClass::Denied),
            FsOp::Read { .. } if stat.kind != FileKind::File => {}
            FsOp::Read { .. } if stat.size > FILE_CEILING => outcome.class = OutcomeClass::Capped,
            FsOp::Read { max_bytes } => {
                let max = (*max_bytes).min(READ_CEILING);
                let read = std::fs::File::open(&at)
                    .and_then(|file| file.take(max + 1).read_to_end(&mut outcome.bytes));
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
