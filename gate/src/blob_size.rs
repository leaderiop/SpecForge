//! The `blob-size` check: every committed `.wasm` blob is at most [`CAP`] bytes (ADR 0338 D8). Every user carries each
//! builtin in the binary, so a dependency a family drags in is a cost to all of them.
//!
//! Pure but for [`measure`]; tested on literal input.

use std::path::Path;

/// The check's name.
pub const CHECK: &str = "blob-size";

/// The most bytes one committed blob may hold.
pub const CAP: u64 = 2_500_000;

/// The size of each of `paths` (repo-relative) under `root`. A path that cannot be read (in the index, deleted from
/// the working tree) is left out.
pub fn measure<'a>(root: &Path, paths: impl IntoIterator<Item = &'a str>) -> Vec<(String, u64)> {
    paths
        .into_iter()
        .filter_map(|path| {
            let size = std::fs::metadata(root.join(path)).ok()?.len();
            Some((path.to_string(), size))
        })
        .collect()
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
