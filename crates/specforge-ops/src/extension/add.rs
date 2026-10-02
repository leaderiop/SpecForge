//! `specforge add` and `specforge.add_extension`.

use super::{Origin, builtin_name, check_diamonds, extensions_dir, lock_path};
use crate::OpError;
use crate::registry::Registry;
use specforge_wasm::{
    ExtensionSpecifier, install_extension, parse_extension_specifier, read_lock_file,
    write_lock_file,
};
use std::path::{Path, PathBuf};

/// Where an extension to add comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A builtin, embedded in the binary.
    Builtin(&'static str),
    /// A `.wasm` file on disk.
    Local(PathBuf),
    /// A registry package; `range` is `latest` when none was given.
    Registry { name: String, range: String },
    /// A git repository (not supported yet: E064).
    Git { url: String },
}

/// Parse an `add` specifier: a builtin name, a `.wasm` path (or any
/// `./`, `../` or `/` path), `@scope/name[@range]`, `name@range`, or
/// `git+<url>`. Anything else is E054.
pub fn parse(specifier: &str) -> Result<Source, OpError> {
    let specifier = specifier.trim();
    if let Some(builtin) = builtin_name(specifier) {
        return Ok(Source::Builtin(builtin));
    }
    if specifier.ends_with(".wasm") {
        return Ok(Source::Local(PathBuf::from(specifier)));
    }
    // `@scope/name` with no version resolves to the latest.
    if specifier.starts_with('@')
        && specifier.contains('/')
        && !specifier[1..].contains('@')
        && specifier
            .split('/')
            .all(|part| part.len() > 1 || part == "@")
    {
        return Ok(Source::Registry {
            name: specifier.to_string(),
            range: "latest".to_string(),
        });
    }
    match parse_extension_specifier(specifier).map_err(OpError::from)? {
        ExtensionSpecifier::Local { path } => Ok(Source::Local(path)),
        ExtensionSpecifier::Registry { name, version } => Ok(Source::Registry {
            name,
            range: version,
        }),
        ExtensionSpecifier::Git { url, .. } => Ok(Source::Git { url }),
    }
}

/// How a key change in a signed package is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Refuse it (MCP, `--format json`): nobody can be asked.
    Refuse,
    /// Accept it (`--yes`).
    AssumeYes,
    /// Ask on the terminal, refusing when there is none.
    Prompt,
}

/// What to add, and where.
#[derive(Debug, Clone)]
pub struct AddRequest<'a> {
    pub root: &'a Path,
    pub source: Source,
    /// Accept a registry package with no publisher signature.
    pub allow_unsigned: bool,
    pub trust: Trust,
    /// Resolve and report; write and download nothing.
    pub dry_run: bool,
}

/// What an add did (or, on a dry run, would do).
#[derive(Debug, Clone, PartialEq)]
pub enum AddOutcome {
    /// A builtin enabled; `changed` is false when it already was.
    /// `peers_enabled` are the required builtin peers enabled first.
    Builtin {
        name: &'static str,
        changed: bool,
        peers_enabled: Vec<&'static str>,
    },
    /// Installed under `.specforge/extensions/` and locked.
    Installed {
        name: String,
        version: String,
        sha256: String,
        key_id: Option<String>,
        origin: Origin,
    },
    /// Already installed at this version (or, from a local file, with
    /// these exact bytes) and enabled: nothing changed.
    AlreadyPresent { name: String, version: String },
    /// A dry run: what would be installed or enabled.
    Planned {
        name: String,
        version: Option<String>,
        origin: Origin,
    },
}

