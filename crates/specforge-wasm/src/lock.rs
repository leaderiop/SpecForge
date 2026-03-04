use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::loader::compute_sha256;
use crate::manifest::PackageManifest;

/// A lock file recording the exact versions and integrity hashes of installed Wasm packages.
///
/// Written as `specforge.lock` in the project root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockFile {
    /// Lock file schema version.
    pub version: String,
    /// Locked packages in load order.
    pub packages: Vec<LockedPackage>,
}

/// A single locked package entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package name (e.g., `@specforge/hexagonal`).
    pub package: String,
    /// Package version (semver).
    #[serde(rename = "version")]
    pub pkg_version: String,
    /// Source specifier (local path, registry name, or git URL).
    pub source: String,
    /// Integrity hash of the `.wasm` binary (`sha256:<hex>`).
    pub integrity: String,
}

/// A mismatch between the lock file and a discovered manifest.
#[derive(Debug)]
pub struct LockMismatch {
    pub package: String,
    pub expected_integrity: String,
    pub actual_integrity: String,
}

impl std::fmt::Display for LockMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "lock mismatch for {}: expected {}, found {}",
            self.package,
            &self.expected_integrity[..self.expected_integrity.len().min(20)],
            &self.actual_integrity[..self.actual_integrity.len().min(20)],
        )
    }
}

impl LockFile {
    /// Read a lock file from disk.
    pub fn read(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read lock file: {e}"))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("cannot parse lock file: {e}"))
    }

    /// Write this lock file to disk.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("cannot serialize lock file: {e}"))?;
        std::fs::write(path, format!("{json}\n"))
            .map_err(|e| format!("cannot write lock file: {e}"))
    }

    /// Create a lock file from discovered manifests.
    ///
    /// Reads each `.wasm` binary to compute its integrity hash.
    pub fn from_manifests(manifests: &[PackageManifest]) -> Self {
        let packages = manifests
            .iter()
            .filter_map(|m| {
                let wasm_bytes = std::fs::read(&m.wasm_path).ok()?;
                let hash = compute_sha256(&wasm_bytes);
                Some(LockedPackage {
                    package: m.package.clone(),
                    pkg_version: m.version.clone(),
                    source: m.manifest_path.to_string_lossy().to_string(),
                    integrity: format!("sha256:{hash}"),
                })
            })
            .collect();

        Self {
            version: "1".to_string(),
            packages,
        }
    }

    /// Verify that discovered manifests match the lock file entries.
    ///
    /// Returns a list of mismatches (empty = all good).
    pub fn verify(&self, manifests: &[PackageManifest]) -> Vec<LockMismatch> {
        let mut mismatches = Vec::new();

        for locked in &self.packages {
            if let Some(manifest) = manifests.iter().find(|m| m.package == locked.package) {
                if let Ok(wasm_bytes) = std::fs::read(&manifest.wasm_path) {
                    let actual_hash = format!("sha256:{}", compute_sha256(&wasm_bytes));
                    if actual_hash != locked.integrity {
                        mismatches.push(LockMismatch {
                            package: locked.package.clone(),
                            expected_integrity: locked.integrity.clone(),
                            actual_integrity: actual_hash,
                        });
                    }
                }
            }
        }

        mismatches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_test_manifest(dir: &Path, name: &str, wasm_content: &[u8]) -> PackageManifest {
        let manifest_path = dir.join("manifest.json");
        let wasm_path = dir.join("plugin.wasm");
        std::fs::write(&manifest_path, "{}").unwrap();
        let mut f = std::fs::File::create(&wasm_path).unwrap();
        f.write_all(wasm_content).unwrap();

        PackageManifest {
            package: name.to_string(),
            manifest_version: "1".to_string(),
            kind: crate::manifest::PluginKind::Plugin,
            contributes: crate::manifest::PackageContributions::default(),
            wasm: "plugin.wasm".to_string(),
            description: String::new(),
            version: "1.0.0".to_string(),
            enhancements: vec![],
            dynamic_edge_types: vec![],
            entity_kinds: vec![],
            provider: None,
            generator: None,
            sandbox: crate::manifest::SandboxPolicy::default(),
            peer_dependencies: std::collections::HashMap::new(),
            query_extensions: vec![],
            manifest_path,
            wasm_path,
        }
    }

    #[test]
    fn lock_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = make_test_manifest(dir.path(), "@test/plugin", b"wasm bytes");

        let lock = LockFile::from_manifests(&[manifest]);
        assert_eq!(lock.packages.len(), 1);
        assert!(lock.packages[0].integrity.starts_with("sha256:"));

        let lock_path = dir.path().join("specforge.lock");
        lock.write(&lock_path).unwrap();

        let read_back = LockFile::read(&lock_path).unwrap();
        assert_eq!(read_back.packages.len(), 1);
        assert_eq!(read_back.packages[0].package, "@test/plugin");
        assert_eq!(read_back.packages[0].integrity, lock.packages[0].integrity);
    }

    #[test]
    fn lock_file_verify_matches() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = make_test_manifest(dir.path(), "@test/plugin", b"wasm content");

        let lock = LockFile::from_manifests(&[manifest.clone()]);
        let mismatches = lock.verify(&[manifest]);
        assert!(mismatches.is_empty());
    }

    #[test]
    fn lock_file_verify_detects_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = make_test_manifest(dir.path(), "@test/plugin", b"original");

        let lock = LockFile::from_manifests(&[manifest.clone()]);

        // Overwrite the wasm file with different content
        std::fs::write(&manifest.wasm_path, b"modified").unwrap();

        let mismatches = lock.verify(&[manifest]);
        assert_eq!(mismatches.len(), 1);
        assert_eq!(mismatches[0].package, "@test/plugin");
    }

    #[test]
    fn lock_file_from_empty() {
        let lock = LockFile::from_manifests(&[]);
        assert!(lock.packages.is_empty());
        assert_eq!(lock.version, "1");
    }

    #[test]
    fn lock_file_read_not_found() {
        let result = LockFile::read(Path::new("/nonexistent/specforge.lock"));
        assert!(result.is_err());
    }

    #[test]
    fn lock_mismatch_display() {
        let mismatch = LockMismatch {
            package: "@test/plugin".to_string(),
            expected_integrity: "sha256:aabbccdd".to_string(),
            actual_integrity: "sha256:11223344".to_string(),
        };
        let display = format!("{mismatch}");
        assert!(display.contains("@test/plugin"));
        assert!(display.contains("lock mismatch"));
    }
}
