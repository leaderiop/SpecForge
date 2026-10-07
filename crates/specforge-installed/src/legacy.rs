//! Today's separate install, uninstall and load steps, moved as they were.
//! Each is replaced, and this file shrinks with it: `load_wasm_module` by
//! the extension load, the other two by the change that commits all or
//! nothing.

use crate::Installed;
use crate::layout::MODULE_FILE;
use crate::lock::{LockFile, LockFileEntry};
use crate::module::hex_sha256;
use specforge_common::{Diagnostic, codes};
use specforge_protocol_types::PackageName;
use specforge_wasm::WasmRuntime;
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

/// Load a Wasm component from `wasm_path` under `extension_name`.
///
/// `expected_hash` enforces the lockfile pin (spec #21, T4): when it carries
/// a hash (from `specforge.lock`), the on-disk binary must match it — a
/// mismatch refuses the load with a remediation hint. `None` or an empty
/// string (legacy lockfile entries from before hash pinning) load unchanged.
pub fn load_wasm_module(
    extension_name: &str,
    wasm_path: &Path,
    runtime: &dyn WasmRuntime,
    expected_hash: Option<&str>,
) -> Result<(), Diagnostic> {
    // Check if the .wasm binary exists
    if !wasm_path.exists() {
        return Err(Diagnostic::new(
            codes::E028,
            format!(
                "extension '{}': .wasm binary not found at '{}'",
                extension_name,
                wasm_path.display()
            ),
        )
        .with_suggestion(format!(
            "install the extension with: specforge add {}",
            extension_name
        )));
    }

    // The binary's content hash, checked against the lockfile pin.
    let bytes = std::fs::read(wasm_path).map_err(|e| {
        Diagnostic::new(
            codes::E028,
            format!(
                "extension '{}': cannot read .wasm binary at '{}': {}",
                extension_name,
                wasm_path.display(),
                e
            ),
        )
    })?;
    let wasm_hash = hex_sha256(&bytes);

    // Lockfile pin enforcement (spec #21, T4): a recorded hash that no
    // longer matches the on-disk binary means the installed extension was
    // tampered with or corrupted after install. Refuse before touching the
    // runtime (this also denies a tampered binary a cache-hit load path).
    if let Some(expected) = expected_hash.filter(|h| !h.is_empty() && h != &wasm_hash) {
        return Err(Diagnostic::new(
            codes::E033,
            format!(
                "integrity mismatch for '{}': lockfile records hash {} but the installed binary is {}",
                extension_name, expected, wasm_hash
            ),
        )
        .with_suggestion(format!(
            "the installed binary changed after install — re-install it: specforge remove \"{0}\" && specforge add \"{0}\"",
            extension_name
        )));
    }

    runtime
        .load_module_named(extension_name, wasm_path)
        .map_err(|e| {
            Diagnostic::new(
                codes::E028,
                format!(
                    "extension '{}': failed to load Wasm module: {}",
                    extension_name, e
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::Severity;
    use specforge_extension_sdk::{ContributionsBuilder, ExtensionMeta};
    use specforge_wasm::testing::InProcessRuntime;
    use std::io::Write;
    use tempfile::TempDir;

    /// A runtime that serves `test-ext`, so loading its binary succeeds.
    fn runtime() -> InProcessRuntime {
        InProcessRuntime::new()
            .with(|| ContributionsBuilder::new(ExtensionMeta::new("test-ext", "1.0.0")))
    }

    fn create_fake_wasm(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"\x00asm\x01\x00\x00\x00fake").unwrap();
        path
    }

    // B:load_wasm_module — verify unit "loads .wasm binary from manifest path"
    #[test]
    fn test_loads_wasm_binary_from_manifest_path() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = runtime();

        load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();
    }

    // B:load_wasm_module — verify unit "missing .wasm produces ExtensionError"
    #[test]
    fn test_missing_wasm_produces_extension_error() {
        let runtime = runtime();
        let missing = Path::new("/nonexistent/ext.wasm");

        let err = load_wasm_module("test-ext", missing, &runtime, None).unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("not found"));
    }

    // B:load_wasm_module — verify contract "requires/ensures consistency for Wasm module loading"
    #[test]
    fn test_load_wasm_module_contract() {
        let dir = TempDir::new().unwrap();
        let wasm_path = create_fake_wasm(&dir, "ext.wasm");
        let runtime = runtime();

        // ensures: extension_loaded on success
        load_wasm_module("test-ext", &wasm_path, &runtime, None).unwrap();

        // ensures: missing_binary_diagnosed
        let missing = Path::new("/nonexistent.wasm");
        let err = load_wasm_module("missing", missing, &runtime, None).unwrap_err();
        assert_eq!(err.code, "E028");
        assert_eq!(err.severity, Severity::Error);
    }
}
