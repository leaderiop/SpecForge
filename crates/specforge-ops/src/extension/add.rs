//! `specforge add` and `specforge.add_extension`.

use super::{Origin, builtin_name, check_diamonds, extensions_dir, lock_path};
use crate::OpError;
use crate::registry::Registry;
use specforge_wasm::{
    ExtensionSpecifier, install_extension, install_from_local, parse_extension_specifier,
    read_lock_file, write_lock_file,
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
/// - A local `.wasm` is copied under `.specforge/extensions/` and locked.
/// - A registry package is resolved, downloaded, integrity- and
///   signature-checked, checked against the ADR-0001 diamond gate,
///   installed, locked and enabled.
pub fn add(req: &AddRequest, registry: &dyn Registry) -> Result<AddOutcome, OpError> {
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
    if !path.exists() {
        return Err(OpError::new(
            "E054",
            format!("file not found: {}", path.display()),
        ));
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let origin = Origin::Installed {
        source: "local".to_string(),
    };
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name,
            version: Some("local".to_string()),
            origin,
        });
    }
    let mut lock = read_lock_file(&lock_path(req.root)).unwrap_or_default();
    let result = install_from_local(&name, "local", path, &extensions_dir(req.root), &mut lock)
        .map_err(OpError::from)?;
    write_lock_file(&lock, &lock_path(req.root)).map_err(OpError::from)?;
    Ok(AddOutcome::Installed {
        name: result.name,
        version: result.version,
        sha256: result.wasm_hash,
        key_id: None,
        origin,
    })
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
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: name.to_string(),
            version: Some(version),
            origin,
        });
    }
    let package = registry.fetch(name, &version)?;

    // Publisher signature and the TOFU pin policy (one implementation,
    // shared with every surface).
    let (assume_yes, format) = match req.trust {
        Trust::Refuse => (false, "json"),
        Trust::AssumeYes => (true, "human"),
        Trust::Prompt => (false, "human"),
    };
    let trust = specforge_registry::client::trust_flow::check_and_pin(
        &package.name,
        &package.response,
        &package.wasm,
        req.allow_unsigned,
        assume_yes,
        format,
        None,
    )
    .map_err(OpError::from)?;

    let mut lock = read_lock_file(&lock_path(req.root)).unwrap_or_default();
    check_diamonds(&lock, &package.name, &package.peers, &|peer| {
        registry.versions(peer)
    })?;

    let result = install_extension(
        &package.name,
        &package.version,
        &package.wasm,
        &package.sha256,
        &extensions_dir(req.root),
        &mut lock,
        trust.key_id.as_deref(),
        package.peers.clone(),
    )
    .map_err(OpError::from)?;
    write_lock_file(&lock, &lock_path(req.root)).map_err(OpError::from)?;

    let entry = format!("{}@{version}", package.name);
    // The install stands even when the config can't be edited: the
    // surface reports that it was not enabled.
    let _ = crate::config::add_extension(req.root, &package.name, &entry);
    Ok(AddOutcome::Installed {
        name: result.name,
        version: result.version,
        sha256: result.wasm_hash,
        key_id: trust.key_id,
        origin,
    })
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
