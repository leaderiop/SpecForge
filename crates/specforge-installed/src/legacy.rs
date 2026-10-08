//! Today's separate install and uninstall steps, moved as they were, until
//! the change that commits all or nothing replaces them.

use crate::Installed;
use crate::layout::MODULE_FILE;
use crate::lock::{LockFile, LockFileEntry};
use crate::module::hex_sha256;
use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::PackageName;
use std::path::Path;

/// Result of an install operation.
#[derive(Debug)]
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub wasm_hash: String,
}

/// Install an extension from downloaded bytes.
/// Steps: verify SHA256 -> place binary (atomic via temp dir) -> update lock file.
#[allow(clippy::too_many_arguments)]
pub fn install_extension(
    name: &PackageName,
    version: &str,
    wasm_bytes: &[u8],
    expected_sha256: &str,
    installed: &Installed,
    lock: &mut LockFile,
    key_id: Option<&str>,
    peer_dependencies: Vec<specforge_protocol_types::PeerDependency>,
) -> Result<InstallResult, Diagnostic> {
    // 1. Verify SHA256
    let actual_hash = hex_sha256(wasm_bytes);
    if actual_hash != expected_sha256 {
        return Err(Diagnostic::new(
            codes::E032,
            format!(
                "integrity check failed for '{}': expected {}, got {}",
                name, expected_sha256, actual_hash
            ),
        )
        .with_suggestion("re-download the extension or verify the source".to_string()));
    }

    // 2. Atomic placement: write to temp dir, then rename
    let extensions_dir = installed.extensions_dir();
    let ext_dir = extensions_dir.join(name.relative_path());
    let temp_dir = extensions_dir.join(format!(".{}.tmp", name));

    // Clean up any leftover temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        Diagnostic::new(
            codes::E032,
            format!("failed to create temp directory for '{}': {}", name, e),
        )
    })?;

    let temp_wasm_path = temp_dir.join(MODULE_FILE);
    if let Err(e) = std::fs::write(&temp_wasm_path, wasm_bytes) {
        let _ = rollback_install(&temp_dir);
        return Err(Diagnostic::new(
            codes::E032,
            format!("failed to write .wasm binary for '{}': {}", name, e),
        ));
    }

    // Remove existing ext_dir if present (upgrade case)
    let _ = std::fs::remove_dir_all(&ext_dir);

    // Ensure parent directory exists (for scoped names like @scope/name)
    if let Some(parent) = ext_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Rename temp dir to final location (atomic on same filesystem)
    if let Err(e) = std::fs::rename(&temp_dir, &ext_dir) {
        let _ = rollback_install(&temp_dir);
        return Err(Diagnostic::new(
            codes::E032,
            format!("failed to finalize installation of '{}': {}", name, e),
        ));
    }

    // 3. Update lock file
    if let Some(existing) = lock.entries.iter_mut().find(|e| e.name == name.as_str()) {
        existing.version = version.to_string();
        existing.wasm_hash = actual_hash.clone();
        existing.source = "registry".to_string();
        existing.key_id = key_id.map(str::to_string);
    } else {
        lock.entries.push(LockFileEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: "registry".to_string(),
            wasm_hash: actual_hash.clone(),
            key_id: key_id.map(str::to_string),
            peer_dependencies,
        });
    }

    Ok(InstallResult {
        name: name.to_string(),
        version: version.to_string(),
        wasm_hash: actual_hash,
    })
}

/// Rollback: remove extension directory if it was partially created.
fn rollback_install(ext_dir: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if ext_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(ext_dir)
    {
        diagnostics.push(
            Diagnostic::new(
                codes::W119,
                format!(
                    "failed to clean up partial install at '{}': {}",
                    ext_dir.display(),
                    e
                ),
            )
            .with_suggestion(format!("manually remove '{}'", ext_dir.display())),
        );
    }
    diagnostics
}

/// Uninstall `name`: remove it from `lock`, then delete its directory
/// (the lock entry is restored when that fails). Whether other extensions
/// still need it is the caller's to decide first.
pub fn uninstall_extension(
    name: &PackageName,
    installed: &Installed,
    lock: &mut LockFile,
) -> Result<(), Diagnostic> {
    // 1. Keep the entry, to restore it if the directory cannot be removed.
    let entry = lock
        .entries
        .iter()
        .find(|e| e.name == name.as_str())
        .cloned();

    // 2. Remove from lock file
    lock.entries.retain(|e| e.name != name.as_str());

    // 3. Delete .wasm binary directory
    let ext_dir = installed.extensions_dir().join(name.relative_path());
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
