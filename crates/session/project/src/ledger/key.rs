//! How a source is keyed: by [`SourceRoot::key`], its one mint. What stays here are the canonical forms of the paths
//! the ledger watches and follows, and the keys of a walk.

use std::path::{Path, PathBuf};

use specforge_common::{SourceRoot, Walked};

use crate::Environment;

impl Environment {
    /// A `.spec` path's key ([`SourceRoot::key`]): what the session holds, the parser is handed and a span says.
    /// With no project, the path itself.
    pub fn source_key(&self, path: &Path) -> String {
        self.sources.key(path).into_string()
    }
}

/// The keys of what `walked` (a walk of `sources`) holds for a session to read:
/// its files and its dangling links, sorted (ADR 0141).
pub(crate) fn discovered_keys(sources: &SourceRoot, walked: &Walked) -> Vec<String> {
    // Sorted as paths, as the walk sorts its files.
    let mut paths: Vec<&PathBuf> = walked.files.iter().chain(&walked.dangling).collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| sources.key(path).into_string())
        .collect()
}

/// The key of `canonical_path` under `canonical_root`, both canonical (a link
/// target, or a path of a batch): `None` when it lies outside.
pub(crate) fn relative_canonical(canonical_root: &Path, canonical_path: &Path) -> Option<String> {
    canonical_path
        .strip_prefix(canonical_root)
        .ok()
        .map(|relative| relative.to_string_lossy().into_owned())
}

/// `path` with its directory canonical and its own name kept: a symlink stays
/// itself. `path` as given when it has no directory or no name.
pub(crate) fn in_canonical_dir(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            canonical(parent).join(name)
        }
        _ => path.to_path_buf(),
    }
}

/// `path`, canonical when it can be: the path itself when it exists, else
/// its directory's canonical form joined with its name (a deleted file),
/// else as given.
pub(crate) fn canonical(path: &Path) -> PathBuf {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            canonical(parent).join(name)
        }
        _ => path.to_path_buf(),
    }
}
