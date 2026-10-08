//! `specforge add` and `specforge.add_extension`.

use super::diamond::{Published, broken_requirers};
use super::{Origin, builtin_name, check_diamonds};
use crate::registry::Registry;
use crate::{OpError, OpErrorKind, Writes};
use specforge_common::codes;
use specforge_installed::{Change, Installed, LockSource, Module, Pin};
use specforge_protocol_types::package::{SpecifierError, Version};
use specforge_protocol_types::{ExtensionDeclaration, PackageName, PackageRef};
use std::path::{Path, PathBuf};

/// Where an extension to add comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A builtin, embedded in the binary.
    Builtin(&'static str),
    /// A `.wasm` file on disk.
    Local(PathBuf),
    /// A registry package and the version asked for (`latest` when none).
    Registry(PackageRef),
    /// A git repository (not supported yet: E064).
    Git { url: String },
}

/// Read an `add` argument once (ADR 0036): a builtin name, a `.wasm` path
/// (or any `./`, `../` or `/` path), a `git+<url>`, or a package reference
/// `@scope/name[@requirement]`. Anything else is E054; a requirement that
/// is none is R-RES-003. No registry is asked for either.
pub fn parse(specifier: &str) -> Result<Source, OpError> {
    let specifier = specifier.trim();
    if specifier.is_empty() {
        return Err(
            OpError::diagnostic(codes::E054, "empty extension specifier")
                .with_suggestion(SPECIFIER_FORMS),
        );
    }
    if let Some(builtin) = builtin_name(specifier) {
        return Ok(Source::Builtin(builtin));
    }
    if let Some(url) = specifier.strip_prefix("git+") {
        // A `#rev` names a revision; a git source is not installable yet.
        let url = url.rfind('#').map_or(url, |hash| &url[..hash]);
        return Ok(Source::Git {
            url: url.to_string(),
        });
    }
    if specifier.ends_with(".wasm")
        || ["./", "../", "/"]
            .iter()
            .any(|prefix| specifier.starts_with(prefix))
    {
        return Ok(Source::Local(PathBuf::from(specifier)));
    }
    Ok(Source::Registry(PackageRef::parse(specifier)?))
}

/// What an `add` argument may be, as the suggestion of E054.
const SPECIFIER_FORMS: &str =
    "use a builtin's name, './local/path.wasm', 'git+https://...', or '@scope/name[@version]'";

