//! The tree the `specs` check runs `specforge check` on: the tracked files, as the working tree holds them, copied to a
//! scratch directory. What a machine keeps beside them, untracked or ignored (the fact stores and the effects journal
//! under `.specforge/`, the rename ledger `specforge-harness`, a test report), is not there, so the counts the
//! baseline judges are the tree's alone, as CI sees them.

use std::path::Path;

/// Copy each of `tracked` (paths from `root`) into `into`, emptied first. A tracked path the working tree no longer
/// holds as a file (deleted, not yet staged) is left out, as `check` would not read it either.
pub fn copy_tracked(root: &Path, tracked: &[String], into: &Path) -> Result<(), String> {
    if into.exists() {
        std::fs::remove_dir_all(into).map_err(|e| format!("{}: {e}", into.display()))?;
    }
    for path in tracked {
        let Ok(bytes) = std::fs::read(root.join(path)) else {
            continue;
        };
        let to = into.join(path);
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
