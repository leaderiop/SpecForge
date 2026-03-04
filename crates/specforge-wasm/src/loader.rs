use sha2::{Digest, Sha256};
use std::path::Path;

use crate::error::WasmError;
use crate::manifest::{PluginLifecycleState, PackageManifest};

/// A Wasm plugin that has been loaded into memory.
pub struct LoadedPackage {
    /// The parsed manifest.
    pub manifest: PackageManifest,
    /// The Extism plugin instance.
    pub plugin: extism::Plugin,
    /// Current lifecycle state.
    pub state: PluginLifecycleState,
    /// SHA-256 hash of the `.wasm` binary.
    pub wasm_hash: String,
}

impl std::fmt::Debug for LoadedPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPackage")
            .field("package", &self.manifest.package)
            .field("state", &self.state)
            .field("wasm_hash", &self.wasm_hash)
            .finish()
    }
}

/// Load a Wasm manifest from a directory path containing `manifest.json`.
///
/// Resolves the `.wasm` binary path relative to the manifest and validates it exists.
pub fn load_manifest(manifest_path: &Path) -> Result<PackageManifest, WasmError> {
    if !manifest_path.exists() {
        return Err(WasmError::ManifestNotFound {
            path: manifest_path.to_path_buf(),
        });
    }

    let content = std::fs::read_to_string(manifest_path).map_err(|e| {
        WasmError::ManifestParseError {
            path: manifest_path.to_path_buf(),
            message: e.to_string(),
        }
    })?;

    let mut manifest: PackageManifest =
        serde_json::from_str(&content).map_err(|e| WasmError::ManifestParseError {
            path: manifest_path.to_path_buf(),
            message: e.to_string(),
        })?;

    // Migrate v1 manifests: populate `contributes` from `kind` if needed
    manifest.migrate_v1_if_needed();

    let manifest_dir = manifest_path
        .parent()
        .unwrap_or(Path::new("."));

    manifest.manifest_path = manifest_path.to_path_buf();
    manifest.wasm_path = manifest_dir.join(&manifest.wasm);

    if !manifest.wasm_path.exists() {
        return Err(WasmError::WasmBinaryNotFound {
            path: manifest.wasm_path.clone(),
            manifest_package: manifest.package.clone(),
        });
    }

    Ok(manifest)
}

/// Load a Wasm module from a manifest into an Extism plugin instance.
///
/// Reads the `.wasm` binary, computes its SHA-256 hash, and creates an `extism::Plugin`.
pub fn load_wasm_module(
    manifest: PackageManifest,
    host_functions: Vec<extism::Function>,
) -> Result<LoadedPackage, WasmError> {
    let wasm_bytes = std::fs::read(&manifest.wasm_path).map_err(|e| {
        WasmError::WasmLoadFailed {
            package: manifest.package.clone(),
            message: format!("failed to read wasm binary: {e}"),
        }
    })?;

    let wasm_hash = compute_sha256(&wasm_bytes);

    let extism_manifest = extism::Manifest::new([extism::Wasm::data(wasm_bytes)])
        .with_memory_max((manifest.sandbox.max_memory_bytes / 65536) as u32);

    let plugin = extism::Plugin::new(&extism_manifest, host_functions, true).map_err(|e| {
        WasmError::WasmLoadFailed {
            package: manifest.package.clone(),
            message: e.to_string(),
        }
    })?;

    Ok(LoadedPackage {
        manifest,
        plugin,
        state: PluginLifecycleState::Loading,
        wasm_hash,
    })
}

/// Compute the SHA-256 hash of a byte slice, returning a hex string.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn manifest_not_found() {
        let result = load_manifest(Path::new("/nonexistent/manifest.json"));
        assert!(matches!(result, Err(WasmError::ManifestNotFound { .. })));
    }

    #[test]
    fn manifest_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(&manifest_path, "not json").unwrap();
        let result = load_manifest(&manifest_path);
        assert!(matches!(result, Err(WasmError::ManifestParseError { .. })));
    }

    #[test]
    fn manifest_wasm_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");
        std::fs::write(
            &manifest_path,
            r#"{"package": "@test/plugin", "wasm": "missing.wasm"}"#,
        )
        .unwrap();
        let result = load_manifest(&manifest_path);
        assert!(matches!(
            result,
            Err(WasmError::WasmBinaryNotFound { .. })
        ));
    }

    #[test]
    fn load_manifest_success() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join("manifest.json");
        let wasm_path = dir.path().join("plugin.wasm");

        // Create a dummy wasm file
        let mut f = std::fs::File::create(&wasm_path).unwrap();
        f.write_all(b"\0asm\x01\x00\x00\x00").unwrap();

        std::fs::write(
            &manifest_path,
            r#"{"package": "@test/plugin", "wasm": "plugin.wasm"}"#,
        )
        .unwrap();

        let manifest = load_manifest(&manifest_path).unwrap();
        assert_eq!(manifest.package, "@test/plugin");
        assert_eq!(manifest.wasm_path, wasm_path);
        assert_eq!(manifest.manifest_path, manifest_path);
    }

    #[test]
    fn compute_sha256_deterministic() {
        let data = b"hello world";
        let hash1 = compute_sha256(data);
        let hash2 = compute_sha256(data);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn compute_sha256_different_inputs() {
        let hash1 = compute_sha256(b"hello");
        let hash2 = compute_sha256(b"world");
        assert_ne!(hash1, hash2);
    }
}
