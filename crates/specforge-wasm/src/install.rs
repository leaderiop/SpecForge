use crate::cache::aot_compile;
use crate::integrity::hex_sha256;
use crate::lock_file::{LockFile, LockFileEntry};
use specforge_common::{Diagnostic, Severity};
use std::path::{Path, PathBuf};

/// Result of an install operation.
#[derive(Debug)]
pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub wasm_hash: String,
    pub aot_compiled: bool,
}

/// Install an extension from downloaded bytes.
/// Steps: verify SHA256 -> place binary (atomic via temp dir) -> AOT compile -> update lock file.
#[allow(clippy::too_many_arguments)]
pub fn install_extension(
    name: &str,
    version: &str,
    wasm_bytes: &[u8],
    expected_sha256: &str,
    extensions_dir: &Path,
    cache_dir: &Path,
    lock: &mut LockFile,
    skip_aot: bool,
) -> Result<InstallResult, Diagnostic> {
    // 1. Verify SHA256
    let actual_hash = hex_sha256(wasm_bytes);
    if actual_hash != expected_sha256 {
        return Err(Diagnostic {
            code: "E032".to_string(),
            severity: Severity::Error,
            message: format!(
                "integrity check failed for '{}': expected {}, got {}",
                name, expected_sha256, actual_hash
            ),
            span: None,
            suggestion: Some("re-download the extension or verify the source".to_string()),
        });
    }

    // 2. Atomic placement: write to temp dir, then rename
    let ext_dir = extensions_dir.join(name);
    let temp_dir = extensions_dir.join(format!(".{}.tmp", name));

    // Clean up any leftover temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!("failed to create temp directory for '{}': {}", name, e),
            span: None,
            suggestion: None,
        }
    })?;

    let temp_wasm_path = temp_dir.join("extension.wasm");
    if let Err(e) = std::fs::write(&temp_wasm_path, wasm_bytes) {
        let _ = rollback_install(&temp_dir);
        return Err(Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!("failed to write .wasm binary for '{}': {}", name, e),
            span: None,
            suggestion: None,
        });
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
        return Err(Diagnostic {
            code: "E033".to_string(),
            severity: Severity::Error,
            message: format!("failed to finalize installation of '{}': {}", name, e),
            span: None,
            suggestion: None,
        });
    }

    // 3. AOT compile (optional)
    let aot_compiled = if !skip_aot {
        let wasm_path = ext_dir.join("extension.wasm");
        aot_compile(&wasm_path, cache_dir).is_ok()
    } else {
        false
    };

    // 4. Update lock file
    if let Some(existing) = lock.entries.iter_mut().find(|e| e.name == name) {
        existing.version = version.to_string();
        existing.wasm_hash = actual_hash.clone();
        existing.source = "registry".to_string();
    } else {
        lock.entries.push(LockFileEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: "registry".to_string(),
            wasm_hash: actual_hash.clone(),
        });
    }

    Ok(InstallResult {
        name: name.to_string(),
        version: version.to_string(),
        wasm_hash: actual_hash,
        aot_compiled,
    })
}

/// Install an extension from a local path (copy).
pub fn install_from_local(
    name: &str,
    version: &str,
    local_wasm_path: &Path,
    extensions_dir: &Path,
    cache_dir: &Path,
    lock: &mut LockFile,
    skip_aot: bool,
) -> Result<InstallResult, Diagnostic> {
    let wasm_bytes = std::fs::read(local_wasm_path).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!(
            "cannot read local .wasm binary at '{}': {}",
            local_wasm_path.display(),
            e
        ),
        span: None,
        suggestion: None,
    })?;

    let hash = hex_sha256(&wasm_bytes);

    // Use install_extension with the computed hash as expected (always matches)
    install_extension(name, version, &wasm_bytes, &hash, extensions_dir, cache_dir, lock, skip_aot)
}

/// Rollback: remove extension directory if it was partially created.
fn rollback_install(ext_dir: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if ext_dir.exists()
        && let Err(e) = std::fs::remove_dir_all(ext_dir)
    {
        diagnostics.push(Diagnostic {
            code: "W033".to_string(),
            severity: Severity::Warning,
            message: format!(
                "failed to clean up partial install at '{}': {}",
                ext_dir.display(),
                e
            ),
            span: None,
            suggestion: Some(format!("manually remove '{}'", ext_dir.display())),
        });
    }
    diagnostics
}

/// Get the wasm path for an installed extension.
pub fn installed_wasm_path(extensions_dir: &Path, name: &str) -> PathBuf {
    extensions_dir.join(name).join("extension.wasm")
}
