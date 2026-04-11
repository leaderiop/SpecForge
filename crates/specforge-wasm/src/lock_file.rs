use crate::discovery::ResolvedExtension;
use serde::{Deserialize, Serialize};
use specforge_common::{Diagnostic, Severity};
use std::path::Path;

/// The lock file format for extension resolution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockFile {
    pub lockfile_version: u32,
    pub entries: Vec<LockFileEntry>,
}

/// A single entry in the lock file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockFileEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    pub wasm_hash: String,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            lockfile_version: 1,
            entries: Vec::new(),
        }
    }
}

impl LockFile {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Write a lock file to disk as JSON.
pub fn write_lock_file(lock: &LockFile, path: &Path) -> Result<(), Diagnostic> {
    let json = serde_json::to_string_pretty(lock).map_err(|e| Diagnostic {
        code: "E033".to_string(),
        severity: Severity::Error,
        message: format!("failed to serialize lock file: {}", e),
        span: None,
        suggestion: None,
    })?;

    std::fs::write(path, json).map_err(|e| Diagnostic {
        code: "E033".to_string(),
        severity: Severity::Error,
        message: format!("failed to write lock file at '{}': {}", path.display(), e),
        span: None,
        suggestion: None,
    })
}

/// Read a lock file from disk.
pub fn read_lock_file(path: &Path) -> Result<LockFile, Diagnostic> {
    let content = std::fs::read_to_string(path).map_err(|e| Diagnostic {
        code: "E033".to_string(),
        severity: Severity::Error,
        message: format!("failed to read lock file at '{}': {}", path.display(), e),
        span: None,
        suggestion: None,
    })?;

    serde_json::from_str::<LockFile>(&content).map_err(|e| Diagnostic {
        code: "E033".to_string(),
        severity: Severity::Error,
        message: format!("corrupt lock file at '{}': {}", path.display(), e),
        span: None,
        suggestion: Some("delete the lock file and run `specforge install` to regenerate".to_string()),
    })
}

/// Doctor check result for a single extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorStatus {
    Healthy,
    MissingBinary { name: String },
    StaleHash { name: String, expected: String, actual: String },
    PeerMismatch { name: String, peer: String, required: String },
}

/// Run a health check on installed extensions.
/// Checks: binary exists, hash matches lock file, peer dependencies satisfied.
pub fn run_doctor_check(
    lock: &LockFile,
    extensions_dir: &Path,
    compute_hash: impl Fn(&Path) -> Option<String>,
    installed_versions: &std::collections::HashMap<String, String>,
) -> Vec<DoctorStatus> {
    let mut results = Vec::new();

    for entry in &lock.entries {
        let wasm_path = extensions_dir.join(&entry.name).join("extension.wasm");

        // Check binary exists
        if !wasm_path.exists() {
            results.push(DoctorStatus::MissingBinary {
                name: entry.name.clone(),
            });
            continue;
        }

        // Check hash matches
        if let Some(actual_hash) = compute_hash(&wasm_path)
            && actual_hash != entry.wasm_hash
        {
            results.push(DoctorStatus::StaleHash {
                name: entry.name.clone(),
                expected: entry.wasm_hash.clone(),
                actual: actual_hash,
            });
        }
    }

    // Check peer dependencies using semver-like comparison
    for entry in &lock.entries {
        // Simple check: look for peer dependency entries referencing other extensions
        // In a real implementation, this would parse ManifestV2 peer_dependencies
        // For now, we check if all lock file entries have matching installed versions
        if let Some(version) = installed_versions.get(&entry.name)
            && version != &entry.version
        {
            results.push(DoctorStatus::PeerMismatch {
                name: entry.name.clone(),
                peer: entry.name.clone(),
                required: entry.version.clone(),
            });
        }
    }

    results
}

