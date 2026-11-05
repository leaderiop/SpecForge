//! The `blob-size` check: every committed `.wasm` blob is at most [`CAP`] bytes (ADR 0338 D8). Every user carries each
//! builtin in the binary, so a dependency a family drags in is a cost to all of them.
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

/// The most bytes one committed blob may hold.
pub const CAP: u64 = cap!();

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

/// One line per blob over the cap; the check passes when there is none.
pub fn verdict(found: &[(String, u64)]) -> Vec<String> {
    found
        .iter()
        .filter(|(_, size)| *size > CAP)
        .map(|(path, size)| format!("{path} is {size} B, over the {CAP} B cap"))
        .collect()
}

#[cfg(test)]
mod tests;
