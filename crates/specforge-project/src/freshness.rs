//! A session brings itself up to date with disk without a watcher
//! (behavior `bring_session_up_to_date`).
//!
//! Every build records what it read: each source's size and modification
//! time, the same for each environment input, and for each check input
//! before the checks run. The config is stamped before its one read, the
//! lock and the modules before the extension runtime and the environment
//! read them. [`ProjectSession::stale`] compares that record
//! with disk. Stamps are taken *before* the read they describe, so a write
//! that races the read is seen next time rather than lost.
//!
//! Racy-clean (git's rule): a file modified within the timestamp
//! granularity of when it was stamped could change again without its
//! stamp changing (same second, same length). Its content hash is recorded
//! too, and compared whenever its stamp is unchanged, so such a rewrite is
//! still seen, and an unchanged file is not reported.
//!
//! [`ProjectSession::stale`]: crate::ProjectSession::stale

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::inputs::{Changes, SessionInputs};

/// Coarse filesystems record modification times to the second (some to
/// two); an entry modified this close to when it was stamped is racy.
const GRANULARITY: Duration = Duration::from_secs(2);

/// What a path looked like when it was stamped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    len: u64,
    modified: Option<SystemTime>,
    /// The content's hash, for a racy entry only.
    racy_hash: Option<u64>,
}

impl Entry {
    /// Stamp `path` now: `None` when it does not exist.
    pub(crate) fn stamp(path: &Path) -> Option<Entry> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok();
        let now = SystemTime::now();
        // No modification time, or one within the granularity of now (a
        // time in the future included): the stamp alone cannot be trusted.
        let racy = modified.is_none_or(|m| m + GRANULARITY >= now);
        Some(Entry {
            len: metadata.len(),
            modified,
            racy_hash: if racy { Some(content_hash(path)) } else { None },
        })
    }

    /// Whether `path` still holds what this entry recorded.
    fn holds(&self, path: &Path) -> bool {
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };
        if metadata.len() != self.len || metadata.modified().ok() != self.modified {
            return false;
        }
        self.racy_hash.is_none_or(|hash| content_hash(path) == hash)
    }
}

/// Whether `path` is as `recorded` says: absent when it recorded nothing.
fn unchanged(path: &Path, recorded: Option<&Entry>) -> bool {
    match recorded {
        Some(entry) => entry.holds(path),
        None => !path.exists(),
    }
}

/// A file's bytes, or a directory's sorted entry names, hashed.
fn content_hash(path: &Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if path.is_dir() {
        let mut names: Vec<_> = std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok().map(|e| e.file_name()))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names.hash(&mut hasher);
    } else {
        std::fs::read(path).ok().hash(&mut hasher);
    }
    hasher.finish()
}

/// What a session last built from, as it was when read.
#[derive(Debug, Default, Clone)]
pub(crate) struct DiskSnapshot {
    /// Every source, by key relative to the spec root.
    sources: BTreeMap<String, Entry>,
    /// Every environment input (absent ones recorded as `None`).
    environment: BTreeMap<PathBuf, Option<Entry>>,
    /// Every check input, as the last run of the checks read it.
    checks: BTreeMap<PathBuf, Option<Entry>>,
}

impl DiskSnapshot {
    /// Stamp `specforge.json`, before its one read: a config rewritten after
    /// this stamp is seen next time, whatever the rest was read from. Starts
    /// the environment record again.
    pub(crate) fn stamp_config(&mut self, config: &Path) {
        self.environment = BTreeMap::new();
        self.environment
            .insert(config.to_path_buf(), Entry::stamp(config));
    }

    /// Stamp the other environment inputs (the lock and each extension
    /// module), before the extension runtime and the environment read
    /// them.
    pub(crate) fn stamp_environment(&mut self, inputs: &SessionInputs) {
        for path in inputs.environment_files() {
            let entry = Entry::stamp(path);
            self.environment.insert(path.to_path_buf(), entry);
        }
    }

    /// Stamp every source `discovered` under the spec root, before the
    /// sources are read.
    pub(crate) fn stamp_all_sources(&mut self, inputs: &SessionInputs, discovered: &[PathBuf]) {
        let Some(spec_root) = inputs.spec_root() else {
            return;
        };
        self.sources = discovered
            .iter()
            .filter_map(|path| {
                let entry = Entry::stamp(path)?;
                Some((key(spec_root, path), entry))
            })
            .collect();
    }

    /// Stamp the sources `keys` names, before they are read again: the
    /// others keep what they recorded, so a change to one of them that
    /// this rebuild does not read is still seen next time.
    pub(crate) fn stamp_sources(&mut self, inputs: &SessionInputs, keys: &[String]) {
        let Some(spec_root) = inputs.spec_root() else {
            return;
        };
        for key in keys {
            match Entry::stamp(&spec_root.join(key)) {
                Some(entry) => self.sources.insert(key.clone(), entry),
                None => self.sources.remove(key),
            };
        }
    }

    /// Stamp every check input, before the checks read them.
    pub(crate) fn stamp_checks(&mut self, inputs: &SessionInputs) {
        self.checks = inputs
            .check_files()
            .map(|path| (path.to_path_buf(), Entry::stamp(path)))
            .collect();
    }

    /// What changed on disk since: the sources `inputs` discovers now
    /// against those recorded, and every recorded input.
    pub(crate) fn changes(&self, inputs: &SessionInputs) -> Changes {
        let Some(spec_root) = inputs.spec_root() else {
            return Changes::default();
        };
        let discovered = inputs.discover();
        let mut changes = Changes::default();
        let mut seen = std::collections::BTreeSet::new();
        for path in &discovered {
            let key = key(spec_root, path);
            if !unchanged(path, self.sources.get(&key)) {
                changes.sources.push(key.clone());
            }
            seen.insert(key);
        }
        // Recorded, and no longer found: deleted (or excluded since).
        changes.sources.extend(
            self.sources
                .keys()
                .filter(|key| !seen.contains(*key))
                .cloned(),
        );
        changes.sources.sort();
        changes.sources.dedup();
        changes.environment = self
            .environment
            .iter()
            .any(|(path, entry)| !unchanged(path, entry.as_ref()));
        changes.check_inputs = self
            .checks
            .iter()
            .any(|(path, entry)| !unchanged(path, entry.as_ref()));
        changes
    }
}

/// A discovered file's key, as the resolver names it.
fn key(spec_root: &Path, path: &Path) -> String {
    path.strip_prefix(spec_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