/// Add an extension to the project at `req.root`.
///
/// - A builtin is enabled in `specforge.json`, after the builtins it
///   requires as non-optional peers.
/// - A local `.wasm` is validated by its handshake, copied under
///   `.specforge/extensions/`, locked with the version it declares and
///   `source: "local:<path>"`, and enabled.
/// - A registry package is resolved, downloaded, integrity- and
///   signature-checked, validated by its handshake, checked against the
///   ADR-0001 diamond gate, installed, locked and enabled.
///
/// An installed extension is enabled by its bare name, which the runtime
/// loads from the lock (ADR 0004 D3-b).
pub fn add(req: &AddRequest, registry: &dyn Registry) -> Result<AddOutcome, OpError> {
    // The project must exist, with a config the writer can edit, before
    // anything is installed into it.
    crate::config::edit_extensions(req.root, |_| false).map_err(config_error)?;
    match &req.source {
        Source::Builtin(name) => add_builtin(req, name),
        Source::Local(path) => add_local(req, path),
        Source::Registry { name, range } => add_from_registry(req, registry, name, range),
        Source::Git { url } => Err(OpError::new(
            "E064",
            format!("git source '{url}' not yet supported"),
        )),
    }
}

fn add_builtin(req: &AddRequest, name: &'static str) -> Result<AddOutcome, OpError> {
    let enabled = super::enabled_builtins(req.root);
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: name.to_string(),
            version: None,
            origin: Origin::Builtin,
        });
    }
    // Required builtin peers come first, so they're enabled before the
    // extension that builds on them. An already-enabled extension is left
    // exactly as it is.
    let peers = if enabled.contains(&name) {
        Vec::new()
    } else {
        super::required_builtin_peers(name)
    };
    let mut peers_enabled = Vec::new();
    for peer in peers {
        if crate::config::add_extension(req.root, peer, peer).map_err(config_error)? {
            peers_enabled.push(peer);
        }
    }
    let changed = crate::config::add_extension(req.root, name, name).map_err(config_error)?;
    Ok(AddOutcome::Builtin {
        name,
        changed,
        peers_enabled,
    })
}

/// A config the writer can't edit, under the code `add` has always used.
fn config_error(e: OpError) -> OpError {
    let message = match &e.suggestion {
        Some(hint) => format!("{} — {hint}", e.message),
        None => e.message.clone(),
    };
    OpError::new("E032", message)
}

fn add_local(req: &AddRequest, path: &Path) -> Result<AddOutcome, OpError> {
    let wasm = std::fs::read(path).map_err(|e| {
        let message = if path.exists() {
            format!("cannot read {}: {e}", path.display())
        } else {
            format!("file not found: {}", path.display())
        };
        OpError::new("E054", message)
    })?;
    let declared = Declared::of(&wasm)?;
    let origin = Origin::Installed {
        source: format!("local:{}", shown_path(req.root, path)),
    };
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: declared.name,
            version: Some(declared.version),
            origin,
        });
    }
    let sha256 = specforge_wasm::hex_sha256(&wasm);
    let mut lock = read_lock_file(&lock_path(req.root)).unwrap_or_default();
    if let Some(present) = already_present(req.root, &lock, &declared.name, |e| {
        e.wasm_hash == sha256 && e.source.starts_with("local:")
    }) {
        return Ok(present);
    }
    install(
        req.root, &mut lock, &declared, &wasm, &sha256, None, &origin,
    )
}

fn add_from_registry(
    req: &AddRequest,
    registry: &dyn Registry,
    name: &str,
    range: &str,
) -> Result<AddOutcome, OpError> {
    let version = registry.resolve_version(name, range)?;
    let origin = Origin::Installed {
        source: "registry".to_string(),
    };
    let mut lock = read_lock_file(&lock_path(req.root)).unwrap_or_default();
    if let Some(present) = already_present(req.root, &lock, name, |e| {
        e.version == version && e.source == "registry"
    }) {
        return Ok(present);
    }
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: name.to_string(),
            version: Some(version),
            origin,
        });
    }
    let checked = fetch_checked(
        registry,
        &lock,
        name,
        &version,
        req.allow_unsigned,
        req.trust,
    )?;
    install(
        req.root,
        &mut lock,
        &checked.declared,
        &checked.package.wasm,
        &checked.package.sha256,
        checked.key_id,
        &origin,
    )
}

