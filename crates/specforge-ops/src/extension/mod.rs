//! Extension management: the operations behind `specforge add`, `remove`,
//! `extensions` and their MCP tools.

mod add;
mod diamond;
mod list;
mod remove;

pub use add::{AddOutcome, AddRequest, Source, Trust, add, parse};
pub use diamond::check_diamonds;
pub use list::{ExtensionEntry, ProviderEntry, Status, list, providers};
pub use remove::{RemoveOutcome, RemoveRequest, remove};

use specforge_component::builtins::BUILTIN_EXTENSIONS;
use std::path::{Path, PathBuf};

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

/// Builtins enabled in the project's `specforge.json`, in declaration order.
pub fn enabled_builtins(root: &Path) -> Vec<&'static str> {
    specforge_common::load_project_config(root)
        .extensions
        .iter()
        .filter_map(|entry| builtin_name(entry))
        .collect()
}

/// Builtins that `name` requires: its non-optional peer dependencies that
/// are themselves builtins, read from its handshake.
pub fn required_builtin_peers(name: &str) -> Vec<&'static str> {
    let runtime = specforge_component::ComponentRuntime::new();
    if specforge_component::builtins::load_builtins_for(&runtime, &[name.to_string()]).is_err() {
        return Vec::new();
    }
    let host = specforge_wasm::protocol::ProtocolHost::new(&runtime);
    let Ok(handshake) = host.handshake(name) else {
        return Vec::new();
    };
    handshake
        .peer_dependencies
        .iter()
        .filter(|peer| !peer.optional)
        .filter_map(|peer| builtin_name(&peer.name))
        .collect()
}

/// `specforge.lock` at the project root.
pub(crate) fn lock_path(root: &Path) -> PathBuf {
    root.join("specforge.lock")
}

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