/// E054 for a package reference that is not one, R-RES-003 for a
/// requirement that is none.
impl From<SpecifierError> for OpError {
    fn from(error: SpecifierError) -> Self {
        match error {
            SpecifierError::Requirement(_) => {
                OpError::diagnostic(codes::R_RES_003, error.to_string()).with_suggestion(
                    "use a version such as 1.2.0, a requirement such as ^1.0, ~2.3, 1.x or \
                     >=1.0.0, <2.0.0, or latest",
                )
            }
            _ => OpError::diagnostic(codes::E054, format!("invalid extension specifier: {error}"))
                .with_suggestion(SPECIFIER_FORMS),
        }
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

/// What an add did, and what it changed on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct Added {
    pub outcome: AddOutcome,
    /// The files the add changed: `specforge.json` when an entry was added,
    /// and for an install the module and `specforge.lock` when their bytes
    /// changed. Nothing for a dry run or an extension already present. A
    /// trust pin the registry adapter keeps for the user is not one.
    pub writes: Writes,
    /// How many extensions `specforge.json` enables after the add (`0`
    /// without one): `extension_added`'s `totalExtensions`.
    pub extensions_enabled: usize,
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
///
/// An install is all or nothing: the module, its lock entry and the
/// `specforge.json` entry are written as one change, and a failure at any
/// step puts every file back (the error's [`OpError::writes`] names what the
/// rollback could not, normally nothing).
pub fn add(req: &AddRequest, registry: &dyn Registry) -> Result<Added, OpError> {
    // The project must exist, with a config the writer can edit, before
    // anything is installed into it: the refusal `update` and `remove`
    // give an unusable specforge.json too (`config_invalid`).
    let config = crate::config::required(req.root)?.config;
    let mut writes = Writes::none();
    let outcome = match &req.source {
        Source::Builtin(name) => add_builtin(req, &config, name, &mut writes),
        Source::Local(path) => add_local(req, path, &mut writes),
        Source::Registry(package) => add_from_registry(req, registry, package, &mut writes),
        Source::Git { url } => Err(OpError::diagnostic(
            codes::E064,
            format!("git source '{url}' not yet supported"),
        )),
    }?;
    Ok(Added {
        outcome,
        writes,
        extensions_enabled: specforge_common::read_project_config(req.root)
            .config
            .extensions
            .len(),
    })
}

fn add_builtin(
    req: &AddRequest,
    config: &specforge_common::ProjectConfig,
    name: &'static str,
    writes: &mut Writes,
) -> Result<AddOutcome, OpError> {
    let enabled = super::enabled_builtins(config);
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
    let config = req.root.join(crate::config::CONFIG_FILE);
    for peer in peers {
        let added = crate::config::add_extension(req.root, peer, peer)
            .map_err(|e| e.with_writes(writes.clone()))?;
        writes.record_if(added, &config);
        if added {
            peers_enabled.push(peer);
        }
    }
    let changed = crate::config::add_extension(req.root, name, name)
        .map_err(|e| e.with_writes(writes.clone()))?;
    writes.record_if(changed, &config);
    Ok(AddOutcome::Builtin {
        name,
        changed,
        peers_enabled,
    })
}

fn add_local(req: &AddRequest, path: &Path, writes: &mut Writes) -> Result<AddOutcome, OpError> {
    // A specforge.lock that can't be read refuses before anything is read.
    let installed = Installed::at(req.root);
    let change = installed.change()?;
    let wasm = std::fs::read(path).map_err(|e| {
        let message = if path.exists() {
            format!("cannot read {}: {e}", path.display())
        } else {
            format!("file not found: {}", path.display())
        };
        OpError::diagnostic(codes::E054, message)
    })?;
    let declared = Declared::of(&wasm)?;
    let package = declared.package()?;
    let origin = Origin::Installed {
        source: format!("local:{}", shown_path(req.root, path)),
    };
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: declared.name().to_string(),
            version: Some(declared.version().to_string()),
            origin,
        });
    }
    let module = Module::new(wasm);
    if let Some(present) = already_present(&installed, &package, |e| {
        e.wasm_hash == module.digest() && e.source.local_path().is_some()
    }) {
        return Ok(present);
    }
    // No registry to unify a diamond against: a locked peer outside the
    // range is E027 (ADR 0041).
    check_diamonds(change.lock(), declared.name(), declared.peers(), None)?;
    install(
        req.root, change, &declared, module, None, &origin, None, writes,
    )
}

fn add_from_registry(
    req: &AddRequest,
    registry: &dyn Registry,
    package: &PackageRef,
    writes: &mut Writes,
) -> Result<AddOutcome, OpError> {
    // A specforge.lock that can't be read refuses before a registry is asked.
    let installed = Installed::at(req.root);
    let change = installed.change()?;
    let name = package.name.as_str();
    let version = super::resolve(registry, package)?;
    let origin = Origin::Installed {
        source: "registry".to_string(),
    };
    if let Some(present) = already_present(&installed, &package.name, |e| {
        e.version == version.to_string() && e.source.is_registry()
    }) {
        return Ok(present);
    }
    if req.dry_run {
        return Ok(AddOutcome::Planned {
            name: name.to_string(),
            version: Some(version.to_string()),
            origin,
        });
    }
    let checked = fetch_checked(
        registry,
        change.lock(),
        &package.name,
        &version,
        req.allow_unsigned,
        req.trust,
    )?;
    install(
        req.root,
        change,
        &checked.declared,
        Module::new(checked.package.wasm),
        checked.package.key_id,
        &origin,
        Some(&super::published_versions(registry)),
        writes,
    )
}