/// A registry package downloaded and checked as `add` checks it: its
/// integrity, its publisher signature under the TOFU pin policy, the
/// ADR-0001 diamond gate against `lock`, and that the binary is the
/// package it claims to be. Nothing is written but a trust pin.
pub(super) struct Checked {
    pub(super) package: crate::registry::Package,
    pub(super) declared: Declared,
    pub(super) key_id: Option<String>,
}

pub(super) fn fetch_checked(
    registry: &dyn Registry,
    lock: &specforge_wasm::LockFile,
    name: &str,
    version: &str,
    allow_unsigned: bool,
    trust: Trust,
) -> Result<Checked, OpError> {
    let package = registry.fetch(name, version)?;

    // Publisher signature and the TOFU pin policy (one implementation,
    // shared with every surface).
    let (assume_yes, format) = match trust {
        Trust::Refuse => (false, "json"),
        Trust::AssumeYes => (true, "human"),
        Trust::Prompt => (false, "human"),
    };
    let trusted = specforge_registry::client::trust_flow::check_and_pin(
        &package.name,
        &package.response,
        &package.wasm,
        allow_unsigned,
        assume_yes,
        format,
        None,
    )
    .map_err(OpError::from)?;

    // The peers the published manifest declares decide the diamond gate
    // before anything is loaded; the binary must then be the package it
    // claims to be.
    check_diamonds(lock, &package.name, &package.peers, &|peer| {
        registry.versions(peer)
    })?;
    let declared = Declared::of(&package.wasm)?;
    if declared.name != package.name || declared.version != package.version {
        return Err(OpError::new(
            "E028",
            format!(
                "registry package {}@{} declares itself {}@{}",
                package.name, package.version, declared.name, declared.version
            ),
        ));
    }
    Ok(Checked {
        package,
        declared,
        key_id: trusted.key_id,
    })
}

/// The name and version the extension binary at `path` declares, checked
/// as `add` checks it (loadable, not a builtin's name), without installing
/// it.
pub fn declared(path: &Path) -> Result<(String, String), OpError> {
    let wasm = std::fs::read(path).map_err(|e| {
        OpError::new(
            "E054",
            if path.exists() {
                format!("cannot read {}: {e}", path.display())
            } else {
                format!("file not found: {}", path.display())
            },
        )
    })?;
    let declared = Declared::of(&wasm)?;
    Ok((declared.name, declared.version))
}

/// What an extension binary's handshake declares.
pub(super) struct Declared {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) peers: Vec<specforge_registry::PeerDependency>,
}

impl Declared {
    /// Load `wasm` and read its handshake: a binary that isn't a loadable
    /// extension, or that claims a builtin's name, is refused.
    fn of(wasm: &[u8]) -> Result<Self, OpError> {
        const CANDIDATE: &str = "__candidate";
        let runtime = specforge_component::ComponentRuntime::new();
        let invalid = |why: String| {
            OpError::new("E028", format!("not a loadable SpecForge extension: {why}"))
                .with_suggestion("build it with specforge-extension-sdk for wasm32-wasip2")
        };
        runtime
            .load_module_bytes(CANDIDATE, wasm)
            .map_err(invalid)?;
        let handshake = specforge_wasm::protocol::ProtocolHost::new(&runtime)
            .handshake(CANDIDATE)
            .map_err(|e| invalid(e.to_string()))?;
        if super::builtin_name(&handshake.name).is_some() {
            return Err(OpError::new(
                "extension_conflict",
                format!(
                    "the extension declares the name of the builtin '{}'",
                    handshake.name
                ),
            )
            .with_suggestion(format!(
                "enable the builtin instead: specforge add {}",
                handshake.name
            )));
        }
        Ok(Declared {
            name: handshake.name,
            version: handshake.version,
            peers: handshake
                .peer_dependencies
                .into_iter()
                .map(|p| specforge_registry::PeerDependency {
                    name: p.name,
                    version: p.version,
                    optional: p.optional,
                })
                .collect(),
        })
    }
}

