use crate::lock_file::LockFile;
use specforge_common::{Diagnostic, Severity};
use std::path::Path;

/// Result of an uninstall operation.
#[derive(Debug)]
pub struct UninstallResult {
    pub name: String,
    pub version: String,
}

/// Uninstall `name`: remove it from `lock`, then delete its directory under
/// `extensions_dir` (the lock entry is restored when that fails). Whether
/// other extensions still need it is the caller's to decide first.
pub fn uninstall_extension(
    name: &str,
    extensions_dir: &Path,
    lock: &mut LockFile,
) -> Result<UninstallResult, Diagnostic> {
    // 1. Find and save entry info before removal (for rollback)
    let entry = lock.entries.iter().find(|e| e.name == name).cloned();

    let version = entry
        .as_ref()
        .map(|e| e.version.clone())
        .unwrap_or_default();

    // 2. Remove from lock file
    let original_len = lock.entries.len();
    lock.entries.retain(|e| e.name != name);
    let _removed_from_lock = lock.entries.len() < original_len;

    // 3. Delete .wasm binary directory
    let ext_dir = extensions_dir.join(name);
    if ext_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&ext_dir)
    {
        // Rollback: restore lock entry
        if let Some(entry) = entry {
            lock.entries.push(entry);
        }
        return Err(Diagnostic {
            code: "E032".to_string(),
            severity: Severity::Error,
            message: format!(
                "failed to remove extension directory '{}': {}",
                ext_dir.display(),
                e
            ),
            span: None,
            suggestion: Some(format!("manually remove '{}'", ext_dir.display())),
            data: None,
        });
    }

    Ok(UninstallResult {
        name: name.to_string(),
        version,
    })
}
