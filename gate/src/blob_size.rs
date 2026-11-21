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
/// re-measures and sets the allowance in the same commit (ADR 0373 D3). `@gsd/core` measured 5,470,859 B when vendored
/// and 5,500,412 B at protocol 2.3.0, then 5,633,814 B once it placed, threaded and measured plans, 5,643,169 B
/// after the review of that plan flow, 5,643,233 B once gsd-strict preset the traceability, verdict and trailer
/// gates, 5,653,802 B once a gsd/E516 refusal named the prompts that remedy it and every phase holding the verdict,
/// the trailer reader took a task's full path and gsd/E521 suggested the trailer to add, 5,678,141 B once each
/// workflow declared the judges it runs again and the refusal listed what blocks every named run, 5,683,226 B once a
/// commit carried a person's waiver, `waive_trailer` recorded it at a terminal and one rule said what gsd/E521 judges,
/// 5,683,413 B once that rule also said why it does not judge a commit and the refusal named the reason, and
/// 5,704,569 B once a check since a revision said what changed under each stale truth (gsd/I527), which took the
/// allowance to 5,800,000 B, and 5,702,268 B once that advisory stopped claiming files it could not compare unchanged
/// and escaped every id and name it echoes, and 5,709,951 B once a gsd/E516 refusal named the entities owing each
/// verdict that blocks its runs, five per holder and judge, and a verdict no phase or milestone holds by its own
/// entity.
pub const ALLOWANCES: &[(&str, u64)] = &[("gsd/core/wasm/gsd_core.wasm", 5_800_000)];

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
