//! Which registries a project uses, and the HTTP adapter behind
//! `specforge_ops::registry::Registry`.
//!
//! `specforge-ops` names only the port, so a surface that never reaches a
//! registry (the LSP) links no HTTP client, keyring or signature code
//! (ADR 0010). The CLI and MCP, whose `add` and `update` do, build an
//! [`HttpRegistry`] and pass it in.

use specforge_common::{Code, Diagnostic, codes};
use specforge_ops::extension::Trust;
use specforge_ops::registry::{
    METADATA_MISMATCH, Package, Registry, UNREADABLE_MANIFEST, no_registry,
};
use specforge_ops::{OpError, OpErrorKind};
use specforge_protocol_types::package::Version;
use specforge_protocol_types::{ExtensionDeclaration, PackageName};
use specforge_registry_client::trust_flow::TrustPolicy;
use specforge_registry_client::{
    HttpRegistryClient, RegistryClient, RegistryConfig, RegistryError, find_registry_for,
    parse_registries_from_config, verify_registry_integrity,
};
use std::path::{Path, PathBuf};

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
const INVALID_CONFIG: Code = codes::E067;

/// The registries the project at `root` configures.
///
/// `operation` names the command for the message (`add`, `search`, ...).
/// No `specforge.json`, or one without a `registries` entry, fails with E063;
/// one that is there and unusable fails as `add` fails (`config_invalid`).
/// When `registries` has entries but none can be read, it fails with E067
/// naming them.
pub fn configured(root: &Path, operation: &str) -> Result<Configured, OpError> {
    // `specforge.json` is read the way `add`, `update` and `remove` read
    // it: one that is there and unusable is refused as they refuse it
    // (`config_invalid`, E069), not as a missing registry.
    let read = specforge_ops::config::usable(root)?;
    let Some(raw) = read.config.raw.as_ref().filter(|_| read.found) else {
        return Err(no_registry(operation));
    };
    let (registries, diagnostics) = parse_registries_from_config(&raw.to_string());
    if registries.is_empty() {
        let unreadable: Vec<&str> = diagnostics
            .iter()
            .filter(|d| d.is(INVALID_CONFIG))
            .map(|d| d.message.as_str())
            .collect();
        if unreadable.is_empty() {
            return Err(no_registry(operation));
        }
        return Err(OpError::coded(
            OpErrorKind::SchemaMismatch,
            INVALID_CONFIG,
            unreadable.join("; "),
        )
        .with_suggestion(
            "fix the \"registries\" entries in specforge.json: each needs an \"alias\" and a \"url\"",
        ));
    }
    Ok(Configured {
        registries,
        diagnostics,
    })
}

/// The project's configured registries over HTTP. Built without touching
/// the network or failing: with no registry configured, each call fails
/// with E063 before any request.
pub struct HttpRegistry {
    registries: Result<Configured, OpError>,
    client: HttpRegistryClient,
    /// Where publisher keys are pinned; `None` is the user's
    /// `~/.specforge/known-keys.json`.
    known_keys: Option<PathBuf>,
}

impl HttpRegistry {
    /// The registries `root`'s `specforge.json` configures; `operation`
    /// names the command in E063.
    pub fn for_project(root: &Path, operation: &str) -> Self {
        Self {
            registries: configured(root, operation),
            client: HttpRegistryClient::new(),
            known_keys: None,
        }
    }

