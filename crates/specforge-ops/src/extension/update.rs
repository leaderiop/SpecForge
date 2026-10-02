//! `specforge update`: move registry installs to newer published versions,
//! through the steps `add` uses (fetch, integrity, trust, the ADR-0001
//! diamond gate, the handshake's name and version check, install).
//!
//! An update is all or nothing (`update_all_extensions`): every newer
//! package is fetched and checked before anything is written, and if one
//! fails, nothing is applied. A failure while placing the binaries puts
//! the previous ones back; the lock is written once, last. (A publisher key
//! pinned while checking a signature stays pinned: it records trust, not
//! a change to the project.)

use super::add::{Checked, fetch_checked, place};
use super::{Origin, Trust, check_diamonds, extensions_dir, lock_path};
use crate::OpError;
use crate::registry::{NO_REGISTRY, Registry};
use specforge_wasm::{LockFile, installed_wasm_path, read_lock_file, write_lock_file};
use std::path::Path;

/// The code `update` reports when the project has no lock file.
pub const NO_LOCK: &str = "E033";

/// What to update, and how.
#[derive(Debug, Clone)]
pub struct UpdateRequest<'a> {
    pub root: &'a Path,
    /// One extension by name; every locked extension when `None`.
    pub name: Option<&'a str>,
    /// Allow a new major version (`--major`); otherwise an extension only
    /// moves within its locked version's caret range.
    pub major: bool,
    /// Accept a registry package with no publisher signature.
    pub allow_unsigned: bool,
    pub trust: Trust,
}

/// What became of one locked extension.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    /// Moved from `from` to `to` (only when the outcome is applied; see
    /// [`UpdateOutcome::applied`]).
    Updated {
        from: String,
        to: String,
        sha256: String,
        key_id: Option<String>,
    },
    /// Already at the newest version the request allows.
    UpToDate { version: String },
    /// Not a registry install (`source` is the lock's, e.g.
    /// `local:<path>`): a registry never replaces it (ADR 0004 D3-b).
    NotFromRegistry { source: String },
    /// Its newer version could not be fetched, checked or installed.
    Failed(OpError),
}

/// One locked extension and what became of it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionUpdate {
    pub name: String,
    pub status: UpdateStatus,
}

/// What an update did, extension by extension, in lock order.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOutcome {
    pub extensions: Vec<ExtensionUpdate>,
    /// Whether a registry was asked about anything (its configuration's
    /// diagnostics are then worth showing).
    pub registry_used: bool,
}

impl UpdateOutcome {
    /// Whether the updates were written: no extension failed. When one
    /// did, the [`UpdateStatus::Updated`] entries are what would have
    /// changed, and nothing did.
    pub fn applied(&self) -> bool {
        self.failures().next().is_none()
    }

    /// The extensions that moved (or, unapplied, would have).
    pub fn updated(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.extensions.iter().filter_map(|e| match &e.status {
            UpdateStatus::Updated { from, to, .. } => {
                Some((e.name.as_str(), from.as_str(), to.as_str()))
            }
            _ => None,
        })
    }

    /// The extensions that failed, with why.
    pub fn failures(&self) -> impl Iterator<Item = (&str, &OpError)> {
        self.extensions.iter().filter_map(|e| match &e.status {
            UpdateStatus::Failed(error) => Some((e.name.as_str(), error)),
            _ => None,
        })
    }
}