/// A registry package downloaded and checked as `add` checks it: its
/// integrity, its publisher signature under the TOFU pin policy, the
/// ADR-0001 diamond gate against `lock`, and that the binary is the
/// package it claims to be. Nothing is written but a trust pin.
pub(super) struct Checked {
    pub(super) package: crate::registry::Package,
    pub(super) declared: Declared,
}

pub(super) fn fetch_checked(
    registry: &dyn Registry,
    lock: &specforge_installed::LockFile,
    name: &PackageName,
    version: &Version,
    allow_unsigned: bool,
    trust: Trust,
) -> Result<Checked, OpError> {
    // Integrity, then the publisher signature under the TOFU pin policy:
    // the registry adapter's (one implementation, shared with every surface).
    let package = registry.fetch(name, version, allow_unsigned, trust)?;

    // The peers the published declaration declares decide the diamond gate
    // before anything is loaded (ADR 0001); the binary must then be the
    // package it claims to be, and declare exactly what was published
    // (ADR 0012).
    check_diamonds(
        lock,
        package.name.as_str(),
        package.declaration.peers(),
        Some(&super::published_versions(registry)),
    )?;
    let declared = Declared::of(&package.wasm)?;
    if declared.name() != package.name.as_str() || declared.version() != package.version.to_string()
    {
        return Err(OpError::diagnostic(
            codes::E028,
            format!(
                "registry package {}@{} declares itself {}@{}",
                package.name,
                package.version,
                declared.name(),
                declared.version()
            ),
        ));
    }
    if let Some(category) = first_difference(&package.declaration, &declared.declaration) {
        return Err(OpError::coded(
            OpErrorKind::SchemaMismatch,
            crate::registry::METADATA_MISMATCH,
            format!(
                "registry package {}@{} declares another {category} than the binary it serves",
                package.name, package.version
            ),
        )
        .with_suggestion("don't install the package, and check the registry"));
    }
    Ok(Checked { package, declared })
}

/// The name and version the extension binary at `path` declares, checked
/// as `add` checks it (loadable, not a builtin's name), without installing
/// it.
pub fn declared(path: &Path) -> Result<(String, String), OpError> {
    let wasm = std::fs::read(path).map_err(|e| {
        OpError::diagnostic(
            codes::E054,
            if path.exists() {
                format!("cannot read {}: {e}", path.display())
            } else {
                format!("file not found: {}", path.display())
            },
        )
    })?;
    let declared = Declared::of(&wasm)?;
    Ok((declared.name().to_string(), declared.version().to_string()))
}

/// What an extension binary declares: its whole declaration, loaded as
/// every environment loads it.
pub(super) struct Declared {
    pub(super) declaration: ExtensionDeclaration,
}

impl Declared {
    pub(super) fn name(&self) -> &str {
        self.declaration.name()
    }

    pub(super) fn version(&self) -> &str {
        self.declaration.version()
    }

    /// The declared name as the package name the extension installs under:
    /// E072 when it is none, before anything is written (ADR 0036).
    pub(super) fn package(&self) -> Result<PackageName, OpError> {
        self.declaration
            .package_name()
            .map_err(|why| OpError::from(specforge_common::package::invalid(&why)))
    }

    pub(super) fn peers(&self) -> &[specforge_registry::PeerDependency] {
        self.declaration.peers()
    }

    /// Load `wasm` and read its declaration: a binary that isn't a loadable
    /// extension, or that claims a builtin's name, is refused.
    fn of(wasm: &[u8]) -> Result<Self, OpError> {
        let runtime = specforge_component::ComponentRuntime::new();
        let declaration =
            specforge_installed::declaration_of(&Module::new(wasm.to_vec()), &runtime)?.declaration;
        if super::builtin_name(declaration.name()).is_some() {
            return Err(OpError::new(
                OpErrorKind::Conflict,
                "extension_conflict",
                format!(
                    "the extension declares the name of the builtin '{}'",
                    declaration.name()
                ),
            )
            .with_suggestion(format!(
                "enable the builtin instead: specforge add {}",
                declaration.name()
            )));
        }
        Ok(Declared { declaration })
    }
}

