use crate::lock_file::LockFile;
use specforge_common::{Diagnostic, codes};
use std::path::Path;

/// Uninstall `name`: remove it from `lock`, then delete its directory under
/// `extensions_dir` (the lock entry is restored when that fails). Whether
/// other extensions still need it is the caller's to decide first.
pub fn uninstall_extension(
    name: &str,
    extensions_dir: &Path,
    lock: &mut LockFile,
) -> Result<(), Diagnostic> {
    // 1. Keep the entry, to restore it if the directory cannot be removed.
    let entry = lock.entries.iter().find(|e| e.name == name).cloned();

    // 2. Remove from lock file
    lock.entries.retain(|e| e.name != name);

    // 3. Delete .wasm binary directory
    let ext_dir = extensions_dir.join(name);
    if ext_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&ext_dir)
    {
        // Rollback: restore lock entry
        if let Some(entry) = entry {
            lock.entries.push(entry);
        }
        return Err(Diagnostic::new(
            codes::E032,
            format!(
                "failed to remove extension directory '{}': {}",
                ext_dir.display(),
                e
            ),
        )
        .with_suggestion(format!("manually remove '{}'", ext_dir.display())));
    }

    Ok(())
}