/// Update the extensions `req` names in the project at `req.root`.
///
/// Fails outright (nothing asked, nothing written) with E033 when the
/// project has no lock file, and with E063 when a registry install needs a
/// registry and none is configured.
pub fn update(req: &UpdateRequest, registry: &dyn Registry) -> Result<UpdateOutcome, OpError> {
    let lock_file = lock_path(req.root);
    let lock = read_lock_file(&lock_file)
        .map_err(|_| OpError::new(NO_LOCK, "no lock file found. Run `specforge add` first."))?;

    // Plan: resolve and check every newer package against the lock as it
    // will be, before anything is written.
    let mut staged = lock.clone();
    let mut planned: Vec<(String, Checked)> = Vec::new();
    let mut extensions = Vec::new();
    let mut registry_used = false;
    for entry in lock
        .entries
        .iter()
        .filter(|e| req.name.is_none_or(|n| e.name == n))
    {
        let status = if entry.source != "registry" {
            UpdateStatus::NotFromRegistry {
                source: entry.source.clone(),
            }
        } else {
            registry_used = true;
            match plan_one(req, registry, &staged, &entry.name, &entry.version) {
                Ok(None) => UpdateStatus::UpToDate {
                    version: entry.version.clone(),
                },
                Ok(Some(checked)) => {
                    let status = UpdateStatus::Updated {
                        from: entry.version.clone(),
                        to: checked.declared.version.clone(),
                        sha256: checked.package.sha256.clone(),
                        key_id: checked.key_id.clone(),
                    };
                    if let Some(staged_entry) =
                        staged.entries.iter_mut().find(|e| e.name == entry.name)
                    {
                        staged_entry.version = checked.declared.version.clone();
                        staged_entry.peer_dependencies = checked.declared.peers.clone();
                    }
                    planned.push((entry.name.clone(), checked));
                    status
                }
                Err(error) if error.code == NO_REGISTRY => return Err(error),
                Err(error) => UpdateStatus::Failed(error),
            }
        };
        extensions.push(ExtensionUpdate {
            name: entry.name.clone(),
            status,
        });
    }

    // An update must also leave the extensions that require it satisfied.
    for (dependent, peer) in broken_dependents(&staged, &planned, registry) {
        if let Some(e) = extensions.iter_mut().find(|e| e.name == peer.0) {
            e.status = UpdateStatus::Failed(OpError::new(
                peer.1.code.clone(),
                format!("updating {} breaks {dependent}: {}", peer.0, peer.1.message),
            ));
        }
    }

    let mut outcome = UpdateOutcome {
        extensions,
        registry_used,
    };
    if !outcome.applied() || planned.is_empty() {
        return Ok(outcome);
    }

    // Apply: place every binary, then write the lock once. Any failure
    // puts the previous binaries back and leaves the lock as it was.
    let mut lock = lock;
    let mut placed: Vec<(String, Option<Vec<u8>>)> = Vec::new();
    let origin = Origin::Installed {
        source: "registry".to_string(),
    };
    let mut failure = None;
    for (name, checked) in &planned {
        let previous = std::fs::read(installed_wasm_path(&extensions_dir(req.root), name)).ok();
        match place(
            req.root,
            &mut lock,
            &checked.declared,
            &checked.package.wasm,
            &checked.package.sha256,
            checked.key_id.as_deref(),
            &origin,
        ) {
            Ok(_) => placed.push((name.clone(), previous)),
            Err(error) => {
                failure = Some((name.clone(), error));
                break;
            }
        }
    }
    if failure.is_none()
        && let Err(diagnostic) = write_lock_file(&lock, &lock_file)
    {
        failure = Some((planned[0].0.clone(), OpError::from(diagnostic)));
    }
    if let Some((name, error)) = failure {
        restore(req.root, &placed);
        if let Some(e) = outcome.extensions.iter_mut().find(|e| e.name == name) {
            e.status = UpdateStatus::Failed(error);
        }
    }
    Ok(outcome)
}

/// The newer package `name` (locked at `current`) moves to, checked, or
/// `None` when it is up to date.
fn plan_one(
    req: &UpdateRequest,
    registry: &dyn Registry,
    staged: &LockFile,
    name: &str,
    current: &str,
) -> Result<Option<Checked>, OpError> {
    // Within the caret range of the locked version unless --major: a new
    // major version is a breaking change the user opts into.
    let range = match semver::Version::parse(current) {
        Ok(_) if !req.major => format!("^{current}"),
        _ => "*".to_string(),
    };
    let latest = registry.resolve_version(name, &range)?;
    if latest == current {
        return Ok(None);
    }
    // The package's own locked peers are the ones it replaces.
    let mut others = staged.clone();
    others.entries.retain(|e| e.name != name);
    fetch_checked(
        registry,
        &others,
        name,
        &latest,
        req.allow_unsigned,
        req.trust,
    )
    .map(Some)
}

