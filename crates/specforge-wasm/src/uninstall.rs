use crate::cache::invalidate_entry;
use crate::lock_file::LockFile;
use specforge_common::{Diagnostic, Severity};
use specforge_registry::ManifestV2;
use std::path::Path;

/// Result of an uninstall operation.
#[derive(Debug)]
pub struct UninstallResult {
    pub name: String,
    pub version: String,
    pub cache_invalidated: bool,
}

/// Check if any installed extensions depend on the one being uninstalled.
pub fn check_dependents(name: &str, installed_manifests: &[ManifestV2]) -> Vec<String> {
    installed_manifests
        .iter()
        .filter(|m| {
            m.name != name
                && m.peer_dependencies
                    .iter()
                    .any(|dep| dep.name == name)
        })
        .map(|m| m.name.clone())
        .collect()
}

/// Uninstall: check dependents -> remove from lock -> delete .wasm -> invalidate AOT.
pub fn uninstall_extension(
    name: &str,
    installed_manifests: &[ManifestV2],
    extensions_dir: &Path,
    cache_dir: &Path,
    lock: &mut LockFile,
    force: bool,
) -> Result<UninstallResult, Diagnostic> {
    // 1. Check dependents
    let dependents = check_dependents(name, installed_manifests);
    if !dependents.is_empty() && !force {
        return Err(Diagnostic {
            code: "E027".to_string(),
            severity: Severity::Error,
            message: format!(
                "cannot uninstall '{}': required by {}",
                name,
                dependents.join(", ")
            ),
            span: None,
            suggestion: Some("use --force to uninstall anyway, or remove dependent extensions first".to_string()),
        });
    }

    // 2. Find and save entry info before removal (for rollback)
    let entry = lock
        .entries
        .iter()
        .find(|e| e.name == name)
        .cloned();

    let version = entry
        .as_ref()
        .map(|e| e.version.clone())
        .unwrap_or_default();

    let wasm_hash = entry
        .as_ref()
        .map(|e| e.wasm_hash.clone())
        .unwrap_or_default();

    // 3. Remove from lock file
    let original_len = lock.entries.len();
    lock.entries.retain(|e| e.name != name);
    let _removed_from_lock = lock.entries.len() < original_len;

    // 4. Delete .wasm binary directory
    let ext_dir = extensions_dir.join(name);
    if ext_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(&ext_dir)
    {
        // Rollback: restore lock entry
        if let Some(entry) = entry {
            lock.entries.push(entry);
        }
        return Err(Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!(
                "failed to remove extension directory '{}': {}",
                ext_dir.display(),
                e
            ),
            span: None,
            suggestion: Some(format!("manually remove '{}'", ext_dir.display())),
        });
    }

    // 5. Invalidate AOT cache
    let cache_invalidated = if !wasm_hash.is_empty() {
        invalidate_entry(cache_dir, &wasm_hash)
    } else {
        false
    };

    Ok(UninstallResult {
        name: name.to_string(),
        version,
        cache_invalidated,
    })
}