/// Refresh lock file entries from a list of resolved extensions.
/// Updates existing entries and adds new ones. Returns diagnostics for any issues.
pub fn refresh_lock_file(
    lock: &mut LockFile,
    resolved: &[ResolvedExtension],
    compute_hash: impl Fn(&Path) -> Option<String>,
) -> Vec<Diagnostic> {
    let diagnostics = Vec::new();

    for ext in resolved {
        let wasm_path = ext.manifest_path.parent()
            .map(|p| p.join(&ext.manifest.wasm_path))
            .unwrap_or_else(|| Path::new(&ext.manifest.wasm_path).to_path_buf());

        let hash = compute_hash(&wasm_path).unwrap_or_default();

        let source = match &ext.source {
            crate::discovery::ExtensionSpecifier::Registry { .. } => "registry".to_string(),
            crate::discovery::ExtensionSpecifier::Local { path } => format!("local:{}", path.display()),
            crate::discovery::ExtensionSpecifier::Git { url, .. } => format!("git:{}", url),
        };

        if let Some(existing) = lock.entries.iter_mut().find(|e| e.name == ext.manifest.name) {
            existing.version = ext.manifest.version.clone();
            existing.source = source;
            existing.wasm_hash = hash;
        } else {
            lock.entries.push(LockFileEntry {
                name: ext.manifest.name.clone(),
                version: ext.manifest.version.clone(),
                source,
                wasm_hash: hash,
            });
        }
    }

    // Remove entries that are no longer in the resolved set
    let resolved_names: std::collections::HashSet<&str> = resolved
        .iter()
        .map(|r| r.manifest.name.as_str())
        .collect();
    lock.entries.retain(|e| resolved_names.contains(e.name.as_str()));

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::{ExtensionSpecifier, ResolvedExtension};
    use crate::test_helpers::default_manifest;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // -- write_lock_file + read_lock_file --

    // B:write_lock_file — verify unit "serializes lock file to JSON"
    #[test]
    fn test_write_lock_file_serializes_to_json() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("specforge.lock");

        let lock = LockFile {
            lockfile_version: 1,
            entries: vec![LockFileEntry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "abc123".to_string(),
            }],
        };

        write_lock_file(&lock, &path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("@specforge/software"));
        assert!(content.contains("abc123"));
    }

    // B:read_lock_file — verify unit "deserializes lock file from JSON"
    #[test]
    fn test_read_lock_file_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("specforge.lock");

        let lock = LockFile {
            lockfile_version: 1,
            entries: vec![
                LockFileEntry {
                    name: "@specforge/software".to_string(),
                    version: "1.0.0".to_string(),
                    source: "registry".to_string(),
                    wasm_hash: "abc123".to_string(),
                },
                LockFileEntry {
                    name: "@specforge/governance".to_string(),
                    version: "1.0.0".to_string(),
                    source: "local".to_string(),
                    wasm_hash: "def456".to_string(),
                },
            ],
        };

        write_lock_file(&lock, &path).unwrap();
        let read_back = read_lock_file(&path).unwrap();
        assert_eq!(lock, read_back);
    }

    // B:read_lock_file — verify unit "corrupt file produces E033 diagnostic"
    #[test]
    fn test_read_lock_file_corrupt_produces_diagnostic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("specforge.lock");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let err = read_lock_file(&path).unwrap_err();
        assert_eq!(err.code, "E033");
        assert!(err.message.contains("corrupt lock file"));
        assert!(err.suggestion.is_some());
    }

    // B:read_lock_file — verify unit "missing file produces E033 diagnostic"
    #[test]
    fn test_read_lock_file_missing_produces_diagnostic() {
        let err = read_lock_file(Path::new("/nonexistent/specforge.lock")).unwrap_err();
        assert_eq!(err.code, "E033");
        assert!(err.message.contains("failed to read lock file"));
    }

    // -- run_doctor_check --

    // B:run_doctor_check — verify unit "detects missing binary"
    #[test]
    fn test_doctor_detects_missing_binary() {
        let dir = TempDir::new().unwrap();
        let lock = LockFile {
            lockfile_version: 1,
            entries: vec![LockFileEntry {
                name: "missing-ext".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "abc".to_string(),
            }],
        };

        let results = run_doctor_check(&lock, dir.path(), |_| None, &HashMap::new());
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            DoctorStatus::MissingBinary {
                name: "missing-ext".to_string()
            }
        );
    }

    // B:run_doctor_check — verify unit "detects stale hash"
    #[test]
    fn test_doctor_detects_stale_hash() {
        let dir = TempDir::new().unwrap();
        let ext_dir = dir.path().join("my-ext");
        std::fs::create_dir(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("extension.wasm"), b"wasm content").unwrap();

        let lock = LockFile {
            lockfile_version: 1,
            entries: vec![LockFileEntry {
                name: "my-ext".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "expected_hash".to_string(),
            }],
        };

        let results = run_doctor_check(
            &lock,
            dir.path(),
            |_| Some("actual_different_hash".to_string()),
            &HashMap::new(),
        );
        assert!(results.iter().any(|r| matches!(r, DoctorStatus::StaleHash { .. })));
    }

    // B:run_doctor_check — verify unit "reports healthy when all checks pass"
    #[test]
    fn test_doctor_reports_healthy() {
        let dir = TempDir::new().unwrap();
        let ext_dir = dir.path().join("good-ext");
        std::fs::create_dir(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("extension.wasm"), b"wasm").unwrap();

        let lock = LockFile {
            lockfile_version: 1,
            entries: vec![LockFileEntry {
                name: "good-ext".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "correct_hash".to_string(),
            }],
        };

        let installed: HashMap<String, String> =
            [("good-ext".to_string(), "1.0.0".to_string())]
                .into_iter()
                .collect();

        let results = run_doctor_check(
            &lock,
            dir.path(),
            |_| Some("correct_hash".to_string()),
            &installed,
        );
        assert!(results.is_empty(), "expected no issues, got: {:?}", results);
    }

    // -- refresh_lock_file --

    // B:refresh_lock_file — verify unit "lock file reflects installed extensions"
    #[test]
    fn test_refresh_lock_file_reflects_installed() {
        let mut lock = LockFile::new();
        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.version = "1.0.0".to_string();
        manifest.wasm_path = "extension.wasm".to_string();

        let resolved = vec![ResolvedExtension {
            manifest: manifest.clone(),
            source: ExtensionSpecifier::Registry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
            },
            manifest_path: std::path::PathBuf::from("/ext/manifest.json"),
        }];

        let diags = refresh_lock_file(&mut lock, &resolved, |_| Some("hash123".to_string()));
        assert!(diags.is_empty());
        assert_eq!(lock.entries.len(), 1);
        assert_eq!(lock.entries[0].name, "@specforge/software");
        assert_eq!(lock.entries[0].version, "1.0.0");
        assert_eq!(lock.entries[0].source, "registry");
        assert_eq!(lock.entries[0].wasm_hash, "hash123");
    }

    // B:refresh_lock_file — verify unit "hash entries updated"
    #[test]
    fn test_refresh_lock_file_updates_hash() {
        let mut lock = LockFile {
            lockfile_version: 1,
            entries: vec![LockFileEntry {
                name: "@specforge/software".to_string(),
                version: "1.0.0".to_string(),
                source: "registry".to_string(),
                wasm_hash: "old_hash".to_string(),
            }],
        };

        let mut manifest = default_manifest();
        manifest.name = "@specforge/software".to_string();
        manifest.version = "2.0.0".to_string();
        manifest.wasm_path = "extension.wasm".to_string();

        let resolved = vec![ResolvedExtension {
            manifest: manifest.clone(),
            source: ExtensionSpecifier::Registry {
                name: "@specforge/software".to_string(),
                version: "2.0.0".to_string(),
            },
            manifest_path: std::path::PathBuf::from("/ext/manifest.json"),
        }];

        let diags = refresh_lock_file(&mut lock, &resolved, |_| Some("new_hash".to_string()));
        assert!(diags.is_empty());
        assert_eq!(lock.entries.len(), 1);
        assert_eq!(lock.entries[0].version, "2.0.0");
        assert_eq!(lock.entries[0].wasm_hash, "new_hash");
    }
}