/// Each locked extension a planned update leaves outside its declared peer
/// range: `(dependent, (updated peer, why))`.
fn broken_dependents(
    staged: &LockFile,
    planned: &[(String, Checked)],
    registry: &dyn Registry,
) -> Vec<(String, (String, OpError))> {
    let mut broken = Vec::new();
    for entry in &staged.entries {
        if planned.iter().any(|(name, _)| *name == entry.name) {
            continue;
        }
        for peer in &entry.peer_dependencies {
            if !planned.iter().any(|(name, _)| *name == peer.name) {
                continue;
            }
            if let Err(error) =
                check_diamonds(staged, &entry.name, std::slice::from_ref(peer), &|p| {
                    registry.versions(p)
                })
            {
                broken.push((entry.name.clone(), (peer.name.clone(), error)));
            }
        }
    }
    broken
}

/// Put back the binaries an aborted update replaced.
fn restore(root: &Path, placed: &[(String, Option<Vec<u8>>)]) {
    for (name, previous) in placed {
        let path = installed_wasm_path(&extensions_dir(root), name);
        match previous {
            Some(bytes) => {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = std::fs::write(&path, bytes);
            }
            // It had no binary before: take the new one away.
            None => {
                let _ = std::fs::remove_dir_all(extensions_dir(root).join(name));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Package;
    use specforge_registry::PeerDependency;
    use specforge_registry::registry_client::RegistryResponse;
    use specforge_test_macros::test as specforge_test;
    use specforge_wasm::{LockFileEntry, hex_sha256};
    use std::cell::RefCell;

    /// `@sdk/greet` 0.1.0, a real extension binary.
    fn greet() -> Vec<u8> {
        std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm"),
        )
        .expect("the greet fixture is vendored")
    }

    /// An in-memory registry: each package's versions, and the bytes it
    /// serves for one of them (unsigned).
    struct FakeRegistry {
        published: Vec<(&'static str, Vec<&'static str>)>,
        served: Vec<(&'static str, &'static str, Vec<u8>)>,
        ranges: RefCell<Vec<String>>,
    }

    impl FakeRegistry {
        fn new() -> Self {
            Self {
                published: Vec::new(),
                served: Vec::new(),
                ranges: RefCell::new(Vec::new()),
            }
        }
        fn publish(mut self, name: &'static str, versions: &[&'static str]) -> Self {
            self.published.push((name, versions.to_vec()));
            self
        }
        fn serve(mut self, name: &'static str, version: &'static str, wasm: Vec<u8>) -> Self {
            self.served.push((name, version, wasm));
            self
        }
    }

    impl Registry for FakeRegistry {
        fn resolve_version(&self, name: &str, range: &str) -> Result<String, OpError> {
            self.ranges.borrow_mut().push(range.to_string());
            let req = match range {
                "*" | "latest" => semver::VersionReq::STAR,
                r => semver::VersionReq::parse(r).unwrap(),
            };
            self.versions(name)?
                .iter()
                .filter_map(|v| semver::Version::parse(v).ok())
                .filter(|v| req.matches(v))
                .max()
                .map(|v| v.to_string())
                .ok_or_else(|| OpError::new("R-RES-004", format!("no {name} matches {range}")))
        }

        fn fetch(&self, name: &str, version: &str) -> Result<Package, OpError> {
            let (_, _, wasm) = self
                .served
                .iter()
                .find(|(n, v, _)| *n == name && *v == version)
                .ok_or_else(|| OpError::new("R-RES-001", format!("{name}@{version} not served")))?;
            let sha256 = hex_sha256(wasm);
            Ok(Package {
                name: name.to_string(),
                version: version.to_string(),
                wasm: wasm.clone(),
                sha256: sha256.clone(),
                peers: Vec::new(),
                response: RegistryResponse {
                    name: name.to_string(),
                    version: version.to_string(),
                    wasm_url: String::new(),
                    sha256,
                    signature: String::new(),
                    key_id: String::new(),
                    manifest: String::new(),
                },
            })
        }

        fn versions(&self, name: &str) -> Result<Vec<String>, OpError> {
            Ok(self
                .published
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.iter().map(|v| v.to_string()).collect())
                .unwrap_or_default())
        }
    }

    fn entry(name: &str, version: &str, source: &str, peers: &[(&str, &str)]) -> LockFileEntry {
        LockFileEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: source.to_string(),
            wasm_hash: hex_sha256(b"old"),
            key_id: None,
            peer_dependencies: peers
                .iter()
                .map(|(name, range)| PeerDependency {
                    name: name.to_string(),
                    version: range.to_string(),
                    optional: false,
                })
                .collect(),
        }
    }

    /// A project locking `entries`, each installed with the bytes `old`.
    fn project(entries: Vec<LockFileEntry>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for e in &entries {
            let path = installed_wasm_path(&extensions_dir(dir.path()), &e.name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"old").unwrap();
        }
        let lock = LockFile {
            lockfile_version: 1,
            entries,
        };
        write_lock_file(&lock, &lock_path(dir.path())).unwrap();
        dir
    }

    fn request(root: &Path, major: bool) -> UpdateRequest<'_> {
        UpdateRequest {
            root,
            name: None,
            major,
            allow_unsigned: true,
            trust: Trust::Refuse,
        }
    }

    fn status_of<'a>(outcome: &'a UpdateOutcome, name: &str) -> &'a UpdateStatus {
        &outcome
            .extensions
            .iter()
            .find(|e| e.name == name)
            .unwrap()
            .status
    }

    #[specforge_test(
        behavior = "update_all_extensions",
        verify = "lock file records new binary hashes after update"
    )]
    fn an_update_installs_the_newer_binary_and_locks_its_hash() {
        let dir = project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])]);
        let registry = FakeRegistry::new()
            .publish("@sdk/greet", &["0.0.9", "0.1.0"])
            .serve("@sdk/greet", "0.1.0", greet());

        let outcome = update(&request(dir.path(), true), &registry).unwrap();

        assert!(outcome.applied(), "{outcome:?}");
        assert_eq!(
            outcome.updated().collect::<Vec<_>>(),
            [("@sdk/greet", "0.0.9", "0.1.0")]
        );
        let lock = read_lock_file(&lock_path(dir.path())).unwrap();
        assert_eq!(lock.entries[0].version, "0.1.0");
        assert_eq!(lock.entries[0].wasm_hash, hex_sha256(&greet()));
        assert_eq!(lock.entries[0].source, "registry");
        let installed = installed_wasm_path(&extensions_dir(dir.path()), "@sdk/greet");
        assert_eq!(std::fs::read(installed).unwrap(), greet());
    }

    #[specforge_test(
        behavior = "update_all_extensions",
        verify = "major version skipped without --major flag"
    )]
    fn a_new_major_version_needs_major() {
        // ^0.0.9 admits only 0.0.9: 0.1.0 is a breaking change.
        let dir = project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])]);
        let registry = FakeRegistry::new()
            .publish("@sdk/greet", &["0.0.9", "0.1.0"])
            .serve("@sdk/greet", "0.1.0", greet());

        let outcome = update(&request(dir.path(), false), &registry).unwrap();

        assert_eq!(registry.ranges.borrow().as_slice(), ["^0.0.9"]);
        assert_eq!(
            status_of(&outcome, "@sdk/greet"),
            &UpdateStatus::UpToDate {
                version: "0.0.9".into()
            }
        );
        assert_eq!(
            read_lock_file(&lock_path(dir.path())).unwrap().entries[0].version,
            "0.0.9"
        );
    }

    #[specforge_test(
        behavior = "update_all_extensions",
        verify = "failed upgrade rolls back all changes"
    )]
    fn one_failed_upgrade_applies_none() {
        let dir = project(vec![
            entry("@sdk/greet", "0.0.9", "registry", &[]),
            entry("@acme/liar", "1.0.0", "registry", &[]),
        ]);
        let lock_before = std::fs::read(lock_path(dir.path())).unwrap();
        // @acme/liar 1.1.0 serves greet's binary: E028, as add refuses it.
        let registry = FakeRegistry::new()
            .publish("@sdk/greet", &["0.1.0"])
            .serve("@sdk/greet", "0.1.0", greet())
            .publish("@acme/liar", &["1.1.0"])
            .serve("@acme/liar", "1.1.0", greet());

        let outcome = update(&request(dir.path(), true), &registry).unwrap();

        assert!(!outcome.applied());
        let failures: Vec<(&str, &str)> = outcome
            .failures()
            .map(|(n, e)| (n, e.code.as_ref()))
            .collect();
        assert_eq!(failures, [("@acme/liar", "E028")]);
        assert_eq!(std::fs::read(lock_path(dir.path())).unwrap(), lock_before);
        let installed = installed_wasm_path(&extensions_dir(dir.path()), "@sdk/greet");
        assert_eq!(std::fs::read(installed).unwrap(), b"old");
    }

    #[specforge_test(
        behavior = "update_all_extensions",
        verify = "peer dependency conflicts detected before applying"
    )]
    fn an_update_that_breaks_a_dependent_is_refused() {
        let dir = project(vec![
            entry("@sdk/greet", "0.0.9", "registry", &[]),
            entry(
                "@acme/user",
                "1.0.0",
                "local:user.wasm",
                &[("@sdk/greet", "^0.0.9")],
            ),
        ]);
        let lock_before = std::fs::read(lock_path(dir.path())).unwrap();
        let registry = FakeRegistry::new().publish("@sdk/greet", &["0.1.0"]).serve(
            "@sdk/greet",
            "0.1.0",
            greet(),
        );

        let outcome = update(&request(dir.path(), true), &registry).unwrap();

        assert!(!outcome.applied());
        let (name, error) = outcome.failures().next().unwrap();
        assert_eq!(name, "@sdk/greet");
        assert!(error.message.contains("breaks @acme/user"), "{error:?}");
        assert_eq!(std::fs::read(lock_path(dir.path())).unwrap(), lock_before);
    }

    #[test]
    fn a_local_install_is_never_asked_about() {
        let dir = project(vec![entry("@sdk/greet", "0.0.9", "local:greet.wasm", &[])]);
        let registry = FakeRegistry::new().publish("@sdk/greet", &["9.9.9"]);

        let outcome = update(&request(dir.path(), true), &registry).unwrap();

        assert!(registry.ranges.borrow().is_empty());
        assert!(!outcome.registry_used);
        assert_eq!(
            status_of(&outcome, "@sdk/greet"),
            &UpdateStatus::NotFromRegistry {
                source: "local:greet.wasm".into()
            }
        );
    }

    #[test]
    fn no_lock_and_no_registry_fail_outright() {
        let empty = tempfile::tempdir().unwrap();
        let error = update(&request(empty.path(), false), &FakeRegistry::new()).unwrap_err();
        assert_eq!(error.code, NO_LOCK);

        let dir = project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])]);
        let unconfigured = crate::registry::HttpRegistry::for_project(dir.path(), "update");
        let error = update(&request(dir.path(), false), &unconfigured).unwrap_err();
        assert_eq!(error.code, NO_REGISTRY);
    }
}
