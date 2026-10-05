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
    /// Publisher key id recorded at install from a signed registry package.
    /// `None` for local installs and for lock files written before signed
    /// publishing existed (field defaults on deserialize for compatibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    /// Peer requirements recorded at install from the extension's manifest
    /// (C8-05: doctor verifies these across the other lock entries). Defaults
    /// on deserialize for lock files written before this existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peer_dependencies: Vec<specforge_registry::PeerDependency>,
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
        data: None,
    })?;

    // Write a sibling file, then rename it over the lock: a write that
    // fails part-way (a full disk) leaves the old lock whole.
    let mut temp_name = path.file_name().unwrap_or_default().to_os_string();
    temp_name.push(".tmp");
    let temp = path.with_file_name(temp_name);
    std::fs::write(&temp, json)
        .and_then(|()| std::fs::rename(&temp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&temp);
            Diagnostic {
                code: "E033".to_string(),
                severity: Severity::Error,
                message: format!("failed to write lock file at '{}': {}", path.display(), e),
                span: None,
                suggestion: None,
                data: None,
            }
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
        data: None,
    })?;

    serde_json::from_str::<LockFile>(&content).map_err(|e| Diagnostic {
        code: "E033".to_string(),
        severity: Severity::Error,
        message: format!("corrupt lock file at '{}': {}", path.display(), e),
        span: None,
        suggestion: Some(
            "delete the lock file and run `specforge install` to regenerate".to_string(),
        ),
        data: None,
    })
}

/// Doctor check result for a single extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorStatus {
    Healthy,
    MissingBinary {
        name: String,
    },
    StaleHash {
        name: String,
        expected: String,
        actual: String,
    },
    /// `name` requires `peer` at the range `required`; `installed` is the
    /// version the lock records for the peer (`None`: not installed). A
    /// recorded version that isn't semver, or a range that doesn't parse,
    /// can't be compared, so it is a mismatch too.
    PeerMismatch {
        name: String,
        peer: String,
        required: String,
        installed: Option<String>,
    },
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

    // C8-05: real peer-dependency verification. Each entry's recorded peers
    // must be present among the installed versions and satisfy the declared
    // semver requirement. Optional peers that are absent are fine.
    for entry in &lock.entries {
        for peer in &entry.peer_dependencies {
            let installed = installed_versions.get(&peer.name);
            let satisfied = match installed {
                None => peer.optional,
                Some(version) => match (
                    semver::VersionReq::parse(&peer.version),
                    semver::Version::parse(version),
                ) {
                    (Ok(req), Ok(v)) => req.matches(&v),
                    _ => false,
                },
            };
            if !satisfied {
                results.push(DoctorStatus::PeerMismatch {
                    name: entry.name.clone(),
                    peer: peer.name.clone(),
                    required: peer.version.clone(),
                    installed: installed.cloned(),
                });
            }
        }
    }

    results
}

/// Collect every requirer (name, range) declaring a peer dependency on
/// `peer_name` across all locked entries, plus one optional extra requirer —
/// the package currently being installed, which may not be in `lock` yet.
/// Used to unify a version diamond (C8-07) before it is silently locked.
pub fn collect_peer_requirers(
    lock: &LockFile,
    peer_name: &str,
    extra: Option<(&str, &str)>,
) -> Vec<(String, String)> {
    let mut requirers: Vec<(String, String)> = lock
        .entries
        .iter()
        .flat_map(|e| {
            e.peer_dependencies
                .iter()
                .filter(|p| p.name == peer_name)
                .map(move |p| (e.name.clone(), p.version.clone()))
        })
        .collect();
    if let Some((name, range)) = extra {
        requirers.push((name.to_string(), range.to_string()));
    }
    requirers
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
        let wasm_path = ext
            .manifest_path
            .parent()
            .map(|p| p.join(&ext.manifest.wasm_path))
            .unwrap_or_else(|| Path::new(&ext.manifest.wasm_path).to_path_buf());

        let hash = compute_hash(&wasm_path).unwrap_or_default();

        let source = match &ext.source {
            crate::discovery::ExtensionSpecifier::Registry { .. } => "registry".to_string(),
            crate::discovery::ExtensionSpecifier::Local { path } => {
                format!("local:{}", path.display())
            }
            crate::discovery::ExtensionSpecifier::Git { url, .. } => format!("git:{}", url),
        };

        if let Some(existing) = lock
            .entries
            .iter_mut()
            .find(|e| e.name == ext.manifest.name)
        {
            existing.version = ext.manifest.version.clone();
            existing.source = source;
            existing.wasm_hash = hash;
            existing.peer_dependencies = ext.manifest.peer_dependencies.clone();
        } else {
            lock.entries.push(LockFileEntry {
                name: ext.manifest.name.clone(),
                version: ext.manifest.version.clone(),
                source,
                wasm_hash: hash,
                key_id: None,
                peer_dependencies: ext.manifest.peer_dependencies.clone(),
            });
        }
    }

    // Remove entries that are no longer in the resolved set
    let resolved_names: std::collections::HashSet<&str> =
        resolved.iter().map(|r| r.manifest.name.as_str()).collect();
    lock.entries
        .retain(|e| resolved_names.contains(e.name.as_str()));

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
                key_id: None,
                peer_dependencies: Vec::new(),
            }],
        };

        write_lock_file(&lock, &path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("@specforge/software"));
        assert!(content.contains("abc123"));
    }

    #[test]
    fn a_failed_lock_write_leaves_the_old_lock_whole() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("specforge.lock");
        write_lock_file(&LockFile::new(), &path).unwrap();
        let before = std::fs::read(&path).unwrap();
        // The sibling it writes first cannot be created.
        std::fs::create_dir(dir.path().join("specforge.lock.tmp")).unwrap();

        let error = write_lock_file(&LockFile::new(), &path).unwrap_err();

        assert_eq!(error.code, "E033");
        assert_eq!(std::fs::read(&path).unwrap(), before);
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
                    key_id: None,
                    peer_dependencies: Vec::new(),
                },
                LockFileEntry {
                    name: "@specforge/governance".to_string(),
                    version: "1.0.0".to_string(),
                    source: "local".to_string(),
                    wasm_hash: "def456".to_string(),
                    key_id: None,
                    peer_dependencies: Vec::new(),
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
                key_id: None,
                peer_dependencies: Vec::new(),
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
                key_id: None,
                peer_dependencies: Vec::new(),
            }],
        };

        let results = run_doctor_check(
            &lock,
            dir.path(),
            |_| Some("actual_different_hash".to_string()),
            &HashMap::new(),
        );
        assert!(
            results
                .iter()
                .any(|r| matches!(r, DoctorStatus::StaleHash { .. }))
        );
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
                key_id: None,
                peer_dependencies: Vec::new(),
            }],
        };

        let installed: HashMap<String, String> = [("good-ext".to_string(), "1.0.0".to_string())]
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
                key_id: None,
                peer_dependencies: Vec::new(),
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

