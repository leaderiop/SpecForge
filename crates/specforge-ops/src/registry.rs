//! The package registry port the extension operations use. It lists a package's versions, fetches one and
//! publishes one.
//!
//! SpecForge ships no registry (ADR 0004 N1): the only registries are the
//! ones the project's `specforge.json` lists under `registries`. With none,
//! a registry operation fails with E063 before it makes any network call.
//!
//! The operations name only the [`Registry`] trait. The adapter that talks
//! HTTP, checks publisher signatures and pins keys is
//! `specforge_ops_registry::ConfiguredRegistry`, which only the surfaces that
//! reach a registry (the CLI, MCP) link (ADR 0010).

use crate::extension::Trust;
use crate::{OpError, OpErrorKind};
use specforge_common::{Code, codes};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_protocol_types::package::{PackageName, Version};

/// The diagnostic a registry operation reports when no registry is
/// configured.
pub const NO_REGISTRY: Code = codes::E063;

/// No configured registry serves the package name: no `scope_filter` is its
/// scope, and none is the default (ADR 0045).
pub const NO_REGISTRY_FOR_NAME: Code = codes::R_OPS_001;

/// A publish has no credential for its registry (none stored for its alias, no
/// token in the environment), or the registry refused the one it sent (HTTP 401).
pub const NOT_AUTHENTICATED: Code = codes::R001;

/// The user's publisher signing key can't be read or created.
pub const UNUSABLE_SIGNING_KEY: Code = codes::E074;

/// The registry already holds the version: a published version is immutable.
pub const ALREADY_PUBLISHED: Code = codes::R007;

/// A registry answered with metadata that doesn't describe the package
/// asked for: another name or version, or a declaration other than its
/// binary's.
pub const METADATA_MISMATCH: Code = codes::R_TRUST_004;

/// The registry served a package whose manifest can't be read as an
/// extension declaration.
pub const UNREADABLE_MANIFEST: Code = codes::R_OPS_004;

/// How to configure a registry, for E063's suggestion.
pub const CONFIGURE_HINT: &str = "add a \"registries\" array to specforge.json, e.g. \
     \"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]";

/// E063 for `operation`.
pub fn no_registry(operation: &str) -> OpError {
    OpError::coded(
        OpErrorKind::PreconditionFailed,
        NO_REGISTRY,
        format!("no registry configured: `{operation}` needs one, and SpecForge has no built-in registry"),
    )
    .with_suggestion(CONFIGURE_HINT)
}

/// A package downloaded from a registry: its bytes checked against the
/// SHA-256 the registry published, its publisher signature against the
/// trust policy the caller gave.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: PackageName,
    pub version: Version,
    pub wasm: Vec<u8>,
    pub sha256: String,
    /// The declaration published with it (its manifest): the diamond gate
    /// reads its peers before the binary is loaded, and the binary must then
    /// declare exactly the same (ADR 0012).
    pub declaration: ExtensionDeclaration,
    /// The publisher key it was signed with; `None` when unsigned (and
    /// unsigned packages were allowed).
    pub key_id: Option<String>,
}

/// A package `publish` hands the registry: a scoped package name and a full
/// version (both checked by ops, E072), the binary, and the declaration read
/// from it, which the registry stores as the package's manifest (ADR 0012).
#[derive(Debug, Clone, Copy)]
pub struct Upload<'a> {
    pub name: &'a PackageName,
    pub version: &'a Version,
    pub wasm: &'a [u8],
    pub declaration: &'a ExtensionDeclaration,
}

/// What a registry answered a publish with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    /// The registry that took the package: its `alias` in `specforge.json`.
    pub registry: String,
    /// Where the registry serves the published version.
    pub url: String,
    /// The publisher key the package is signed with.
    pub key_id: String,
    /// Whether that key was created for this publish (the user's first).
    pub key_created: bool,
}

/// The registry port the extension operations use. It lists a package's versions,
/// fetches one and publishes one; it does not resolve a requirement (ADR 0036): ops
/// does, with [`crate::extension::resolve`]. Each call asks the one registry that
/// serves the name (ADR 0045).
///
/// Two adapters (ADR 0044): `specforge_ops_registry::ConfiguredRegistry` in production and
/// [`testing::MemoryRegistry`] in tests, both held to [`testing::assert_registry_contract`].
pub trait Registry {
    /// Every version `name` publishes (unparseable ones dropped);
    /// R-RES-001 when the registry has no such package.
    fn versions(&self, name: &PackageName) -> Result<Vec<Version>, OpError>;
    /// `name@version`, downloaded and integrity-checked, its publisher
    /// signature checked under the TOFU pin policy: a package with none is
    /// refused unless `allow_unsigned`; a changed key is decided by `trust`.
    /// An answer for another package or version, or one whose manifest
    /// can't be read, is refused.
    fn fetch(
        &self,
        name: &PackageName,
        version: &Version,
        allow_unsigned: bool,
        trust: Trust,
    ) -> Result<Package, OpError>;
    /// Publish `package` to the registry that serves its name (the one
    /// `fetch` asks for it), signed with the user's publisher key and
    /// authenticated as the user is to that registry.
    ///
    /// Refused before any request, in this order:
    /// - E063/E067: the registry configuration;
    /// - R-OPS-001: no registry serves the name;
    /// - R001: there is no credential for it (R012, R-AUTH-020, R-AUTH-021
    ///   when the stored one can't be used);
    /// - E074: the signing key can't be read or created.
    ///
    /// Then the registry's own refusals: R007 for a version it holds,
    /// R001/R002 for a credential it refuses.
    fn publish(&self, package: &Upload<'_>) -> Result<Published, OpError>;
}

#[cfg(any(test, feature = "testing"))]
pub mod testing;

/// No registry: every call fails with E063 naming `operation`. What an
/// operation that never reaches a registry (`init`, which installs only
/// builtins and local files) passes where one is asked for.
pub struct Unconfigured(pub &'static str);

impl Registry for Unconfigured {
    fn versions(&self, _: &PackageName) -> Result<Vec<Version>, OpError> {
        Err(no_registry(self.0))
    }

    fn fetch(&self, _: &PackageName, _: &Version, _: bool, _: Trust) -> Result<Package, OpError> {
        Err(no_registry(self.0))
    }

    fn publish(&self, _: &Upload<'_>) -> Result<Published, OpError> {
        Err(no_registry(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconfigured_registry_refuses_a_publish_with_e063() {
        let name = PackageName::parse("@acme/x").unwrap();
        let version = Version::new(1, 0, 0);
        let declaration = testing::declaration("@acme/x", "1.0.0", &[]);
        let error = Unconfigured("publish")
            .publish(&Upload {
                name: &name,
                version: &version,
                wasm: b"\0asm",
                declaration: &declaration,
            })
            .unwrap_err();
        assert!(error.is(NO_REGISTRY), "{error:?}");
        assert!(error.message.contains("`publish`"), "{error:?}");
    }
}
