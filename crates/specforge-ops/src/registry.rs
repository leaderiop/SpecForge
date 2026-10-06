//! The package registry port the extension operations use.
//!
//! SpecForge ships no registry (ADR 0004 N1): the only registries are the
//! ones the project's `specforge.json` lists under `registries`. With none,
//! a registry operation fails with E063 before it makes any network call.
//!
//! The operations name only the [`Registry`] trait. The adapter that talks
//! HTTP, checks publisher signatures and pins keys is
//! `specforge_ops_registry::HttpRegistry`, which only the surfaces that
//! reach a registry (the CLI, MCP) link (ADR 0010).

use crate::OpError;
use crate::extension::Trust;
use specforge_protocol_types::ExtensionDeclaration;

/// The diagnostic a registry operation reports when no registry is
/// configured.
pub const NO_REGISTRY: &str = "E063";

/// A registry answered with metadata that doesn't describe the package
/// asked for: another name or version, or a declaration other than its
/// binary's.
pub const METADATA_MISMATCH: &str = "R-TRUST-004";

/// The registry served a package whose manifest can't be read as an
/// extension declaration.
pub const UNREADABLE_MANIFEST: &str = "R-OPS-004";

/// How to configure a registry, for E063's suggestion.
pub const CONFIGURE_HINT: &str = "add a \"registries\" array to specforge.json, e.g. \
     \"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]";

/// E063 for `operation`.
pub fn no_registry(operation: &str) -> OpError {
    OpError::new(
        NO_REGISTRY,
        format!(
            "no registry configured: `{operation}` needs one, and SpecForge has no built-in registry"
        ),
    )
    .with_suggestion(CONFIGURE_HINT)
}

/// A package downloaded from a registry: its bytes checked against the
/// SHA-256 the registry published, its publisher signature against the
/// trust policy the caller gave.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
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

/// The registry port the extension operations use: an HTTP adapter in
/// production, an in-memory fake in tests.
pub trait Registry {
    /// The version `range` (`latest`, `*`, `^1.2`, `~1`, `>=1`) resolves to;
    /// an exact version is returned as it is.
    fn resolve_version(&self, name: &str, range: &str) -> Result<String, OpError>;
    /// `name@version`, downloaded and integrity-checked, its publisher
    /// signature checked under the TOFU pin policy: a package with none is
    /// refused unless `allow_unsigned`; a changed key is decided by `trust`.
    /// An answer for another package or version, or one whose manifest
    /// can't be read, is refused.
    fn fetch(
        &self,
        name: &str,
        version: &str,
        allow_unsigned: bool,
        trust: Trust,
    ) -> Result<Package, OpError>;
    /// Every version the registry publishes for `name`.
    fn versions(&self, name: &str) -> Result<Vec<String>, OpError>;
}

/// Whether `range` needs resolving against the registry's versions (an
/// exact version doesn't).
pub fn is_range(range: &str) -> bool {
    range == "latest"
        || range == "*"
        || range.starts_with('^')
        || range.starts_with('~')
        || range.starts_with('>')
        || range.starts_with('<')
        || range.starts_with('=')
}

/// No registry: every call fails with E063 naming `operation`. What an
/// operation that never reaches a registry (`init`, which installs only
/// builtins and local files) passes where one is asked for.
pub struct Unconfigured(pub &'static str);

impl Registry for Unconfigured {
    fn resolve_version(&self, _: &str, _: &str) -> Result<String, OpError> {
        Err(no_registry(self.0))
    }

    fn fetch(&self, _: &str, _: &str, _: bool, _: Trust) -> Result<Package, OpError> {
        Err(no_registry(self.0))
    }

    fn versions(&self, _: &str) -> Result<Vec<String>, OpError> {
        Err(no_registry(self.0))
    }
}