/// The first part of a declaration that differs between `served` and
/// `binary` (`handshake`, or a describe category), or `None` when they are
/// equal.
fn first_difference(
    served: &ExtensionDeclaration,
    binary: &ExtensionDeclaration,
) -> Option<&'static str> {
    if served.handshake != binary.handshake {
        return Some("handshake");
    }
    specforge_protocol_types::DECLARED_CATEGORIES
        .iter()
        .copied()
        .find(|category| served.describe_items(category) != binary.describe_items(category))
}

/// `AlreadyPresent` when the lock holds `name` as `same` accepts, its
/// binary is in place and is the one the lock pins, and `specforge.json`
/// enables it. A binary that changed after install is not present: adding
/// it again reinstalls it.
fn already_present(
    installed: &Installed,
    name: &PackageName,
    same: impl Fn(&specforge_installed::LockFileEntry) -> bool,
) -> Option<AddOutcome> {
    let entry = installed.verified(name.as_str()).filter(|e| same(e))?;
    let enabled = specforge_common::load_project_config(installed.root())
        .extensions
        .iter()
        .any(|e| specforge_common::extension_entry_name(e) == name.as_str());
    enabled.then(|| AddOutcome::AlreadyPresent {
        name: name.to_string(),
        version: entry.version.clone(),
    })
}

