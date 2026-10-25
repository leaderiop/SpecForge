//! What a lock plan installs: a local file or a registry fetch, and the check that a fetched binary is what
//! was published (ADR 0012).

use std::path::Path;

use specforge_common::codes;
use specforge_installed::{LockSource, Module, Pin, Sha256};
use specforge_package::{NotAPackage, PackageName, Version};
use specforge_protocol_types::{DeclaredCategory, ExtensionDeclaration};

use super::super::candidate::{Installable, LocalFile};
use super::From;
use super::{Install, LockPlan, Purpose, Staged};
use crate::registry::{KeyPin, LockedPin, METADATA_MISMATCH, Publisher, Registry, TrustPolicy};
use crate::{OpError, OpErrorKind};

impl<'a> Install<'a> {
    /// A `.wasm` file read once, installed as `local:<path>`. The path is relative to `root` when
    /// the file is under it, absolute otherwise. E072 when it declares no package name.
    pub(crate) fn local(root: &Path, local: LocalFile) -> Result<Install<'a>, OpError> {
        let declared = local.binary.declaration();
        let name = super::super::candidate::package(declared)?;
        let version = Version::parse(declared.version())
            .map_err(|why| not_semver(declared.version(), &why))?;
        let pin = Pin {
            name,
            version,
            source: LockSource::local(shown_path(root, &local.path))
                .map_err(|why| unrecordable(&local.path, &why))?,
            key_id: None,
            peers: declared.peers().to_vec(),
        };
        let module = local.binary.into_module();
        Ok(Install {
            sha256: module.digest().clone(),
            module: Some(module),
            pin,
            from: From::File,
        })
    }

    pub(crate) fn name(&self) -> &PackageName {
        &self.pin.name
    }

    /// The version it locks: what its declaration declares.
    pub(crate) fn version(&self) -> &Version {
        &self.pin.version
    }

    /// The module's SHA-256: what the lock pins.
    pub(crate) fn sha256(&self) -> &Sha256 {
        &self.sha256
    }

    pub(crate) fn source(&self) -> &LockSource {
        &self.pin.source
    }

    /// How the registry accepted its publisher; `None` for a local file.
    pub(crate) fn publisher(&self) -> Option<&Publisher> {
        match &self.from {
            From::Registry { publisher, .. } => Some(publisher),
            From::File | From::Published { .. } | From::Locked => None,
        }
    }
}

impl Staged<'_> {
    /// The pin its registry fetch decided, once it is written; none for a local file, a key already
    /// pinned or an unsigned package.
    pub(super) fn key_pin(&self) -> Option<KeyPin> {
        match &self.from {
            From::Registry { publisher, .. } => publisher.pin(&self.name),
            From::File | From::Published { .. } | From::Locked => None,
        }
    }

    /// The registry an unsatisfied peer touching this install is unified against; `None` for a
    /// local file (E027).
    pub(super) fn registry(&self) -> Option<&dyn Registry> {
        match &self.from {
            From::Registry { registry, .. } | From::Published { registry } => Some(*registry),
            From::File | From::Locked => None,
        }
    }

    /// A fetched binary loaded in its runtime and held to what was published (ADR 0012). A local
    /// file was checked when it was read.
    pub(super) fn load(&self) -> Result<(), OpError> {
        let From::Registry {
            runtime,
            declaration,
            bytes,
            ..
        } = &self.from
        else {
            // A local file was checked when it was read; a preview loads nothing.
            return Ok(());
        };
        let binary = Installable::read(*runtime, bytes.clone()).map_err(published_wrongly)?;
        let declared = binary.declaration();
        if declared.name() != self.name.as_str() || declared.version() != self.version.to_string() {
            return Err(OpError::coded(
                OpErrorKind::SchemaMismatch,
                codes::E028,
                format!(
                    "registry package {}@{} declares itself {}@{}",
                    self.name,
                    self.version,
                    declared.name(),
                    declared.version()
                ),
            ));
        }
        if let Some(category) = first_difference(declaration, declared) {
            return Err(OpError::coded(
                OpErrorKind::SchemaMismatch,
                METADATA_MISMATCH,
                format!(
                    "registry package {}@{} declares another {category} than the binary it serves",
                    self.name, self.version
                ),
            )
            .with_suggestion("don't install the package, and check the registry"));
        }
        Ok(())
    }
}

