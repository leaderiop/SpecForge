//! Which registries a project uses.
//!
//! SpecForge ships no registry (ADR 0004 N1): the only registries are the
//! ones the project's `specforge.json` lists under `registries`. With none,
//! a registry operation fails with E063 before it makes any network call.

use crate::OpError;
use crate::config::CONFIG_FILE;
use specforge_common::Diagnostic;
use specforge_registry::{ManifestV2, PeerDependency};
use specforge_registry_client::registry_client::RegistryResponse;
use specforge_registry_client::{
    HttpRegistryClient, RegistryConfig, find_registry_for_specifier, parse_registries_from_config,
    resolve_from_registry, resolve_version, verify_registry_integrity,
};
use std::path::Path;

/// The diagnostic a registry operation reports when no registry is
/// configured.
pub const NO_REGISTRY: &str = "E063";

/// How to configure a registry, for E063's suggestion.
pub const CONFIGURE_HINT: &str = "add a \"registries\" array to specforge.json, e.g. \
     \"registries\": [{\"alias\": \"main\", \"url\": \"<registry URL>\", \"default_registry\": true}]";

/// The registries a project configures, and what reading them reported.
#[derive(Debug, Clone)]
pub struct Configured {
    /// The entries that could be read, in declaration order.
    pub registries: Vec<RegistryConfig>,
    /// E067 for each entry that couldn't be read (it is skipped), W140 for
    /// a duplicate alias, I003 when no entry is the default. The caller
    /// shows them.
    pub diagnostics: Vec<Diagnostic>,
}

/// The diagnostic for a registry configuration that can't be read.
const INVALID_CONFIG: &str = "E067";

/// The registries the project at `root` configures.
///
/// `operation` names the command for the message (`add`, `search`, ...).
/// No `specforge.json`, one that can't be read, or one without a
/// `registries` entry all fail with E063. When `registries` has entries but
/// none can be read, it fails with E067 naming them.
pub fn configured(root: &Path, operation: &str) -> Result<Configured, OpError> {
    let Ok(content) = std::fs::read_to_string(root.join(CONFIG_FILE)) else {
        return Err(no_registry(operation));
    };
    let (registries, diagnostics) = parse_registries_from_config(&content);
    if registries.is_empty() {
        let unreadable: Vec<&str> = diagnostics
            .iter()
            .filter(|d| d.code == INVALID_CONFIG)
            .map(|d| d.message.as_str())
            .collect();
        if unreadable.is_empty() {
            return Err(no_registry(operation));
        }
        return Err(
            OpError::new(INVALID_CONFIG, unreadable.join("; ")).with_suggestion(
                "fix the \"registries\" entries in specforge.json: each needs an \"alias\" and a \"url\"",
            ),
        );
    }
    Ok(Configured {
        registries,
        diagnostics,
    })
}

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

/// A package downloaded from a registry, its bytes checked against the
/// SHA-256 the registry published.
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub wasm: Vec<u8>,
    pub sha256: String,
    /// The peers its published manifest declares.
    pub peers: Vec<PeerDependency>,
    /// The registry's answer, for the publisher signature check.
    pub response: RegistryResponse,
}

/// The registry port the extension operations use: [`HttpRegistry`] in
/// production, an in-memory fake in tests.
pub trait Registry {
    /// The version `range` (`latest`, `*`, `^1.2`, `~1`, `>=1`) resolves to;
    /// an exact version is returned as it is.
    fn resolve_version(&self, name: &str, range: &str) -> Result<String, OpError>;
    /// `name@version`, downloaded and integrity-checked.
    fn fetch(&self, name: &str, version: &str) -> Result<Package, OpError>;
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

/// The project's configured registries over HTTP. Built without touching
/// the network or failing: with no registry configured, each call fails
/// with E063 before any request.
pub struct HttpRegistry {
    registries: Result<Configured, OpError>,
    client: HttpRegistryClient,
}

impl HttpRegistry {
    /// The registries `root`'s `specforge.json` configures; `operation`
    /// names the command in E063.
    pub fn for_project(root: &Path, operation: &str) -> Self {
        Self {
            registries: configured(root, operation),
            client: HttpRegistryClient::new(),
        }
    }

    /// What reading the registry configuration reported (see
    /// [`Configured::diagnostics`]); none when it failed outright, since
    /// each registry call then fails with that error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match &self.registries {
            Ok(configured) => &configured.diagnostics,
            Err(_) => &[],
        }
    }

    fn registry_for(&self, name: &str) -> Result<(&[RegistryConfig], &RegistryConfig), OpError> {
        let registries = &self.registries.as_ref().map_err(Clone::clone)?.registries;
        let registry = find_registry_for_specifier(name, registries)
            .or_else(|| registries.first())
            .ok_or_else(|| no_registry("add"))?;
        Ok((registries, registry))
    }
}

impl Registry for HttpRegistry {
    fn resolve_version(&self, name: &str, range: &str) -> Result<String, OpError> {
        if !is_range(range) {
            return Ok(range.to_string());
        }
        let (_, registry) = self.registry_for(name)?;
        resolve_version(name, range, &self.client, registry).map_err(OpError::from)
    }

    fn fetch(&self, name: &str, version: &str) -> Result<Package, OpError> {
        let (registries, _) = self.registry_for(name)?;
        let response =
            resolve_from_registry(&format!("{name}@{version}"), registries, &self.client)
                .map_err(OpError::from)?;
        let wasm = self
            .client
            .download_wasm(&response.wasm_url)
            .map_err(|e| OpError::from(e.to_diagnostic()))?;
        verify_registry_integrity(&wasm, &response.sha256).map_err(OpError::from)?;
        let peers = serde_json::from_str::<ManifestV2>(&response.manifest)
            .map(|m| m.peer_dependencies)
            .unwrap_or_default();
        Ok(Package {
            name: response.name.clone(),
            version: response.version.clone(),
            sha256: response.sha256.clone(),
            wasm,
            peers,
            response,
        })
    }

    fn versions(&self, name: &str) -> Result<Vec<String>, OpError> {
        let (_, registry) = self.registry_for(name)?;
        self.client
            .fetch_versions(name, registry)
            .map_err(|e| OpError::from(e.to_diagnostic()))
    }
}
