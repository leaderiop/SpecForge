//! The `blob-size` check: every committed `.wasm` blob is at most [`CAP`] bytes (ADR 0338 D8), or the allowance
//! [`ALLOWANCES`] names for it (ADR 0366 D3): one blob, the vendored `@gsd/core`. Every user carries each builtin in
//! the binary, so a dependency a family drags in is a cost to all of them.
//!
//! Pure but for [`measure`], which reads git's index or the disk; tested on literal input and on a repository.

use std::path::Path;
use std::process::{Command, Stdio};

/// The check's name.
pub const CHECK: &str = "blob-size";

/// The cap as a literal, so the check's `proves` line is built from the same number.
macro_rules! cap {
    () => {
        2_500_000
    };
}
pub(crate) use cap;

/// The most bytes one committed blob may hold, unless [`ALLOWANCES`] names it.
pub const CAP: u64 = cap!();

/// The blobs held to an allowance of their own instead of [`CAP`], by repository path (ADR 0366 D3). The rule: the
/// blob's size at the commit that vendors it, rounded up to the next 100,000 B; a change that grows it past that
/// re-measures and sets the allowance in the same commit (ADR 0373 D3). `@gsd/core` measured 5,470,859 B.
pub const ALLOWANCES: &[(&str, u64)] = &[("gsd/core/wasm/gsd_core.wasm", 5_500_000)];

/// The most bytes the committed blob at `path` may hold: its allowance, else [`CAP`].
pub fn cap_of(path: &str) -> u64 {
    ALLOWANCES
        .iter()
        .find(|(allowed, _)| *allowed == path)
        .map_or(CAP, |&(_, allowance)| allowance)
}

/// Which copy of a blob is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The index's, at the pre-commit hook: what the commit will hold, whatever the working tree has since.
    Staged,
    /// The working tree's.
    Working,
}

/// The size of each `.wasm` among `paths` (repo-relative) under `root`, as `source` holds it. A path that cannot be read
/// (deleted from the working tree, or no longer in the index) is left out.
pub fn measure(root: &Path, paths: &[String], source: Source) -> Vec<(String, u64)> {
    paths
        .iter()
        .filter(|path| path.ends_with(".wasm"))
        .filter_map(|path| {
            let size = match source {
                Source::Working => std::fs::metadata(root.join(path)).ok()?.len(),
                Source::Staged => staged_size(root, path)?,
            };
            Some((path.clone(), size))
        })
        .collect()
}

/// `git cat-file -s :<path>`: the size of the blob the index holds at `path`.
fn staged_size(root: &Path, path: &str) -> Option<u64> {
    let out = Command::new("git")
        .args(["cat-file", "-s", &format!(":{path}")])
        .current_dir(root)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// One line per blob over its cap (its allowance, for a blob [`ALLOWANCES`] names); the check passes when there is
/// none.
pub fn verdict(found: &[(String, u64)]) -> Vec<String> {
    found
        .iter()
        .filter(|(path, size)| *size > cap_of(path))
        .map(|(path, size)| {
            let cap = cap_of(path);
            if cap == CAP {
                format!("{path} is {size} B, over the {CAP} B cap")
            } else {
                format!("{path} is {size} B, over its {cap} B allowance (ADR 0366 D3)")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
