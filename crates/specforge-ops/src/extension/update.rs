//! `specforge update`: move registry installs to newer published versions,
//! through the steps `add` uses (fetch, integrity, trust, the ADR-0001
//! diamond gate, the handshake's name and version check, install).
//!
//! An update is all or nothing (`update_all_extensions`): every newer
//! package is fetched and checked before anything is written, and if one
//! fails, nothing is applied. The binaries and the lock are one change
//! (`specforge_installed::Change`): a failure puts the previous ones back.
//! (A publisher key
//! pinned while checking a signature stays pinned: it records trust, not
//! a change to the project.)

use super::add::{Checked, fetch_checked};
use super::diamond::broken_requirers;
use super::{Trust, published_versions};
use crate::registry::{NO_REGISTRY, Registry};
use crate::{OpError, OpErrorKind, Writes};
use specforge_common::{Code, codes};
use specforge_installed::{Installed, LockFile, LockSource, LockState, Pin};
use specforge_protocol_types::PackageName;
use specforge_protocol_types::package::VersionRequirement;
use specforge_wasm::WasmRuntime;
use std::path::Path;

/// The code `update` reports when the project has no lock file.
pub const NO_LOCK: Code = codes::E033;

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

    /// The `batch_update_completed` event's counts
    /// (`spec/events/wasm-extensions.spec`). An extension counts as
    /// updated only when the update was applied; one an aborted update
    /// would have moved counts as skipped, since nothing changed. The
    /// adapter that emits the event stamps its `timestamp`.
    pub fn batch_update_completed(&self) -> BatchUpdateCompleted {
        let failed_count = self.failures().count();
        let updated_count = if failed_count == 0 {
            self.updated().count()
        } else {
            0
        };
        BatchUpdateCompleted {
            updated_count,
            failed_count,
            skipped_count: self.extensions.len() - updated_count - failed_count,
        }
    }
}

/// The payload of `batch_update_completed`, which an update that ran to
/// the end (applied or rolled back) produces, its timestamp aside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchUpdateCompleted {
    /// Extensions moved to a newer version.
    pub updated_count: usize,
    /// Extensions whose newer version could not be fetched, checked or
    /// installed.
    pub failed_count: usize,
    /// Extensions left as they were: up to date, not from a registry, or
    /// held back because another failed.
    pub skipped_count: usize,
}

