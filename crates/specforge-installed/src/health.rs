use specforge_protocol_types::PackageName;

use crate::Installed;
use crate::lock::LockFileEntry;
use crate::module::hex_sha256;

/// One problem with one lock entry (doctor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    /// The entry's module is not on disk.
    MissingModule { name: String },
    /// The entry's module is not the one the lock pins.
    Changed {
        name: String,
        locked: String,
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

impl Installed {
    /// Every lock entry checked: its module present and the pinned one,
    /// its peers installed at versions their ranges accept. Healthy
    /// entries are absent.
    pub fn health(&self) -> Vec<Health> {
        let mut problems = Vec::new();
        let entries = self.lock().entries();

        for entry in entries {
            if let Some(problem) = self.module_health(entry) {
                problems.push(problem);
            }
        }

        // C8-05: real peer-dependency verification. Each entry's recorded
        // peers must be present among the installed versions and satisfy
        // the declared semver requirement. Optional peers that are absent
        // are fine.
        for entry in entries {
            for peer in &entry.peer_dependencies {
                let installed = entries
                    .iter()
                    .find(|e| e.name == peer.name)
                    .map(|e| &e.version);
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
                    problems.push(Health::PeerMismatch {
                        name: entry.name.clone(),
                        peer: peer.name.clone(),
                        required: peer.version.clone(),
                        installed: installed.cloned(),
                    });
                }
            }
        }

        problems
    }

    /// What is wrong with `entry`'s module, if anything.
    fn module_health(&self, entry: &LockFileEntry) -> Option<Health> {
        let missing = || Health::MissingModule {
            name: entry.name.clone(),
        };
        // A lock entry that names no package has no module to find.
        let Ok(name) = PackageName::parse(&entry.name) else {
            return Some(missing());
        };
        let path = self.module_path(&name);
        if !path.exists() {
            return Some(missing());
        }
        // A module that cannot be read is not judged here: loading it says why.
        let actual = hex_sha256(&std::fs::read(&path).ok()?);
        (actual != entry.wasm_hash).then(|| Health::Changed {
            name: entry.name.clone(),
            locked: entry.wasm_hash.clone(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::{LockFile, LockState};
    use specforge_protocol_types::PeerDependency;
    use tempfile::TempDir;

    fn entry(name: &str, hash: &str, peers: Vec<PeerDependency>) -> LockFileEntry {
        LockFileEntry {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            source: "registry".to_string(),
            wasm_hash: hash.to_string(),
            key_id: None,
            peer_dependencies: peers,
        }
    }

    fn peer(name: &str, req: &str) -> PeerDependency {
        PeerDependency {
            name: name.to_string(),
            version: req.to_string(),
            optional: false,
        }
    }

    fn installed(dir: &TempDir, entries: Vec<LockFileEntry>) -> Installed {
        Installed::with_lock(
            dir.path(),
            LockState::Read(LockFile {
                entries,
                ..Default::default()
            }),
        )
    }

    /// `name`'s module on disk under `dir`, holding `bytes`.
    fn place(installed: &Installed, name: &str, bytes: &[u8]) {
        let path = installed.module_path(&PackageName::parse(name).unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    // B:run_doctor_check — verify unit "detects missing binary"
    #[test]
    fn test_doctor_detects_missing_binary() {
        let dir = TempDir::new().unwrap();
        let installed = installed(&dir, vec![entry("missing-ext", "abc", vec![])]);

        assert_eq!(
            installed.health(),
            [Health::MissingModule {
                name: "missing-ext".to_string()
            }]
        );
    }

    // B:run_doctor_check — verify unit "detects stale hash"
    #[test]
    fn test_doctor_detects_stale_hash() {
        let dir = TempDir::new().unwrap();
        let installed = installed(&dir, vec![entry("my-ext", "expected_hash", vec![])]);
        place(&installed, "my-ext", b"wasm content");

        let problems = installed.health();

        assert_eq!(
            problems,
            [Health::Changed {
                name: "my-ext".to_string(),
                locked: "expected_hash".to_string(),
                actual: hex_sha256(b"wasm content"),
            }]
        );
    }

    // B:run_doctor_check — verify unit "reports healthy when all checks pass"
    #[test]
    fn test_doctor_reports_healthy() {
        let dir = TempDir::new().unwrap();
        let installed = installed(&dir, vec![entry("good-ext", &hex_sha256(b"wasm"), vec![])]);
        place(&installed, "good-ext", b"wasm");

        assert_eq!(installed.health(), []);
    }

    #[test]
    fn a_lock_entry_that_names_no_package_has_no_module() {
        let dir = TempDir::new().unwrap();
        let installed = installed(&dir, vec![entry("../../outside", "abc", vec![])]);

        assert_eq!(
            installed.health(),
            [Health::MissingModule {
                name: "../../outside".to_string()
            }]
        );
    }

    // C8-05 acceptance: doctor verifies recorded peers across OTHER entries.

    fn peers_only(entries: Vec<LockFileEntry>) -> Vec<Health> {
        let dir = TempDir::new().unwrap();
        let installed = installed(&dir, entries);
        installed
            .health()
            .into_iter()
            .filter(|h| matches!(h, Health::PeerMismatch { .. }))
            .collect()
    }

    #[test]
    fn satisfied_peer_is_clean() {
        let problems = peers_only(vec![
            entry("@a/ext", "hash", vec![peer("@b/lib", "^2.0.0")]),
            LockFileEntry {
                version: "2.1.0".to_string(),
                ..entry("@b/lib", "hash", vec![])
            },
        ]);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn unsatisfied_peer_reports_the_peer_not_self() {
        let problems = peers_only(vec![
            entry("@a/ext", "hash", vec![peer("@b/lib", "^2.0.0")]),
            entry("@b/lib", "hash", vec![]),
        ]);
        assert_eq!(
            problems,
            [Health::PeerMismatch {
                name: "@a/ext".to_string(),
                peer: "@b/lib".to_string(),
                required: "^2.0.0".to_string(),
                installed: Some("1.0.0".to_string()),
            }]
        );
    }

    #[test]
    fn missing_optional_peer_is_clean() {
        let mut peers = vec![peer("@b/lib", "^2.0.0")];
        peers[0].optional = true;
        let problems = peers_only(vec![entry("@a/ext", "hash", peers)]);
        assert!(problems.is_empty(), "{problems:?}");
    }
}