// C8-05 acceptance: doctor verifies recorded peers across OTHER entries.
#[cfg(test)]
mod peer_check_tests {
    use super::*;

    fn entry(name: &str, peers: Vec<specforge_registry::PeerDependency>) -> LockFileEntry {
        LockFileEntry {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: "hash".to_string(),
            key_id: None,
            peer_dependencies: peers,
        }
    }

    fn peer(name: &str, req: &str) -> specforge_registry::PeerDependency {
        specforge_registry::PeerDependency {
            name: name.to_string(),
            version: req.to_string(),
            optional: false,
        }
    }

    #[test]
    fn satisfied_peer_is_clean() {
        let lock = LockFile {
            entries: vec![
                entry("@a/ext", vec![peer("@b/lib", "^2.0.0")]),
                entry("@b/lib", vec![]),
            ],
            ..Default::default()
        };
        let installed = std::collections::HashMap::from([
            ("@a/ext".to_string(), "1.0.0".to_string()),
            ("@b/lib".to_string(), "2.1.0".to_string()),
        ]);
        let results = run_doctor_check(&lock, Path::new("/nonexistent"), |_| None, &installed);
        assert!(
            !results
                .iter()
                .any(|s| matches!(s, DoctorStatus::PeerMismatch { .. })),
            "satisfied peer must be clean: {results:?}"
        );
    }

    #[test]
    fn unsatisfied_peer_reports_the_peer_not_self() {
        let lock = LockFile {
            entries: vec![
                entry("@a/ext", vec![peer("@b/lib", "^2.0.0")]),
                entry("@b/lib", vec![]),
            ],
            ..Default::default()
        };
        let installed = std::collections::HashMap::from([
            ("@a/ext".to_string(), "1.0.0".to_string()),
            ("@b/lib".to_string(), "1.0.0".to_string()),
        ]);
        let results = run_doctor_check(&lock, Path::new("/nonexistent"), |_| None, &installed);
        let mismatches: Vec<&DoctorStatus> = results
            .iter()
            .filter(|s| matches!(s, DoctorStatus::PeerMismatch { .. }))
            .collect();
        assert_eq!(mismatches.len(), 1, "one mismatch: {results:?}");
        if let DoctorStatus::PeerMismatch {
            name,
            peer,
            required,
            installed,
        } = mismatches[0]
        {
            assert_eq!(name, "@a/ext");
            assert_eq!(peer, "@b/lib", "names the actual peer (not self)");
            assert_eq!(required, "^2.0.0");
            assert_eq!(installed.as_deref(), Some("1.0.0"));
        }
    }

    #[test]
    fn collect_peer_requirers_gathers_every_locked_entry_wanting_the_peer() {
        let lock = LockFile {
            entries: vec![
                entry("@a/ext", vec![peer("@shared/lib", "^1.0.0")]),
                entry("@b/ext", vec![peer("@shared/lib", "^2.0.0")]),
                entry("@shared/lib", vec![]),
            ],
            ..Default::default()
        };

        let requirers = collect_peer_requirers(&lock, "@shared/lib", None);
        assert_eq!(requirers.len(), 2);
        assert!(requirers.contains(&("@a/ext".to_string(), "^1.0.0".to_string())));
        assert!(requirers.contains(&("@b/ext".to_string(), "^2.0.0".to_string())));
    }

    #[test]
    fn collect_peer_requirers_includes_the_extra_in_flight_requirer() {
        let lock = LockFile {
            entries: vec![entry("@a/ext", vec![peer("@shared/lib", "^1.0.0")])],
            ..Default::default()
        };

        let requirers = collect_peer_requirers(&lock, "@shared/lib", Some(("@c/ext", "^3.0.0")));
        assert_eq!(requirers.len(), 2);
        assert!(requirers.contains(&("@c/ext".to_string(), "^3.0.0".to_string())));
    }

    #[test]
    fn missing_optional_peer_is_clean() {
        let mut peers = vec![peer("@b/lib", "^2.0.0")];
        peers[0].optional = true;
        let lock = LockFile {
            entries: vec![entry("@a/ext", peers)],
            ..Default::default()
        };
        let installed =
            std::collections::HashMap::from([("@a/ext".to_string(), "1.0.0".to_string())]);
        let results = run_doctor_check(&lock, Path::new("/nonexistent"), |_| None, &installed);
        assert!(
            !results
                .iter()
                .any(|s| matches!(s, DoctorStatus::PeerMismatch { .. })),
            "missing optional peer is fine: {results:?}"
        );
    }
}