/// Update the extensions `req` names in the project at `req.root`.
///
/// Fails outright (nothing asked, nothing written) with `config_invalid`
/// when `specforge.json` cannot be used, with E033 when the
/// project has no lock file, and with E063 when a registry install needs a
/// registry and none is configured.
pub fn update(
    req: &UpdateRequest,
    registry: &dyn Registry,
    runtime: &dyn WasmRuntime,
) -> Result<UpdateOutcome, OpError> {
    // A project whose specforge.json cannot be used is refused before
    // anything is read or written, as `add` and `remove` refuse it.
    crate::config::usable(req.root)?;
    let installed = Installed::at(req.root);
    let lock = match installed.lock() {
        LockState::Read(lock) => lock.clone(),
        LockState::Absent => {
            return Err(OpError::coded(
                OpErrorKind::PreconditionFailed,
                NO_LOCK,
                "no lock file found. Run `specforge add` first.",
            ));
        }
        LockState::Unreadable(problem) => {
            return Err(OpError::coded(
                OpErrorKind::PreconditionFailed,
                NO_LOCK,
                format!("{}. Run `specforge add` first.", problem.message),
            ));
        }
    };

    // Plan: resolve and check every newer package against the lock as it
    // will be, before anything is written.
    let mut staged = lock.clone();
    let mut planned: Vec<(String, Checked)> = Vec::new();
    let mut extensions = Vec::new();
    let mut registry_used = false;
    for entry in lock
        .entries
        .iter()
        .filter(|e| req.name.is_none_or(|n| e.name.as_str() == n))
    {
        let status = if !entry.source.is_registry() {
            UpdateStatus::NotFromRegistry {
                source: entry.source.to_string(),
            }
        } else {
            registry_used = true;
            match plan_one(req, registry, runtime, &staged, &entry.name, &entry.version) {
                Ok(None) => UpdateStatus::UpToDate {
                    version: entry.version.clone(),
                },
                Ok(Some(checked)) => {
                    let status = UpdateStatus::Updated {
                        from: entry.version.clone(),
                        to: checked.binary.candidate().version().to_string(),
                        sha256: checked.package.sha256.clone(),
                        key_id: checked.package.key_id.clone(),
                    };
                    if let Some(staged_entry) =
                        staged.entries.iter_mut().find(|e| e.name == entry.name)
                    {
                        staged_entry.version = checked.binary.candidate().version().to_string();
                        staged_entry.peer_dependencies =
                            checked.binary.candidate().peers().to_vec();
                    }
                    planned.push((entry.name.to_string(), checked));
                    status
                }
                Err(error) if error.is(NO_REGISTRY) => return Err(error),
                Err(error) => UpdateStatus::Failed(error),
            }
        };
        extensions.push(ExtensionUpdate {
            name: entry.name.to_string(),
            status,
        });
    }

    // An update must also leave the extensions that require it satisfied.
    for (dependent, peer) in broken_dependents(&staged, &planned, registry) {
        if let Some(e) = extensions.iter_mut().find(|e| e.name == peer.0) {
            let failed = OpError::new(
                peer.1.kind,
                peer.1.code.clone(),
                format!("updating {} breaks {dependent}: {}", peer.0, peer.1.message),
            );
            e.status = UpdateStatus::Failed(failed);
        }
    }

    let mut outcome = UpdateOutcome {
        extensions,
        registry_used,
    };
    if !outcome.applied() || planned.is_empty() {
        return Ok(outcome);
    }

    // Apply: one change that places every binary and writes the lock once.
    // A failure puts the previous binaries and lock back.
    let mut change = installed.change().map_err(OpError::from)?;
    for (_, checked) in &planned {
        change.install(
            checked.binary.module().clone(),
            Pin {
                name: checked.package.name.clone(),
                version: checked.binary.candidate().version().to_string(),
                source: LockSource::Registry,
                key_id: checked.package.key_id.clone(),
                peers: checked.binary.candidate().peers().to_vec(),
            },
        );
    }
    if let Err(failed) = change.commit() {
        let error = OpError::from(failed.error).with_writes(Writes::of(failed.left));
        if let Some(e) = outcome
            .extensions
            .iter_mut()
            .find(|e| e.name == planned[0].0)
        {
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
    runtime: &dyn WasmRuntime,
    staged: &LockFile,
    package: &PackageName,
    current: &str,
) -> Result<Option<Checked>, OpError> {
    // Within the caret range of the locked version unless --major: a new
    // major version is a breaking change the user opts into.
    let requirement = match semver::Version::parse(current) {
        Ok(current) if !req.major => VersionRequirement::compatible_with(&current),
        _ => VersionRequirement::Latest,
    };
    let latest = super::resolve_requirement(registry, package, &requirement)?;
    if latest.to_string() == current {
        return Ok(None);
    }
    // The package's own locked peers are the ones it replaces.
    let mut others = staged.clone();
    others.entries.retain(|e| e.name != *package);
    fetch_checked(
        registry,
        runtime,
        &others,
        package,
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
    let names: Vec<&str> = planned.iter().map(|(name, _)| name.as_str()).collect();
    let published = published_versions(registry);
    let mut broken: Vec<(String, (String, OpError))> = names
        .iter()
        .flat_map(|name| {
            broken_requirers(staged, name, &names, Some(&published))
                .into_iter()
                .map(move |(dependent, error)| (dependent, (name.to_string(), error)))
        })
        .collect();
    // In lock order, as the dependents are reported.
    broken.sort_by_key(|(dependent, _)| {
        staged
            .entries
            .iter()
            .position(|e| e.name.as_str() == dependent)
    });
    broken
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Package;
    use specforge_installed::{LockFileEntry, hex_sha256, lock_path, write_lock_file};
    use specforge_protocol_types::package::Version;
    use specforge_protocol_types::{ExtensionDeclaration, PackageName};
    use specforge_registry::PeerDependency;
    use specforge_test_macros::test as specforge_test;
    use std::cell::RefCell;

    /// Where the module of extension `name` is installed under `root`.
    fn installed(root: &Path, name: &str) -> std::path::PathBuf {
        Installed::unread(root).module_path(&PackageName::parse(name).unwrap())
    }

    fn runtime() -> specforge_component::ComponentRuntime {
        specforge_component::ComponentRuntime::new()
    }

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
        /// The declaration published with a package, when it isn't the one
        /// its binary declares.
        declared: Vec<(&'static str, &'static str, ExtensionDeclaration)>,
        /// The packages whose versions were listed, in order.
        listed: RefCell<Vec<String>>,
    }

    /// What `wasm` declares, as `publish` would upload it.
    fn declaration_of(wasm: &[u8]) -> ExtensionDeclaration {
        crate::publish::prepare(&runtime(), wasm.to_vec())
            .expect("a publishable binary")
            .declaration
    }

    impl FakeRegistry {
        fn new() -> Self {
            Self {
                published: Vec::new(),
                served: Vec::new(),
                declared: Vec::new(),
                listed: RefCell::new(Vec::new()),
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
        /// Publish `name@version` with `declaration` as its manifest
        /// instead of what its binary declares.
        fn declare(
            mut self,
            name: &'static str,
            version: &'static str,
            declaration: ExtensionDeclaration,
        ) -> Self {
            self.declared.push((name, version, declaration));
            self
        }
    }

    impl Registry for FakeRegistry {
        /// Serves unsigned packages; the tests allow them.
        fn fetch(
            &self,
            name: &PackageName,
            version: &Version,
            _allow_unsigned: bool,
            _trust: Trust,
        ) -> Result<Package, OpError> {
            let (name, version) = (name.as_str(), version.to_string());
            let (_, _, wasm) = self
                .served
                .iter()
                .find(|(n, v, _)| *n == name && *v == version)
                .ok_or_else(|| {
                    OpError::diagnostic(codes::R_RES_001, format!("{name}@{version} not served"))
                })?;
            let declaration = self
                .declared
                .iter()
                .find(|(n, v, _)| *n == name && *v == version)
                .map(|(_, _, d)| d.clone())
                .unwrap_or_else(|| declaration_of(wasm));
            Ok(Package {
                name: PackageName::parse(name).unwrap(),
                version: Version::parse(&version).unwrap(),
                wasm: wasm.clone(),
                sha256: hex_sha256(wasm),
                declaration,
                key_id: None,
            })
        }

        fn versions(&self, name: &PackageName) -> Result<Vec<Version>, OpError> {
            self.listed.borrow_mut().push(name.to_string());
            Ok(self
                .published
                .iter()
                .find(|(n, _)| *n == name.as_str())
                .map(|(_, v)| v.iter().map(|v| Version::parse(v).unwrap()).collect())
                .unwrap_or_default())
        }
    }

    fn entry(name: &str, version: &str, source: &str, peers: &[(&str, &str)]) -> LockFileEntry {
        LockFileEntry {
            name: specforge_protocol_types::PackageName::parse(name).unwrap(),
            version: version.to_string(),
            source: LockSource::parse(source),
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
            let path = installed(dir.path(), e.name.as_str());
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

        let outcome = update(&request(dir.path(), true), &registry, &runtime()).unwrap();

        assert!(outcome.applied(), "{outcome:?}");
        assert_eq!(
            outcome.updated().collect::<Vec<_>>(),
            [("@sdk/greet", "0.0.9", "0.1.0")]
        );
        assert_eq!(
            outcome.batch_update_completed(),
            BatchUpdateCompleted {
                updated_count: 1,
                failed_count: 0,
                skipped_count: 0,
            }
        );
        let lock = specforge_installed::read_lock_file(&lock_path(dir.path())).unwrap();
        assert_eq!(lock.entries[0].version, "0.1.0");
        assert_eq!(lock.entries[0].wasm_hash, hex_sha256(&greet()));
        assert_eq!(lock.entries[0].source, LockSource::Registry);
        let installed = installed(dir.path(), "@sdk/greet");
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

        let outcome = update(&request(dir.path(), false), &registry, &runtime()).unwrap();

        // Asked for what ^0.0.9 admits: 0.1.0 is not it.
        assert_eq!(registry.listed.borrow().as_slice(), ["@sdk/greet"]);
        assert_eq!(
            status_of(&outcome, "@sdk/greet"),
            &UpdateStatus::UpToDate {
                version: "0.0.9".into()
            }
        );
        assert_eq!(
            specforge_installed::read_lock_file(&lock_path(dir.path()))
                .unwrap()
                .entries[0]
                .version,
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

        let outcome = update(&request(dir.path(), true), &registry, &runtime()).unwrap();

        assert!(!outcome.applied());
        let failures: Vec<(&str, &str)> = outcome
            .failures()
            .map(|(n, e)| (n, e.code.as_ref()))
            .collect();
        assert_eq!(failures, [("@acme/liar", "E028")]);
        // greet would have moved but did not: it is skipped, not updated.
        assert_eq!(
            outcome.batch_update_completed(),
            BatchUpdateCompleted {
                updated_count: 0,
                failed_count: 1,
                skipped_count: 1,
            }
        );
        assert_eq!(std::fs::read(lock_path(dir.path())).unwrap(), lock_before);
        let installed = installed(dir.path(), "@sdk/greet");
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

        let outcome = update(&request(dir.path(), true), &registry, &runtime()).unwrap();

        assert!(!outcome.applied());
        let (name, error) = outcome.failures().next().unwrap();
        assert_eq!(name, "@sdk/greet");
        assert!(error.message.contains("breaks @acme/user"), "{error:?}");
        assert_eq!(std::fs::read(lock_path(dir.path())).unwrap(), lock_before);
    }

    #[cfg(unix)]
    #[specforge_test(
        behavior = "update_all_extensions",
        verify = "failed upgrade rolls back all changes"
    )]
    fn a_lock_that_cannot_be_written_puts_the_old_binaries_back() {
        use std::os::unix::fs::PermissionsExt;
        let dir = project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])]);
        let lock_before = std::fs::read(lock_path(dir.path())).unwrap();
        let registry = FakeRegistry::new().publish("@sdk/greet", &["0.1.0"]).serve(
            "@sdk/greet",
            "0.1.0",
            greet(),
        );
        // The binaries can be placed, the lock beside them cannot be written.
        let mode = |m| std::fs::Permissions::from_mode(m);
        std::fs::set_permissions(dir.path(), mode(0o555)).unwrap();
        if std::fs::write(dir.path().join("probe"), b"").is_ok() {
            // Running as a user permissions don't bind (root): nothing to test.
            std::fs::set_permissions(dir.path(), mode(0o755)).unwrap();
            return;
        }

        let outcome = update(&request(dir.path(), true), &registry, &runtime()).unwrap();
        std::fs::set_permissions(dir.path(), mode(0o755)).unwrap();

        assert!(!outcome.applied(), "{outcome:?}");
        let (_, error) = outcome.failures().next().unwrap();
        assert_eq!(error.code, "E033", "{error:?}");
        assert_eq!(std::fs::read(lock_path(dir.path())).unwrap(), lock_before);
        let installed = installed(dir.path(), "@sdk/greet");
        assert_eq!(std::fs::read(installed).unwrap(), b"old");
    }

    #[test]
    fn a_local_install_is_never_asked_about() {
        let dir = project(vec![entry("@sdk/greet", "0.0.9", "local:greet.wasm", &[])]);
        let registry = FakeRegistry::new().publish("@sdk/greet", &["9.9.9"]);

        let outcome = update(&request(dir.path(), true), &registry, &runtime()).unwrap();

        assert!(registry.listed.borrow().is_empty());
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
        let error = update(
            &request(empty.path(), false),
            &FakeRegistry::new(),
            &runtime(),
        )
        .unwrap_err();
        assert!(error.is(NO_LOCK), "{error:?}");

        let dir = project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])]);
        let unconfigured = crate::registry::Unconfigured("update");
        let error = update(&request(dir.path(), false), &unconfigured, &runtime()).unwrap_err();
        assert!(error.is(NO_REGISTRY), "{error:?}");
    }

    /// A project enabling nothing yet, with `lock` locked.
    fn add_project(lock: Vec<LockFileEntry>) -> tempfile::TempDir {
        let dir = project(lock);
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name": "p", "version": "0.1.0", "extensions": []}"#,
        )
        .unwrap();
        dir
    }

    fn add_greet(
        root: &Path,
        registry: &FakeRegistry,
    ) -> Result<super::super::AddOutcome, OpError> {
        super::super::add(
            &super::super::AddRequest {
                root,
                source: super::super::Source::Registry(
                    specforge_protocol_types::PackageRef::parse("@sdk/greet@0.1.0").unwrap(),
                ),
                allow_unsigned: true,
                trust: Trust::Refuse,
                dry_run: false,
            },
            registry,
            &runtime(),
        )
        .map(|added| added.outcome)
    }

    fn with_peer(
        mut declaration: ExtensionDeclaration,
        name: &str,
        range: &str,
    ) -> ExtensionDeclaration {
        declaration
            .handshake
            .peer_dependencies
            .push(PeerDependency {
                name: name.to_string(),
                version: range.to_string(),
                optional: false,
            });
        declaration
    }

    #[specforge_test(
        behavior = "check_registry_reply",
        verify = "add refuses a package whose binary declares other than its published declaration"
    )]
    fn a_binary_that_declares_other_than_its_published_declaration_is_refused() {
        // R4/R5: the published declaration differs from the binary's, in
        // its handshake (another description) or in a category (no kinds).
        // A served declaration is trusted for nothing the binary doesn't
        // declare: the package is refused before anything is installed.
        let mut description = declaration_of(&greet());
        description.handshake.description = Some("Something else".to_string());
        for published in [description, {
            let mut kinds = declaration_of(&greet());
            kinds.entities.clear();
            kinds
        }] {
            let dir = add_project(Vec::new());
            let registry = FakeRegistry::new()
                .publish("@sdk/greet", &["0.1.0"])
                .serve("@sdk/greet", "0.1.0", greet())
                .declare("@sdk/greet", "0.1.0", published);
            let err = add_greet(dir.path(), &registry).unwrap_err();
            assert!(err.is(crate::registry::METADATA_MISMATCH), "{err:?}");
            assert!(err.message.contains("@sdk/greet@0.1.0"), "{err:?}");
            assert!(
                !installed(dir.path(), "@sdk/greet").exists(),
                "nothing is installed"
            );
        }
        // The message names the first part that differs.
        let mut kinds = declaration_of(&greet());
        kinds.entities.clear();
        let dir = add_project(Vec::new());
        let registry = FakeRegistry::new()
            .publish("@sdk/greet", &["0.1.0"])
            .serve("@sdk/greet", "0.1.0", greet())
            .declare("@sdk/greet", "0.1.0", kinds);
        let err = add_greet(dir.path(), &registry).unwrap_err();
        assert!(err.message.contains("another entities"), "{err:?}");
        // The published declaration equal to the binary's installs.
        let dir = add_project(Vec::new());
        let registry = FakeRegistry::new().publish("@sdk/greet", &["0.1.0"]).serve(
            "@sdk/greet",
            "0.1.0",
            greet(),
        );
        add_greet(dir.path(), &registry).unwrap();
    }

    #[specforge_test(
        behavior = "check_registry_reply",
        verify = "the diamond gate decides on the published declaration's peers"
    )]
    fn the_diamond_gate_reads_the_published_declarations_peers() {
        // R4: the published declaration names a peer @acme/x ^2 that the
        // lock holds at 1.0.0. The gate refuses before the binary (which
        // declares no peer) is loaded.
        let dir = add_project(vec![entry("@acme/x", "1.0.0", "registry", &[])]);
        let registry = FakeRegistry::new()
            .publish("@sdk/greet", &["0.1.0"])
            .publish("@acme/x", &["1.0.0"])
            .serve("@sdk/greet", "0.1.0", greet())
            .declare(
                "@sdk/greet",
                "0.1.0",
                with_peer(declaration_of(&greet()), "@acme/x", "^2"),
            );
        let err = add_greet(dir.path(), &registry).unwrap_err();
        assert!(!err.is(crate::registry::METADATA_MISMATCH), "{err:?}");
        assert!(err.code.starts_with("R-RES"), "{err:?}");
        assert!(err.message.contains("@acme/x"), "{err:?}");
    }

    #[specforge_test(
        behavior = "management_operations_over_the_project_view",
        verify = "add, update and remove refuse an unusable specforge.json with one refusal, before they write"
    )]
    fn an_unusable_config_is_refused_before_the_lock_is_read() {
        use crate::config::testing::{UNUSABLE, files_under};

        for config in UNUSABLE {
            // With a lock and without one: the config is refused first.
            for locked in [true, false] {
                let dir = if locked {
                    project(vec![entry("@sdk/greet", "0.0.9", "registry", &[])])
                } else {
                    tempfile::tempdir().unwrap()
                };
                std::fs::write(dir.path().join("specforge.json"), config).unwrap();
                let before = files_under(dir.path());
                let read = specforge_common::read_project_config(dir.path());
                let refused = crate::config::refusal(&read.problems[0]);

                let unconfigured = crate::registry::Unconfigured("update");
                let error =
                    update(&request(dir.path(), false), &unconfigured, &runtime()).unwrap_err();

                assert_eq!(error, refused, "{config}, locked: {locked}");
                assert_eq!(files_under(dir.path()), before, "{config}");
            }
        }
    }
}
