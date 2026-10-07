//! Extension management: the operations behind `specforge add`, `remove`,
//! `extensions` and their MCP tools.

mod add;
mod diamond;
mod list;
mod remove;
mod resolve;
mod update;

pub use add::{AddOutcome, AddRequest, Added, Source, Trust, add, declared, parse};
pub use diamond::check_diamonds;
pub use list::{
    ExtensionEntry, ExtensionListing, LockedExtension, ProviderEntry, ProviderListing, Status,
    list, providers,
};
pub use remove::{RemoveOutcome, RemoveRequest, remove};
pub use resolve::{resolve, resolve_requirement};
pub use update::{
    BatchUpdateCompleted, ExtensionUpdate, NO_LOCK, UpdateOutcome, UpdateRequest, UpdateStatus,
    update,
};

use crate::OpError;
use crate::registry::Registry;
use specforge_component::builtins::BUILTIN_EXTENSIONS;
use specforge_project::EnabledExtension;
use specforge_protocol_types::PackageName;
use specforge_protocol_types::package::Version;
use specforge_wasm::LockFile;
use std::path::{Path, PathBuf};

/// The versions a registry publishes of a peer, as the diamond gate asks
/// for them: a peer that is not a package name is E072.
pub(crate) fn published_versions(
    registry: &dyn Registry,
) -> impl Fn(&str) -> Result<Vec<String>, OpError> + '_ {
    move |peer| {
        let name = PackageName::parse(peer)
            .map_err(|why| OpError::from(specforge_common::package::invalid(&why)))?;
        Ok(registry
            .versions(&name)?
            .iter()
            .map(Version::to_string)
            .collect())
    }
}

/// The code an operation reports for an extension the project doesn't
/// have.
pub const NOT_FOUND: &str = "extension_not_found";

/// Where an extension comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Embedded in the binary; enabling it is only a config entry.
    Builtin,
    /// Installed under `.specforge/extensions/`, pinned in `specforge.lock`.
    /// `source` is the lock entry's (`registry`, `local:<path>`, ...).
    Installed { source: String },
    /// Loaded from the `.wasm` file a `specforge.json` entry names; `path`
    /// as the entry writes it.
    File { path: String },
}

impl Origin {
    /// Where `name` comes from in a project: the `.wasm` file an
    /// `extensions` entry names (over a lock entry), the lock entry's
    /// source, a builtin, else `Installed { source: "unknown" }`. The one
    /// rule `list` and `doctor` name sources by.
    pub fn of(name: &str, enabled: &[EnabledExtension], lock: Option<&LockFile>) -> Origin {
        if let Some(path) = enabled
            .iter()
            .find(|e| e.name == name)
            .and_then(|e| e.file.clone())
        {
            return Origin::File { path };
        }
        if let Some(entry) = lock.and_then(|lock| lock.entries.iter().find(|e| e.name == name)) {
            return Origin::Installed {
                source: entry.source.clone(),
            };
        }
        match builtin_name(name) {
            Some(_) => Origin::Builtin,
            None => Origin::Installed {
                source: "unknown".to_string(),
            },
        }
    }

    /// What the listings call it: `builtin`, the lock entry's source, or
    /// `file:<path>`.
    pub fn source(&self) -> String {
        match self {
            Origin::Builtin => "builtin".to_string(),
            Origin::Installed { source } => source.clone(),
            Origin::File { path } => format!("file:{path}"),
        }
    }
}

/// The builtin `specifier` names (`@specforge/product`, optionally with an
/// `@version` suffix, which is ignored: builtins track the binary).
pub fn builtin_name(specifier: &str) -> Option<&'static str> {
    let name = crate::config::entry_name(specifier);
    BUILTIN_EXTENSIONS
        .iter()
        .map(|(builtin, _)| *builtin)
        .find(|builtin| *builtin == name)
}

/// Builtins `config` enables, in declaration order.
pub fn enabled_builtins(config: &specforge_common::ProjectConfig) -> Vec<&'static str> {
    config
        .extensions
        .iter()
        .filter_map(|entry| builtin_name(entry))
        .collect()
}

/// Builtins that `name` requires: its non-optional peer dependencies that
/// are themselves builtins, read from its declaration.
pub fn required_builtin_peers(name: &str) -> Vec<&'static str> {
    let runtime = specforge_component::ComponentRuntime::new();
    if specforge_component::builtins::load_builtins_for(&runtime, &[name.to_string()]).is_err() {
        return Vec::new();
    }
    let Ok(loaded) = specforge_wasm::protocol::load_declaration(&runtime, name) else {
        return Vec::new();
    };
    loaded
        .declaration
        .peers()
        .iter()
        .filter(|peer| !peer.optional)
        .filter_map(|peer| builtin_name(&peer.name))
        .collect()
}

pub(crate) use specforge_wasm::lock_path;

/// `.specforge/extensions` at the project root.
pub(crate) fn extensions_dir(root: &Path) -> PathBuf {
    root.join(".specforge").join("extensions")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_name_accepts_bare_and_versioned_names() {
        assert_eq!(
            builtin_name("@specforge/product"),
            Some("@specforge/product")
        );
        assert_eq!(
            builtin_name("@specforge/formal@latest"),
            Some("@specforge/formal")
        );
        assert_eq!(builtin_name("@acme/thing"), None);
        assert_eq!(builtin_name("./local.wasm"), None);
    }
}
