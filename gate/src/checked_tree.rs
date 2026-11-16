//! The tree the `specs` check runs `specforge check` on: the tracked files, as the working tree holds them, copied to a
//! scratch directory. What a machine keeps beside them, untracked or ignored (the fact stores and the effects journal
//! under `.specforge/`, the rename ledger `specforge-harness`, a test report), is not there, so the counts the
//! baseline judges are the tree's alone, as CI sees them.
//!
//! Each copy keeps its file's modification time and permissions. Cargo judges a crate fresh when no source file is
//! newer than its last build, and keys a path crate by its path from the workspace root: a copy of the workspace
//! built with the gate's own `CARGO_TARGET_DIR` then reuses every crate the gate built, never compiles one again.
//! A copy dated now would rebuild them all. A tracked script stays executable.

use std::io::Write;
use std::path::Path;

/// Copy each of `tracked` (paths from `root`) into `into`, emptied first. A tracked path the working tree no longer
/// holds as a file (deleted, not yet staged) is left out, as `check` would not read it either.
pub fn copy_tracked(root: &Path, tracked: &[String], into: &Path) -> Result<(), String> {
    if into.exists() {
        std::fs::remove_dir_all(into).map_err(|e| format!("{}: {e}", into.display()))?;
    }
    for path in tracked {
        let from = root.join(path);
        let (Ok(bytes), Ok(meta)) = (std::fs::read(&from), std::fs::metadata(&from)) else {
            continue;
        };
        let to = into.join(path);
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let at = |e: std::io::Error| format!("{}: {e}", to.display());
        let modified = meta.modified().map_err(at)?;
        // One handle writes the bytes and the time, then the mode: a read-only file is never reopened to write.
        let mut file = std::fs::File::create(&to).map_err(at)?;
        file.write_all(&bytes).map_err(at)?;
        file.set_modified(modified).map_err(at)?;
        drop(file);
        std::fs::set_permissions(&to, meta.permissions()).map_err(at)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
