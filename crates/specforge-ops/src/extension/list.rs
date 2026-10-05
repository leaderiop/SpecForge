//! `specforge extensions` / `specforge providers` and their MCP tools.

use super::{Origin, builtin_name, lock_path};
use specforge_common::{Diagnostic, extension_entry_name, load_project_config};
use specforge_graph::Graph;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{
    KindRegistry, ManifestV2, ProviderStatus, load_provider_configurations,
    register_provider_schemes_with_status,
};
use specforge_wasm::read_lock_file;
use std::path::Path;

/// Whether an extension listed is part of the compiled project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Enabled in `specforge.json` and loaded.
    Loaded,
    /// Enabled but not loaded (not installed, or it failed to load).
    NotLoaded,
    /// Installed (locked) or loaded, but no longer enabled.
    NotConfigured,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Loaded => "loaded",
            Status::NotLoaded => "not_loaded",
            Status::NotConfigured => "not_configured",
        }
    }
}

/// One extension of the project: what it is and what it contributes.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionEntry {
    pub name: String,
    /// The loaded version, else the locked one.
    pub version: Option<String>,
    pub origin: Origin,
    pub status: Status,
    /// The entity kinds it contributes, sorted.
    pub entity_kinds: Vec<String>,
    /// How many of the project's entities are of those kinds.
    pub entity_count: usize,
    pub validation_rules: usize,
}

/// Every extension the project enables, has installed, or loaded, sorted
/// by name, with the entity kinds each registered (from the KindRegistry),
/// how many of the graph's entities use them, and its rule count.
pub fn list(
    root: &Path,
    loaded: &[ManifestV2],
    kinds: &KindRegistry,
    graph: &Graph,
) -> Vec<ExtensionEntry> {
    let entries = load_project_config(root).extensions;
    let enabled: Vec<String> = entries
        .iter()
        .map(|e| extension_entry_name(e).to_string())
        .collect();
    // The version a legacy `name@version` entry names.
    let configured_version = |name: &str| {
        entries.iter().find_map(|e| {
            let e = e.trim();
            (extension_entry_name(e) == name && e.len() > name.len())
                .then(|| e[name.len() + 1..].to_string())
        })
    };
    let lock = read_lock_file(&lock_path(root)).unwrap_or_default();

    let mut names: Vec<String> = enabled
        .iter()
        .cloned()
        .chain(lock.entries.iter().map(|e| e.name.clone()))
        .chain(loaded.iter().map(|m| m.name.clone()))
        .collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            let manifest = loaded.iter().find(|m| m.name == name);
            let locked = lock.entries.iter().find(|e| e.name == name);
            let status = match (enabled.contains(&name), manifest) {
                (true, Some(_)) => Status::Loaded,
                (true, None) => Status::NotLoaded,
                (false, _) => Status::NotConfigured,
            };
            let origin = match (builtin_name(&name), locked) {
                (_, Some(entry)) => Origin::Installed {
                    source: entry.source.clone(),
                },
                (Some(_), None) => Origin::Builtin,
                (None, None) => Origin::Installed {
                    source: "unknown".to_string(),
                },
            };
            let mut entity_kinds: Vec<String> = kinds
                .iter()
                .filter(|(_, entry)| entry.source_extension == name)
                .map(|(kind, _)| kind.clone())
                .collect();
            entity_kinds.sort();
            let entity_count = graph
                .nodes()
                .iter()
                .filter(|n| entity_kinds.iter().any(|k| k == n.kind.raw.as_str()))
                .count();
            ExtensionEntry {
                version: manifest
                    .map(|m| m.version.clone())
                    .or_else(|| locked.map(|e| e.version.clone()))
                    .or_else(|| configured_version(&name)),
                validation_rules: manifest.map_or(0, |m| m.validation_rules.len()),
                name,
                origin,
                status,
                entity_kinds,
                entity_count,
            }
        })
        .collect()
}

/// One configured provider, as the scheme registry sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderEntry {
    pub scheme: String,
    pub alias: String,
    pub extension: String,
    pub status: ProviderStatus,
}

/// The providers `specforge.json` configures, in declaration order (the
/// first to declare a scheme wins it), each with its status in the scheme
/// registry built from the loaded declarations, and the diagnostics
/// loading and registering them produced (W118, E057).
pub fn providers(
    root: &Path,
    loaded: &[ExtensionDeclaration],
) -> (Vec<ProviderEntry>, Vec<Diagnostic>) {
    let config = load_project_config(root)
        .raw
        .unwrap_or(serde_json::Value::Null);
    let (configs, mut diagnostics) = load_provider_configurations(&config);
    let (_, statuses, registration) = register_provider_schemes_with_status(&configs, loaded);
    diagnostics.extend(registration);
    let entries = configs
        .into_iter()
        .zip(statuses)
        .map(|(config, status)| ProviderEntry {
            scheme: config.scheme,
            alias: config.alias,
            extension: config.extension,
            status,
        })
        .collect();
    (entries, diagnostics)
}