/// `AlreadyPresent` when the lock holds `name` as `same` accepts, its
/// binary is in place and `specforge.json` enables it.
fn already_present(
    root: &Path,
    lock: &specforge_wasm::LockFile,
    name: &str,
    same: impl Fn(&specforge_wasm::LockFileEntry) -> bool,
) -> Option<AddOutcome> {
    let entry = lock.entries.iter().find(|e| e.name == name && same(e))?;
    let installed = specforge_wasm::installed_wasm_path(&extensions_dir(root), name).is_file();
    let enabled = specforge_common::load_project_config(root)
        .extensions
        .iter()
        .any(|e| specforge_common::extension_entry_name(e) == name);
    (installed && enabled).then(|| AddOutcome::AlreadyPresent {
        name: name.to_string(),
        version: entry.version.clone(),
    })
}

/// Place the binary, lock it as `origin` with its declared version and
/// peers, and enable it by its bare name.
fn install(
    root: &Path,
    lock: &mut specforge_wasm::LockFile,
    declared: &Declared,
    wasm: &[u8],
    sha256: &str,
    key_id: Option<String>,
    origin: &Origin,
) -> Result<AddOutcome, OpError> {
    let result = place(
        root,
        lock,
        declared,
        wasm,
        sha256,
        key_id.as_deref(),
        origin,
    )?;
    write_lock_file(lock, &lock_path(root)).map_err(OpError::from)?;
    crate::config::add_extension(root, &declared.name, &declared.name).map_err(config_error)?;
    Ok(AddOutcome::Installed {
        name: result.name,
        version: result.version,
        sha256: result.wasm_hash,
        key_id,
        origin: origin.clone(),
    })
}

/// Place the binary under `.specforge/extensions/` and record it in `lock`
/// (in memory) as `origin`, with its declared version and peers.
pub(super) fn place(
    root: &Path,
    lock: &mut specforge_wasm::LockFile,
    declared: &Declared,
    wasm: &[u8],
    sha256: &str,
    key_id: Option<&str>,
    origin: &Origin,
) -> Result<specforge_wasm::InstallResult, OpError> {
    let result = install_extension(
        &declared.name,
        &declared.version,
        wasm,
        sha256,
        &extensions_dir(root),
        lock,
        key_id,
        declared.peers.clone(),
    )
    .map_err(OpError::from)?;
    if let Some(entry) = lock.entries.iter_mut().find(|e| e.name == declared.name) {
        if let Origin::Installed { source } = origin {
            entry.source = source.clone();
        }
        entry.peer_dependencies = declared.peers.clone();
    }
    Ok(result)
}

/// `path` as the lock records it: relative to the project root when it
/// is under it, absolute otherwise.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_names_each_source() {
        assert_eq!(
            parse("@specforge/product"),
            Ok(Source::Builtin("@specforge/product"))
        );
        assert_eq!(
            parse("ext/greet.wasm"),
            Ok(Source::Local(PathBuf::from("ext/greet.wasm")))
        );
        assert_eq!(
            parse("./ext/dir"),
            Ok(Source::Local(PathBuf::from("./ext/dir")))
        );
        assert_eq!(
            parse("@acme/tool@^1.2"),
            Ok(Source::Registry {
                name: "@acme/tool".into(),
                range: "^1.2".into()
            })
        );
        assert_eq!(
            parse("git+https://example.com/x.git"),
            Ok(Source::Git {
                url: "https://example.com/x.git".into()
            })
        );
        assert_eq!(parse("").unwrap_err().code, "E054");
        assert_eq!(parse("not-scoped").unwrap_err().code, "E054");
        assert_eq!(parse("@acme/").unwrap_err().code, "E054");
    }
}
