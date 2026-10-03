//! Which registries a project uses, and the HTTP adapter behind
//! `specforge_ops::registry::Registry`.
//!
//! `specforge-ops` names only the port, so a surface that never reaches a
//! registry (the LSP) links no HTTP client, keyring or signature code
//! (ADR 0010). The CLI and MCP, whose `add` and `update` do, build an
//! [`HttpRegistry`] and pass it in.

use specforge_common::Diagnostic;
use specforge_ops::OpError;
use specforge_ops::config::CONFIG_FILE;
use specforge_ops::extension::Trust;
use specforge_ops::registry::{Package, Registry, is_range, no_registry};
use specforge_registry::ManifestV2;
use specforge_registry_client::{
    HttpRegistryClient, RegistryConfig, find_registry_for_specifier, parse_registries_from_config,
    resolve_from_registry, resolve_version, verify_registry_integrity,
};
use std::path::Path;

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

    fn fetch(
        &self,
        name: &str,
        version: &str,
        allow_unsigned: bool,
        trust: Trust,
    ) -> Result<Package, OpError> {
        let (registries, _) = self.registry_for(name)?;
        let response =
            resolve_from_registry(&format!("{name}@{version}"), registries, &self.client)
                .map_err(OpError::from)?;
        let wasm = self
            .client
            .download_wasm(&response.wasm_url)
            .map_err(|e| OpError::from(e.to_diagnostic()))?;
        verify_registry_integrity(&wasm, &response.sha256).map_err(OpError::from)?;

        // Publisher signature and the TOFU pin policy.
        let (assume_yes, format) = match trust {
            Trust::Refuse => (false, "json"),
            Trust::AssumeYes => (true, "human"),
            Trust::Prompt => (false, "human"),
        };
        let trusted = specforge_registry_client::trust_flow::check_and_pin(
            &response.name,
            &response,
            &wasm,
            allow_unsigned,
            assume_yes,
            format,
            None,
        )
        .map_err(OpError::from)?;

        let peers = serde_json::from_str::<ManifestV2>(&response.manifest)
            .map(|m| m.peer_dependencies)
            .unwrap_or_default();
        Ok(Package {
            name: response.name,
            version: response.version,
            sha256: response.sha256,
            wasm,
            peers,
            key_id: trusted.key_id,
        })
    }

    fn versions(&self, name: &str) -> Result<Vec<String>, OpError> {
        let (_, registry) = self.registry_for(name)?;
        self.client
            .fetch_versions(name, registry)
            .map_err(|e| OpError::from(e.to_diagnostic()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconfigured_project_fails_each_call_with_e063_before_any_request() {
        let dir = tempfile::tempdir().unwrap();
        let registry = HttpRegistry::for_project(dir.path(), "update");
        assert!(registry.diagnostics().is_empty());
        let error = registry.versions("@sdk/greet").unwrap_err();
        assert_eq!(error.code, specforge_ops::registry::NO_REGISTRY);
        assert!(error.message.contains("`update`"), "{error:?}");
        let error = registry
            .fetch("@sdk/greet", "0.1.0", false, Trust::Refuse)
            .unwrap_err();
        assert_eq!(error.code, specforge_ops::registry::NO_REGISTRY);
    }
}
