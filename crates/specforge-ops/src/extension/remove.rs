//! `specforge remove` and `specforge.remove_extension`.

use super::{NOT_FOUND, Origin, builtin_name, extensions_dir, lock_path};
use crate::OpError;
use specforge_graph::Graph;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::KindRegistry;
use specforge_wasm::{LockFile, read_lock_file, uninstall_extension, write_lock_file};
use std::path::Path;

/// What to remove, and the compiled project it is removed from.
pub struct RemoveRequest<'a> {
    pub root: &'a Path,
    pub name: &'a str,
    /// Remove even when another extension requires it.
    pub force: bool,
    /// Report what would be removed; change nothing.
    pub dry_run: bool,
    /// The declarations a compile of the project loaded.
    pub loaded: &'a [ExtensionDeclaration],
    pub kinds: &'a KindRegistry,
    pub graph: &'a Graph,
}

/// What a removal did (or, on a dry run, would do).
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveOutcome {
    pub name: String,
    /// The locked version; for a builtin, the loaded one, if it loaded.
    pub version: Option<String>,
    pub origin: Origin,
    /// One per entity whose kind only the removed extension defines: those
    /// entities fail E024 on the next compile.
    pub orphan_warnings: Vec<String>,
    pub dry_run: bool,
}

/// Remove `name` from the project: a locked install is uninstalled
/// (binary, lock entry and `specforge.json` entry); a builtin is disabled
/// (its `specforge.json` entry). Refused with E027 while another loaded or
/// locked extension requires it as a non-optional peer, unless `force`.
pub fn remove(req: &RemoveRequest) -> Result<RemoveOutcome, OpError> {
    let lock = read_lock_file(&lock_path(req.root)).ok();
    let locked = lock
        .as_ref()
        .and_then(|lock| lock.entries.iter().find(|e| e.name == req.name));

    let (version, origin) = match (locked, builtin_name(req.name)) {
        (Some(entry), _) => (
            Some(entry.version.clone()),
            Origin::Installed {
                source: entry.source.clone(),
            },
        ),
        (None, Some(builtin)) => {
            if !enabled(req.root, builtin)? {
                return Err(not_installed(req.name, None));
            }
            let loaded = req.loaded.iter().find(|d| d.name() == builtin);
            (loaded.map(|d| d.version().to_string()), Origin::Builtin)
        }
        (None, None) => {
            let why = lock.is_none().then_some("no lock file found");
            return Err(not_installed(req.name, why));
        }
    };

    let dependents = dependents(req.name, req.loaded, lock.as_ref());
    if !dependents.is_empty() && !req.force {
        return Err(OpError::new(
            "E027",
            format!(
                "cannot uninstall '{}': required by {}",
                req.name,
                dependents.join(", ")
            ),
        )
        .with_suggestion("use --force to uninstall anyway, or remove dependent extensions first"));
    }

    let outcome = RemoveOutcome {
        name: req.name.to_string(),
        version,
        orphan_warnings: orphan_warnings(req.graph, req.kinds, req.name),
        dry_run: req.dry_run,
        origin,
    };
    if req.dry_run {
        return Ok(outcome);
    }

    if let (Origin::Installed { .. }, Some(mut lock)) = (&outcome.origin, lock) {
        // Dependents are checked above, over the loaded declarations and the lock.
        uninstall_extension(req.name, &extensions_dir(req.root), &mut lock)
            .map_err(OpError::from)?;
        write_lock_file(&lock, &lock_path(req.root)).map_err(OpError::from)?;
    }
    // A project without specforge.json (an install only the lock knows)
    // has no entry to drop.
    match crate::config::remove_extension(req.root, req.name) {
        Err(e) if e.code != "config_not_found" => Err(e),
        _ => Ok(outcome),
    }
}

/// Whether `specforge.json` enables `name`. No config: not enabled.
fn enabled(root: &Path, name: &str) -> Result<bool, OpError> {
    let mut found = false;
    match crate::config::edit_extensions(root, |extensions| {
        found = crate::config::has_extension(extensions, name);
        false
    }) {
        Ok(_) => Ok(found),
        Err(e) if e.code == "config_not_found" => Ok(false),
        Err(e) => Err(e),
    }
}

fn not_installed(name: &str, why: Option<&str>) -> OpError {
    let mut message = format!("extension '{name}' is not installed");
    if let Some(why) = why {
        message.push_str(&format!(" ({why})"));
    }
    OpError::new(NOT_FOUND, message)
}

/// The extensions that require `name` as a non-optional peer: loaded ones
/// (their handshake) and locked ones (the peers recorded at install).
fn dependents(name: &str, loaded: &[ExtensionDeclaration], lock: Option<&LockFile>) -> Vec<String> {
    let requires = |peers: &[specforge_registry::PeerDependency]| {
        peers.iter().any(|p| p.name == name && !p.optional)
    };
    let mut out: Vec<String> = loaded
        .iter()
        .filter(|d| d.name() != name && requires(d.peers()))
        .map(|d| d.name().to_string())
        .chain(
            lock.iter()
                .flat_map(|lock| &lock.entries)
                .filter(|e| e.name != name && requires(&e.peer_dependencies))
                .map(|e| e.name.clone()),
        )
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One warning per entity whose kind only `extension` defines.
fn orphan_warnings(graph: &Graph, kinds: &KindRegistry, extension: &str) -> Vec<String> {
    let mut warnings: Vec<String> = graph
        .nodes()
        .into_iter()
        .filter(|node| {
            kinds
                .get(node.kind.raw.as_str())
                .is_some_and(|kind| kind.source_extension == extension)
        })
        .map(|node| {
            format!(
                "{} '{}' uses a kind only {extension} defines",
                node.kind.raw, node.id.raw
            )
        })
        .collect();
    warnings.sort();
    warnings
}