/// Install `module` as `declared`: its binary, its lock entry (as `origin`,
/// with its declared version and peers) and its `specforge.json` entry (its
/// bare name), as one change that puts everything back when a step fails.
/// A locked extension the new version leaves with an unsatisfied peer
/// refuses the install before anything is written (ADR 0041); `published` is
/// what unifies that diamond, `None` for a local install.
#[allow(clippy::too_many_arguments)]
fn install(
    root: &Path,
    mut change: Change<'_>,
    declared: &Declared,
    module: Module,
    key_id: Option<String>,
    origin: &Origin,
    published: Published<'_>,
    writes: &mut Writes,
) -> Result<AddOutcome, OpError> {
    let package = declared.package()?;
    let sha256 = module.digest().to_string();
    change.install(
        module,
        Pin {
            name: package,
            version: declared.version().to_string(),
            source: LockSource::parse(&origin.source()),
            key_id: key_id.clone(),
            peers: declared.peers().to_vec(),
        },
    );
    let name = declared.name();
    if let Some((dependent, error)) = broken_requirers(change.lock(), name, &[name], published)
        .into_iter()
        .next()
    {
        return Err(error.prefixed(format!(
            "installing '{name}' {} breaks '{dependent}': ",
            declared.version()
        )));
    }
    let committed = change
        .commit_with(&root.join(crate::config::CONFIG_FILE), || {
            crate::config::add_extension(root, declared.name(), declared.name())
        })
        .map_err(|failed| failed.error.with_writes(Writes::of(failed.left)))?;
    *writes = Writes::of(committed.changed);
    Ok(AddOutcome::Installed {
        name: declared.name().to_string(),
        version: declared.version().to_string(),
        sha256,
        key_id,
        origin: origin.clone(),
    })
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
    use specforge_test_macros::test as specforge_test;

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
            Ok(Source::Registry(
                PackageRef::parse("@acme/tool@^1.2").unwrap()
            ))
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

    /// What `parse` makes of every input of plan 12's table (§2.2): a
    /// package reference, or the code of the refusal. Nothing reaches a
    /// registry that is not a scoped package name with a requirement.
    #[specforge_test(
        behavior = "parse_extension_specifier",
        verify = "each add argument reads as one extension source"
    )]
    fn parse_reads_each_input() {
        fn registry(reference: &str) -> Result<Source, String> {
            Ok(Source::Registry(PackageRef::parse(reference).unwrap()))
        }
        let cases: Vec<(&str, Result<Source, String>)> = vec![
            ("@acme/tool", registry("@acme/tool")),                 // I1
            ("@acme/tool@", Err("E054".into())),                    // I2
            ("@acme/tool@1.2.0", registry("@acme/tool@1.2.0")),     // I3
            ("@acme/tool@^1.2", registry("@acme/tool@^1.2")),       // I4
            ("@acme/tool@1.x", registry("@acme/tool@1.x")),         // I5
            ("@acme/tool@1.2", registry("@acme/tool@1.2")),         // I6
            ("@acme/tool@1.0.0/x", Err("R-RES-003".into())),        // I7
            ("@acme/tool@1.0.0?x=1", Err("R-RES-003".into())),      // I8
            ("foo@/bar", Err("E054".into())),                       // I9
            ("tool@1.0.0", Err("E054".into())),                     // I10
            ("tool", Err("E054".into())),                           // I11
            ("@acme/..", Err("E054".into())),                       // I12
            ("@acme/aa/bb", Err("E054".into())),                    // I13
            ("@acme/a/b", Err("E054".into())),                      // I14
            ("@a/x", registry("@a/x")),                             // I15
            ("@acme/T ool", Err("E054".into())),                    // I16
            ("Acme@1", Err("E054".into())),                         // I17
            ("@acme/tool@latest", registry("@acme/tool")),          // I18
            ("@acme/tool@*", registry("@acme/tool")),               // I18
            ("@acme/tool@>=1, <2", registry("@acme/tool@>=1, <2")), // I19
            ("@acme/tool@^bogus", Err("R-RES-003".into())),         // I20
            (
                "@acme/tool@2.0.0+build.1",
                registry("@acme/tool@2.0.0+build.1"),
            ), // I21
            ("@scope", Err("E054".into())),                         // I22
            (
                "@specforge/software",
                Ok(Source::Builtin("@specforge/software")),
            ), // I24
            (
                "git+https://h/r#v",
                Ok(Source::Git {
                    url: "https://h/r".into(),
                }),
            ), // I25
            (" @acme/tool ", registry("@acme/tool")),               // I26
        ];
        for (input, want) in cases {
            let got = parse(input).map_err(|error| error.code.to_string());
            assert_eq!(got, want, "{input:?}");
        }
    }

    #[specforge_test(
        behavior = "management_operations_over_the_project_view",
        verify = "add, update and remove refuse an unusable specforge.json with one refusal, before they write"
    )]
    fn an_unusable_config_is_refused_before_anything_is_installed() {
        use crate::config::testing::{UNUSABLE, files_under};

        for config in UNUSABLE {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("specforge.json"), config).unwrap();
            let before = files_under(dir.path());
            let read = specforge_common::read_project_config(dir.path());
            let refused = crate::config::refusal(&read.problems[0]);

            for source in [
                Source::Builtin("@specforge/product"),
                // A file that is not there: the config is refused first.
                Source::Local(dir.path().join("missing.wasm")),
                Source::Registry(PackageRef::parse("@acme/tool").unwrap()),
            ] {
                for dry_run in [false, true] {
                    let request = AddRequest {
                        root: dir.path(),
                        source: source.clone(),
                        allow_unsigned: false,
                        trust: Trust::Refuse,
                        dry_run,
                    };
                    let error = add(&request, &crate::registry::Unconfigured("add")).unwrap_err();

                    assert_eq!(error, refused, "{config}: {source:?}");
                    assert_eq!(files_under(dir.path()), before, "{config}: {source:?}");
                }
            }
        }
    }

    /// `@sdk/greet` 0.1.0 as the build vendors it.
    fn greet_wasm() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/greet-extension/greet.wasm")
    }

    #[specforge_test(
        behavior = "install_wasm_extension",
        verify = "an install of an extension whose binary changed after install replaces it"
    )]
    fn add_reinstalls_a_binary_that_changed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name": "p", "version": "0.1.0", "extensions": []}"#,
        )
        .unwrap();
        let request = AddRequest {
            root: dir.path(),
            source: Source::Local(greet_wasm()),
            allow_unsigned: false,
            trust: Trust::Refuse,
            dry_run: false,
        };
        let unconfigured = crate::registry::Unconfigured("add");
        add(&request, &unconfigured).unwrap();
        let module = dir
            .path()
            .join(".specforge/extensions/@sdk/greet/extension.wasm");
        let pinned = std::fs::read(&module).unwrap();
        let mut changed = pinned.clone();
        changed.extend_from_slice(b"changed after install");
        std::fs::write(&module, &changed).unwrap();

        let added = add(&request, &unconfigured).unwrap();

        assert!(
            matches!(added.outcome, AddOutcome::Installed { .. }),
            "{:?}",
            added.outcome
        );
        assert_eq!(std::fs::read(&module).unwrap(), pinned);
        assert_eq!(added.writes.len(), 1, "only the module differed");

        // Now that it is the pinned binary again, adding it is a no-op.
        let again = add(&request, &unconfigured).unwrap();
        assert!(matches!(again.outcome, AddOutcome::AlreadyPresent { .. }));
    }

    /// A project directory whose `specforge.json` holds `config` verbatim.
    fn project_with(config: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("specforge.json"), config).unwrap();
        dir
    }

    const EMPTY_PROJECT: &str = r#"{"name": "p", "version": "0.1.0", "extensions": []}"#;

    fn local_request<'a>(root: &'a Path, file: &Path) -> AddRequest<'a> {
        AddRequest {
            root,
            source: Source::Local(file.to_path_buf()),
            allow_unsigned: false,
            trust: Trust::Refuse,
            dry_run: false,
        }
    }

    fn builtin_request<'a>(root: &'a Path, name: &'static str) -> AddRequest<'a> {
        AddRequest {
            root,
            source: Source::Builtin(name),
            allow_unsigned: false,
            trust: Trust::Refuse,
            dry_run: false,
        }
    }

    /// The embedded bytes of the builtin `name`.
    fn builtin_bytes(name: &str) -> &'static [u8] {
        specforge_component::builtins::BUILTIN_EXTENSIONS
            .iter()
            .find(|(builtin, _)| *builtin == name)
            .map(|(_, bytes)| *bytes)
            .expect("a builtin")
    }

    #[test]
    fn add_refuses_a_binary_that_claims_a_builtins_name() {
        use crate::config::testing::files_under;
        let dir = project_with(EMPTY_PROJECT);
        let file = tempfile::tempdir().unwrap();
        let wasm = file.path().join("product.wasm");
        std::fs::write(&wasm, builtin_bytes("@specforge/product")).unwrap();
        let before = files_under(dir.path());

        let error = add(
            &local_request(dir.path(), &wasm),
            &crate::registry::Unconfigured("add"),
        )
        .unwrap_err();

        assert_eq!(error.code, "extension_conflict");
        assert_eq!(error.kind, OpErrorKind::Conflict);
        assert_eq!(
            error.message,
            "the extension declares the name of the builtin '@specforge/product'"
        );
        assert_eq!(
            error.suggestion.as_deref(),
            Some("enable the builtin instead: specforge add @specforge/product")
        );
        assert_eq!(files_under(dir.path()), before);
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "add unresolvable extension rejects with diagnostic"
    )]
    fn a_local_file_add_cannot_read_is_e054() {
        use crate::config::testing::files_under;
        let dir = project_with(EMPTY_PROJECT);
        let registry = crate::registry::Unconfigured("add");

        let before = files_under(dir.path());
        let missing = dir.path().join("missing.wasm");
        let error = add(&local_request(dir.path(), &missing), &registry).unwrap_err();
        assert_eq!(error.code, "E054");
        assert_eq!(
            error.message,
            format!("file not found: {}", missing.display())
        );
        assert_eq!(files_under(dir.path()), before);

        let scratch = tempfile::tempdir().unwrap();
        let directory = scratch.path().join("a-directory");
        std::fs::create_dir(&directory).unwrap();
        let error = add(&local_request(dir.path(), &directory), &registry).unwrap_err();
        assert_eq!(error.code, "E054");
        assert!(
            error
                .message
                .starts_with(&format!("cannot read {}: ", directory.display())),
            "{}",
            error.message
        );
        assert_eq!(files_under(dir.path()), before);
    }

    #[test]
    fn a_local_file_that_is_no_extension_is_e028() {
        use crate::config::testing::files_under;
        let dir = project_with(EMPTY_PROJECT);
        let file = tempfile::tempdir().unwrap();
        let wasm = file.path().join("junk.wasm");
        std::fs::write(&wasm, b"not wasm").unwrap();
        let before = files_under(dir.path());

        let error = add(
            &local_request(dir.path(), &wasm),
            &crate::registry::Unconfigured("add"),
        )
        .unwrap_err();

        assert_eq!(error.code, "E028");
        assert!(
            error
                .message
                .starts_with("not a loadable SpecForge extension:"),
            "{}",
            error.message
        );
        assert!(
            error
                .suggestion
                .as_deref()
                .unwrap()
                .contains("specforge-extension-sdk")
        );
        assert_eq!(files_under(dir.path()), before);
    }

    #[specforge_test(
        behavior = "install_wasm_extension",
        verify = "an install over an unreadable specforge.lock is refused before anything is written"
    )]
    fn an_unreadable_lock_refuses_before_the_local_file_is_read() {
        let dir = project_with(EMPTY_PROJECT);
        std::fs::write(dir.path().join("specforge.lock"), "not a lock {{{").unwrap();
        let missing = dir.path().join("missing.wasm");

        let error = add(
            &local_request(dir.path(), &missing),
            &crate::registry::Unconfigured("add"),
        )
        .unwrap_err();

        assert_eq!(error.code, "E033");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap(),
            "not a lock {{{"
        );
    }

    #[specforge_test(
        behavior = "add_extension_to_existing_project",
        verify = "add enables a builtin's required peers but not its optional ones"
    )]
    fn a_builtin_is_enabled_after_its_required_builtin_peers() {
        let dir = project_with(EMPTY_PROJECT);

        let added = add(
            &builtin_request(dir.path(), "@specforge/cargo-test"),
            &crate::registry::Unconfigured("add"),
        )
        .unwrap();

        assert_eq!(
            added.outcome,
            AddOutcome::Builtin {
                name: "@specforge/cargo-test",
                changed: true,
                peers_enabled: vec!["@specforge/testing"],
            }
        );
        assert_eq!(added.writes.names_under(dir.path()), ["specforge.json"]);
        assert_eq!(
            specforge_common::read_project_config(dir.path())
                .config
                .extensions,
            ["@specforge/testing", "@specforge/cargo-test"]
        );
        assert_eq!(added.extensions_enabled, 2);
    }

    #[test]
    fn extensions_enabled_counts_the_string_entries_after_the_add() {
        let dir =
            project_with(r#"{"name":"p","version":"0.1.0","extensions":[1,"@specforge/rust"]}"#);
        let registry = crate::registry::Unconfigured("add");
        let request = builtin_request(dir.path(), "@specforge/product");

        let added = add(&request, &registry).unwrap();
        assert_eq!(added.extensions_enabled, 2);

        let again = add(&request, &registry).unwrap();
        assert!(matches!(
            again.outcome,
            AddOutcome::Builtin { changed: false, .. }
        ));
        assert!(again.writes.is_empty());
        assert_eq!(again.extensions_enabled, 2);
    }

    #[test]
    fn a_missing_config_is_config_not_found_with_the_hint_to_init() {
        let dir = tempfile::tempdir().unwrap();
        let request = AddRequest {
            root: dir.path(),
            source: Source::Builtin("@specforge/product"),
            allow_unsigned: false,
            trust: Trust::Refuse,
            dry_run: false,
        };

        let error = add(&request, &crate::registry::Unconfigured("add")).unwrap_err();

        assert_eq!(error.code, "config_not_found");
        assert!(error.suggestion.unwrap().contains("specforge init"));
        assert!(!dir.path().join("specforge.json").exists());
    }
}