    /// Pin and check publisher keys in the store at `path` instead of the
    /// user's `~/.specforge/known-keys.json` (a test, or a custom home).
    pub fn with_known_keys(mut self, path: impl Into<PathBuf>) -> Self {
        self.known_keys = Some(path.into());
        self
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

    /// The one registry that serves `name`: made once per call, and the
    /// client fetches from it without choosing again.
    fn registry_for(&self, name: &PackageName) -> Result<&RegistryConfig, OpError> {
        let registries = &self.registries.as_ref().map_err(Clone::clone)?.registries;
        // `configured` refuses an empty list, so there is always a first.
        Ok(find_registry_for(name, registries).unwrap_or(&registries[0]))
    }
}

impl Registry for HttpRegistry {
    fn fetch(
        &self,
        name: &PackageName,
        version: &Version,
        allow_unsigned: bool,
        trust: Trust,
    ) -> Result<Package, OpError> {
        let registry = self.registry_for(name)?;
        let response = self
            .client
            .metadata(name, version, registry)
            .map_err(|e| OpError::from(e.to_diagnostic()))?;
        // The signature covers the name and version the registry answers
        // with, and the pin is keyed by that name: an answer for another
        // package (or another version) would be verified, pinned and
        // installed in place of the one asked for.
        if response.name != name.as_str() || response.version != version.to_string() {
            return Err(OpError::coded(
                OpErrorKind::SchemaMismatch,
                METADATA_MISMATCH,
                format!(
                    "registry answered {name}@{version} with {}@{}",
                    response.name, response.version
                ),
            )
            .with_suggestion("don't install the package, and check the registry"));
        }
        let wasm = self
            .client
            .download(&response.wasm_url)
            .map_err(|e| OpError::from(e.to_diagnostic()))?;
        verify_registry_integrity(&wasm, &response.sha256).map_err(OpError::from)?;

        // The served manifest is the package's declaration (ADR 0012): the
        // peers the diamond gate (ADR 0001) decides on, and what the binary
        // must declare once loaded. One that can't be read must not pass as
        // "no peers", and one describing another package must not be
        // installed as this one. Checked before the signature, so a refused
        // package pins no key.
        let declaration = read_declaration(name, version, &response.manifest)?;
        if declaration.name() != name.as_str() || declaration.version() != version.to_string() {
            return Err(OpError::coded(
                OpErrorKind::SchemaMismatch,
                METADATA_MISMATCH,
                format!(
                    "registry served {name}@{version} with the declaration of {}@{}",
                    declaration.name(),
                    declaration.version()
                ),
            )
            .with_suggestion("don't install the package, and check the registry"));
        }

        // Publisher signature and the TOFU pin policy.
        let trusted = specforge_registry_client::trust_flow::check_and_pin(
            &response.name,
            &response,
            &wasm,
            allow_unsigned,
            policy(trust),
            self.known_keys.as_deref(),
        )
        .map_err(OpError::from)?;

        Ok(Package {
            name: name.clone(),
            version: version.clone(),
            sha256: response.sha256,
            wasm,
            declaration,
            key_id: trusted.key_id,
        })
    }

    fn versions(&self, name: &PackageName) -> Result<Vec<Version>, OpError> {
        let registry = self.registry_for(name)?;
        let published = self
            .client
            .versions(name, registry)
            .map_err(|error| match error {
                RegistryError::NotFound { .. } => Diagnostic::new(
                    codes::R_RES_001,
                    format!(
                        "package '{name}' not found in registry '{}'",
                        registry.alias
                    ),
                )
                .with_suggestion("check the package name and registry configuration".to_string()),
                other => other.to_diagnostic(),
            })?;
        Ok(published
            .iter()
            .filter_map(|text| Version::parse(text).ok())
            .collect())
    }
}

/// How the client decides a key change for the way `add` was asked to.
fn policy(trust: Trust) -> TrustPolicy {
    match trust {
        Trust::Refuse => TrustPolicy::Refuse,
        Trust::AssumeYes => TrustPolicy::AssumeYes,
        Trust::Prompt => TrustPolicy::Prompt,
    }
}

/// The declaration a registry stores as `name@version`'s manifest. A
/// manifest from before ADR 0012 (the camelCase `manifest.json` form, with
/// `manifestVersion`) can't be read as one: it is refused with the
/// suggestion to re-publish the package.
fn read_declaration(
    name: &PackageName,
    version: &Version,
    manifest: &str,
) -> Result<ExtensionDeclaration, OpError> {
    let unreadable = |why: String| {
        OpError::coded(
            OpErrorKind::SchemaMismatch,
            UNREADABLE_MANIFEST,
            format!("the manifest of {name}@{version} can't be read: {why}"),
        )
        .with_suggestion("don't install the package, and check the registry")
    };
    if manifest.trim().is_empty() {
        return Err(unreadable("the registry served none".to_string()));
    }
    let value: serde_json::Value =
        serde_json::from_str(manifest).map_err(|e| unreadable(e.to_string()))?;
    if value.get("manifestVersion").is_some() {
        return Err(unreadable(
            "it is a manifest.json from before extension declarations".to_string(),
        )
        .with_suggestion(format!(
            "re-publish {name}@{version} with this version of specforge publish, which uploads \
             the declaration it reads from the binary"
        )));
    }
    serde_json::from_value(value).map_err(|e| unreadable(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconfigured_project_fails_each_call_with_e063_before_any_request() {
        let dir = tempfile::tempdir().unwrap();
        let registry = HttpRegistry::for_project(dir.path(), "update");
        assert!(registry.diagnostics().is_empty());
        let sdk = PackageName::parse("@sdk/greet").unwrap();
        let error = registry.versions(&sdk).unwrap_err();
        assert!(error.is(specforge_ops::registry::NO_REGISTRY), "{error:?}");
        assert!(error.message.contains("`update`"), "{error:?}");
        let error = registry
            .fetch(&sdk, &Version::new(0, 1, 0), false, Trust::Refuse)
            .unwrap_err();
        assert!(error.is(specforge_ops::registry::NO_REGISTRY), "{error:?}");
    }
}