impl<'a> LockPlan<'a> {
    /// `name@version` from the plan's registry, fetched: downloaded, integrity- and
    /// signature-checked under the plan's trust choices, held to the key the lock records for
    /// `name` when the user has no pin of their own, nothing written. Its pin is read from the
    /// published declaration: its version and its peers, the publisher's key id, `source:
    /// registry`. Its binary is not loaded: [`Self::check`] loads it after the gate. Nothing is
    /// pinned for the user: the publisher the fetch decided is pinned when the plan commits. For a
    /// preview, its [`Manifest`], its publisher decided as the fetch decides it: nothing is downloaded. E063
    /// when the plan fetches from no registry. A preview whose published digest is no SHA-256 is
    /// R-OPS-002, as its download would be.
    pub(crate) fn fetch(
        &self,
        name: &PackageName,
        version: &Version,
    ) -> Result<Install<'a>, OpError> {
        // What the project already has is answered before any registry call (ADR 0138).
        if let Some(entry) =
            self.unchanged(name, |e| e.source.is_registry() && e.version == *version)
        {
            return Ok(Install {
                module: None,
                sha256: entry.wasm_hash.clone(),
                pin: Pin {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    source: entry.source.clone(),
                    key_id: entry.key_id.clone(),
                    peers: entry.peer_dependencies.clone(),
                },
                from: From::Locked,
            });
        }
        let Some((fetching, runtime)) = self.fetching else {
            return Err(crate::registry::no_registry(self.operation));
        };
        let policy = TrustPolicy::new(fetching.allow_unsigned, self.caller.asking())
            .locked(self.installed.locked(name).map(LockedPin::of));
        if self.purpose == Purpose::Preview {
            // Decided as the fetch decides it, downloading nothing (ADR 0213).
            let manifest = fetching.registry.published(name, version, policy)?;
            let sha256 = Sha256::parse(&manifest.sha256).map_err(|why| {
                OpError::coded(
                    OpErrorKind::SchemaMismatch,
                    codes::R_OPS_002,
                    format!(
                        "registry package {}@{} publishes a digest no download can match: {}",
                        manifest.name,
                        manifest.version,
                        specforge_common::remedy::printable(&why),
                    ),
                )
            })?;
            let pin = Pin {
                name: manifest.name,
                version: manifest.version.clone(),
                source: LockSource::Registry,
                key_id: manifest.publisher.key_id().map(str::to_string),
                peers: manifest.declaration.peers().to_vec(),
            };
            return Ok(Install {
                module: None,
                sha256,
                pin,
                from: From::Published {
                    registry: fetching.registry,
                },
            });
        }
        let mut package = fetching.registry.fetch(name, version, policy)?;
        let module = Module::new(std::mem::take(&mut package.wasm));
        let pin = Pin {
            name: package.name.clone(),
            version: package.version.clone(),
            source: LockSource::Registry,
            key_id: package.publisher.key_id().map(str::to_string),
            peers: package.declaration.peers().to_vec(),
        };
        Ok(Install {
            from: From::Registry {
                registry: fetching.registry,
                runtime,
                declaration: Box::new(package.declaration),
                publisher: package.publisher,
                bytes: module.clone(),
            },
            sha256: module.digest().clone(),
            module: Some(module),
            pin,
        })
    }
}

/// A failure of reading a fetched binary: the caller gave no such value, the registry served a
/// binary that is not what it published (`schema_mismatch`, as `METADATA_MISMATCH` beside it).
fn published_wrongly(mut error: OpError) -> OpError {
    if error.kind == OpErrorKind::InvalidInput {
        error.kind = OpErrorKind::SchemaMismatch;
    }
    error
}

/// The first part of a declaration that differs between `served` and `binary` (`handshake`, or a
/// describe category), or `None` when they are equal.
fn first_difference(
    served: &ExtensionDeclaration,
    binary: &ExtensionDeclaration,
) -> Option<&'static str> {
    if served.handshake != binary.handshake {
        return Some("handshake");
    }
    DeclaredCategory::ALL
        .into_iter()
        .find(|category| served.describe_items(*category) != binary.describe_items(*category))
        .map(DeclaredCategory::name)
}

/// An `add` of a local file declaring a version that is no SemVer version: E072, before anything is staged,
/// so the lock never holds a version its own read refuses (ADR 0243).
fn not_semver(text: &str, why: &dyn std::fmt::Display) -> OpError {
    let why = NotAPackage::Version {
        text: text.to_string(),
        why: why.to_string(),
    };
    OpError::of(
        OpErrorKind::InvalidInput,
        specforge_common::package::invalid(&specforge_common::remedy::printable(&why.to_string())),
    )
    .with_suggestion(
        "fix the extension's declared version (its SDK `version`) and rebuild, or enable the file \
         as a `.wasm` file entry in specforge.json, which records nothing in specforge.lock",
    )
}

/// An `add` of a file whose path `specforge.lock` cannot record (ADR 0230): refused before anything is
/// staged, so the lock never holds a source its own read refuses.
fn unrecordable(path: &Path, why: &str) -> OpError {
    OpError::coded(
        OpErrorKind::InvalidInput,
        codes::E054,
        format!(
            "'{}' cannot be recorded in specforge.lock: {}",
            specforge_common::remedy::printable(&path.display().to_string()),
            specforge_common::remedy::printable(why)
        ),
    )
    .with_suggestion("rename the file, then add it again")
}

/// `path` as the lock records it: relative to the project root when it is under it, absolute
/// otherwise.
fn shown_path(root: &Path, path: &Path) -> String {
    let absolute = |p: &Path| {
        std::path::absolute(p)
            .map(|p| p.canonicalize().unwrap_or(p))
            .unwrap_or_else(|_| p.to_path_buf())
    };
    let (root, path) = (absolute(root), absolute(path));
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .display()
        .to_string()
}
