//! The syntax cache (ADR 0344 D2): a file's parsed outcome keyed by the grammar, the query's digest and the file's
//! sha256, under `.specforge/cache/syntax/`. A hit touches its entry; an entry untouched past an age is pruned. A
//! miss, an unreadable entry and a write that fails cost a parse, never an error.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use specforge_protocol_types::SyntaxMatch;

/// Where the cache lives under a project's state base.
pub const CACHE_DIR: &str = ".specforge/cache/syntax";
/// How long an entry nobody touched is kept.
pub const CACHE_MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// The entry format, part of every key: a format change misses every old entry.
const CACHE_FORMAT: &str = "specforge-syntax-cache-1";

/// What an entry holds: a parsed outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) matches: Vec<SyntaxMatch>,
    pub(crate) errors: u32,
}

/// The syntax cache of one project, or none.
#[derive(Debug, Clone)]
pub struct SyntaxCache {
    dir: Option<PathBuf>,
}

impl SyntaxCache {
    /// The cache under `base` (a project's state base): `<base>/.specforge/cache/syntax`.
    pub fn at(base: &Path) -> Self {
        SyntaxCache {
            dir: Some(base.join(CACHE_DIR)),
        }
    }

    /// No cache: every read parses.
    pub fn detached() -> Self {
        SyntaxCache { dir: None }
    }

    /// Its directory, when it has one.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// The key of a file of `sha256` parsed with `grammar` and the query of `query_digest`.
    pub(crate) fn key(grammar: &str, query_digest: &str, sha256: &str) -> String {
        let mut hasher = sha2::Sha256::new();
        for part in [CACHE_FORMAT, grammar, query_digest, sha256] {
            hasher.update(part.as_bytes());
            hasher.update([0]);
        }
        format!("{:x}", hasher.finalize())
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        let dir = self.dir.as_ref()?;
        Some(dir.join(&key[..2]).join(format!("{key}.json")))
    }

    /// The entry under `key`, touched; none when there is none or it does not read.
    pub(crate) fn get(&self, key: &str) -> Option<Entry> {
        let path = self.path(key)?;
        let entry = serde_json::from_slice(&std::fs::read(&path).ok()?).ok()?;
        // A touch that fails only shortens the entry's life.
        if let Ok(file) = std::fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(entry)
    }

    /// Keeps `entry` under `key`: whether it was written (a detached cache writes nothing and says so).
    pub(crate) fn put(&self, key: &str, entry: &Entry) -> bool {
        let Some(path) = self.path(key) else {
            return true;
        };
        let Ok(bytes) = serde_json::to_vec(entry) else {
            return false;
        };
        path.parent()
            .is_some_and(|dir| std::fs::create_dir_all(dir).is_ok())
            && specforge_common::fresh::replace_file(&path, &bytes).is_ok()
    }

    /// Removes the entries untouched for `max_age` before `now`: how many it removed.
    pub fn prune(&self, now: SystemTime, max_age: Duration) -> usize {
        let Some(dir) = &self.dir else {
            return 0;
        };
        let Some(oldest) = now.checked_sub(max_age) else {
            return 0;
        };
        let mut removed = 0;
        for shard in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            for entry in std::fs::read_dir(shard.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let stale = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .is_ok_and(|modified| modified < oldest);
                if stale && entry.file_type().is_ok_and(|kind| kind.is_file()) {
                    removed += usize::from(std::fs::remove_file(entry.path()).is_ok());
                }
            }
        }
        removed
    }
}
