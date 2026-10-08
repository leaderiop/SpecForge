//! `specforge init` and `specforge.init`: one scaffold for both surfaces.
//!
//! [`plan`] validates everything and builds the files in memory; only
//! [`apply`] writes. `init` enables builtins, and installs local `.wasm`
//! extensions through the shared add operation (ADR 0004 D3-e): it never
//! writes an entry `specforge check` can't load. A registry extension needs
//! a configured registry, which a project that doesn't exist yet can't
//! have, so it is added afterwards with `specforge add`.

use crate::extension::{self, Candidate, LocalFile, Source};
use crate::{OpError, OpErrorKind, Writes};
use serde_json::{Value, json};
use specforge_common::validate_project_name;
use specforge_wasm::WasmRuntime;
use std::path::{Path, PathBuf};

/// The code an init refused with because the target is already a project,
/// or is inside the one that forbids it.
pub const PROJECT_EXISTS: &str = "project_exists";
/// The code for a project name init can't use.
pub const INVALID_NAME: &str = "invalid_name";

/// The version a new project gets when none is given: what `specforge init
/// --version` and `specforge.init`'s `version` default to, and what the
/// starter spec's `spec` block states.
pub const DEFAULT_VERSION: &str = "0.1.0";

/// Where the spec files go, relative to the project root.
pub const SPEC_ROOT: &str = "spec";
/// The starter file, relative to the project root.
pub const STARTER_FILE: &str = "spec/hello.spec";
/// The files `.gitignore` gets: the inference cache, the report `collect`
/// writes, and its working directory (installed extensions too).
pub const GITIGNORE: [&str; 3] = [
    "specforge-infer.json",
    "specforge-report.json",
    ".specforge/",
];

/// What to scaffold.
#[derive(Debug, Clone)]
pub struct Request<'a> {
    /// The new project's root.
    pub dir: &'a Path,
    /// The project name; the directory's name when absent.
    pub name: Option<&'a str>,
    /// The project version (`specforge init --version`; [`DEFAULT_VERSION`]
    /// unless given).
    pub version: &'a str,
    /// Extension specifiers; an entry may hold several, comma-separated.
    pub extensions: &'a [String],
    /// A project the new one must not be inside (MCP: the server's own).
    pub forbid_inside: Option<&'a Path>,
}

/// Everything init will write, validated.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub name: String,
    pub version: String,
    /// The extensions enabled, in order: builtins by name, local installs by
    /// the name they declare.
    pub extensions: Vec<String>,
    pub config: Value,
    pub starter: String,
    /// Local `.wasm` files installed through `add`, each read once by the
    /// plan.
    pub installs: Vec<LocalFile>,
}

/// What init wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub starter_path: PathBuf,
    pub name: String,
    pub version: String,
    pub extensions: Vec<String>,
    /// The files init wrote: `specforge.json`, the starter, `.gitignore`
    /// when it lacked an entry, and each local install's module and lock.
    pub writes: Writes,
}

/// Validate `req` and build what init writes, writing nothing.
pub fn plan(req: &Request, runtime: &dyn WasmRuntime) -> Result<Plan, OpError> {
    if let Some(marker) = ["specforge.json", "specforge.spec"]
        .into_iter()
        .find(|marker| req.dir.join(marker).exists())
    {
        return Err(OpError::new(
            OpErrorKind::Conflict,
            PROJECT_EXISTS,
            format!("project already exists at {} ({marker})", req.dir.display()),
        ));
    }
    if let Some(current) = req.forbid_inside
        && absolute(req.dir).starts_with(absolute(current))
    {
        return Err(OpError::new(
            OpErrorKind::Conflict,
            PROJECT_EXISTS,
            format!(
                "{} is inside the current project at {}",
                req.dir.display(),
                current.display()
            ),
        ));
    }

    let name = match req.name {
        Some(name) => name.to_string(),
        None => req
            .dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-project")
            .to_string(),
    };
    validate_project_name(&name).map_err(|why| invalid_name(&name, why))?;
    // The starter declares `spec "<id>"`: an ID outside the identifier
    // contract would fail its first check (E014).
    let spec_id = sanitize_entity_id(&name);
    if !(2..=60).contains(&spec_id.chars().count()) {
        return Err(invalid_name(
            &name,
            "it must be 2-60 characters, as the starter's spec ID must",
        ));
    }
    let version = req.version.to_string();

    let (extensions, installs) = extensions_of(req.extensions, runtime)?;
    // A builtin is enabled after the builtins it requires, as `add` does.
    let mut extensions = with_required_builtins(extensions, runtime)?;
    // Test obligations (`verify`) on software kinds come from
    // @specforge/testing (ADR 0002), so enabling software enables it too;
    // the project's test runners get the extensions that collect their
    // results.
    let has = |list: &[String], name: &str| list.iter().any(|e| e == name);
    if has(&extensions, "@specforge/software") && !has(&extensions, "@specforge/testing") {
        extensions.push("@specforge/testing".to_string());
    }
    if has(&extensions, "@specforge/testing") {
        for runner in detected_runners(req.dir) {
            if !has(&extensions, runner) {
                extensions.push(runner.to_string());
            }
        }
    }

    let starter = match starter_template(&extensions, &installs, runtime)? {
        Some(template) => template
            .replace("{project}", &spec_id)
            .replace("{version}", &version),
        None => structural_starter(&spec_id, &version),
    };
    // Whatever the extensions contribute, the file is written as the
    // formatter writes it, so `specforge format --check` accepts it.
    let starter = canonical(&starter, req.dir);
    let config = json!({
        "$schema": "https://specforge.dev/schema/specforge.json",
        "name": name,
        "version": version,
        "spec_root": SPEC_ROOT,
        "extensions": extensions,
    });
    Ok(Plan {
        name,
        version,
        extensions,
        config,
        starter,
        installs,
    })
}

/// Write `plan` into `dir`: `specforge.json`, the starter file, the
/// `.gitignore` entries, and each local install, without reading the files
/// again. A failed install removes what init wrote (and its error reports
/// nothing written).
pub fn apply(dir: &Path, plan: Plan) -> Result<Outcome, OpError> {
    let write_error = |what: &str, e: std::io::Error| {
        OpError::new(
            OpErrorKind::of_io(&e),
            "init_write_failed",
            format!("cannot write {what}: {e}"),
        )
    };
    let created_dir = !dir.exists();
    let spec_dir = dir.join(SPEC_ROOT);
    let created_spec_dir = !spec_dir.exists();
    std::fs::create_dir_all(&spec_dir).map_err(|e| write_error("the spec directory", e))?;
    let gitignore_path = dir.join(".gitignore");
    let gitignore_before = std::fs::read_to_string(&gitignore_path).ok();

    let mut writes = Writes::none();
    let written = (|| -> Result<(), OpError> {
        crate::config::write(dir, &plan.config)?;
        writes.record(dir.join(crate::config::CONFIG_FILE));
        let appended = append_gitignore(&gitignore_path, gitignore_before.as_deref().unwrap_or(""))
            .map_err(|e| write_error(".gitignore", e))?;
        writes.record_if(appended, &gitignore_path);
        std::fs::write(dir.join(STARTER_FILE), &plan.starter)
            .map_err(|e| write_error(STARTER_FILE, e))?;
        writes.record(dir.join(STARTER_FILE));
        for local in plan.installs {
            let added = extension::install_local(dir, local)?;
            writes.merge(added.writes);
        }
        Ok(())
    })();

    if let Err(mut error) = written {
        // Leave the directory as it was.
        let _ = std::fs::remove_file(dir.join(crate::config::CONFIG_FILE));
        let _ = std::fs::remove_file(dir.join(STARTER_FILE));
        let _ = std::fs::remove_file(specforge_installed::lock_path(dir));
        let _ = std::fs::remove_dir_all(dir.join(".specforge"));
        match &gitignore_before {
            Some(text) => {
                let _ = std::fs::write(&gitignore_path, text);
            }
            None => {
                let _ = std::fs::remove_file(&gitignore_path);
            }
        }
        if created_spec_dir {
            let _ = std::fs::remove_dir(&spec_dir);
        }
        if created_dir {
            let _ = std::fs::remove_dir(dir);
        }
        // What was written is removed again.
        error.writes = Writes::none();
        return Err(error);
    }

    Ok(Outcome {
        root: dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf()),
        config_path: dir.join(crate::config::CONFIG_FILE),
        starter_path: dir.join(STARTER_FILE),
        name: plan.name,
        version: plan.version,
        extensions: plan.extensions,
        writes,
    })
}

fn invalid_name(name: &str, why: &str) -> OpError {
    OpError::new(
        OpErrorKind::InvalidInput,
        INVALID_NAME,
        format!("invalid project name '{name}': {why}"),
    )
}

/// The extensions `specifiers` enable, in order, and the local files to
/// install, each read once. A builtin is enabled by name; a local `.wasm` is
/// checked by its handshake and enabled by the name it declares. Anything
/// else is refused before anything is written.
fn extensions_of(
    specifiers: &[String],
    runtime: &dyn WasmRuntime,
) -> Result<(Vec<String>, Vec<LocalFile>), OpError> {
    let mut extensions = Vec::new();
    let mut installs = Vec::new();
    for specifier in specifiers.iter().flat_map(|s| s.split(',')) {
        let specifier = specifier.trim();
        let unresolvable = |why: String| {
            OpError::new(
                OpErrorKind::ExtensionNotFound,
                extension::NOT_FOUND,
                format!("unresolvable extension '{specifier}': {why}"),
            )
        };
        let name = match extension::parse(specifier).map_err(|e| unresolvable(e.message))? {
            Source::Builtin(name) => name.to_string(),
            Source::Local(path) => {
                let local = LocalFile::read(runtime, &path).map_err(|e| unresolvable(e.message))?;
                let name = local.binary.candidate().name().to_string();
                installs.push(local);
                name
            }
            Source::Registry(_) | Source::Git { .. } => {
                let builtins: Vec<&str> = specforge_component::builtins::BUILTIN_EXTENSIONS
                    .iter()
                    .map(|(name, _)| *name)
                    .collect();
                return Err(unresolvable(
                    "init enables builtins and local .wasm files only".to_string(),
                )
                .with_suggestion(format!(
                    "init with builtins ({}), then configure a registry and run `specforge add {specifier}`",
                    builtins.join(", ")
                )));
            }
        };
        if !extensions.contains(&name) {
            extensions.push(name);
        }
    }
    Ok((extensions, installs))
}

/// `extensions`, each builtin after the builtins it requires, each once, in
/// order.
fn with_required_builtins(
    extensions: Vec<String>,
    runtime: &dyn WasmRuntime,
) -> Result<Vec<String>, OpError> {
    let mut enabled: Vec<String> = Vec::new();
    for name in extensions {
        if let Some(builtin) = extension::builtin_name(&name) {
            for peer in extension::required_builtins(runtime, builtin)? {
                if !enabled.iter().any(|e| e == peer) {
                    enabled.push(peer.to_string());
                }
            }
        }
        if !enabled.contains(&name) {
            enabled.push(name);
        }
    }
    Ok(enabled)
}

/// The starter template the extensions contribute: the one listed first
/// wins. A local file contributes under the name it declares, read by the
/// plan; a builtin is read from the binary, through `runtime`, only until a
/// template is found, and one that does not load refuses the init.
fn starter_template(
    extensions: &[String],
    installs: &[LocalFile],
    runtime: &dyn WasmRuntime,
) -> Result<Option<String>, OpError> {
    for name in extensions {
        let local = installs
            .iter()
            .find(|local| local.binary.candidate().name() == name);
        let template = match local {
            Some(local) => local
                .binary
                .candidate()
                .starter_template()
                .map(str::to_string),
            // A builtin that does not load refuses the init (E028).
            None => match extension::builtin_name(name) {
                Some(builtin) => Candidate::builtin(runtime, builtin)?
                    .starter_template()
                    .map(str::to_string),
                None => None,
            },
        };
        if template.is_some() {
            return Ok(template);
        }
    }
    Ok(None)
}

fn sanitize_entity_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// `starter`, formatted as the project at `dir` formats its files.
fn canonical(starter: &str, dir: &Path) -> String {
    let (config, _) = specforge_formatter::load_config(dir, dir);
    specforge_formatter::format_source(starter, &config).formatted
}

fn structural_starter(project_name: &str, version: &str) -> String {
    format!(
        r#"// {project_name} — starter spec file
//
// This file uses only structural syntax that the core compiler
// understands without any extensions. Install extensions to unlock
// domain-specific entity types.
//
// Try: specforge check

spec "{project_name}" {{
  version "{version}"
}}
"#
    )
}

/// Append the missing [`GITIGNORE`] entries to `path`, whose text is
/// `existing`.
fn append_gitignore(path: &Path, existing: &str) -> std::io::Result<bool> {
    let missing: Vec<&str> = GITIGNORE
        .into_iter()
        .filter(|entry| !existing.lines().any(|l| l.trim() == *entry))
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }
    let mut text = existing.to_string();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    for entry in missing {
        text.push_str(entry);
        text.push('\n');
    }
    std::fs::write(path, text).map(|()| true)
}

/// `path`, absolute and canonical through its nearest existing ancestor.
fn absolute(path: &Path) -> PathBuf {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut existing = path.clone();
    let mut rest = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name().map(|n| n.to_os_string()) else {
            break;
        };
        rest.push(name);
        if !existing.pop() {
            break;
        }
    }
    match existing.canonicalize() {
        Ok(canonical) => rest.iter().rev().fold(canonical, |p, part| p.join(part)),
        Err(_) => path,
    }
}

/// Runner extensions for the test runners the project at `path` uses.
fn detected_runners(path: &Path) -> Vec<&'static str> {
    let mut runners = Vec::new();
    if path.join("Cargo.toml").is_file() {
        runners.push("@specforge/cargo-test");
    }
    if uses_vitest(path) {
        runners.push("@specforge/vitest");
    }
    runners
}

/// A vitest config file, or vitest among package.json's dependencies (a
/// project may configure it inside vite.config.*).
fn uses_vitest(path: &Path) -> bool {
    let config = std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| {
            e.file_name().to_str().is_some_and(|n| {
                n.starts_with("vitest.config.") || n.starts_with("vitest.workspace.")
            })
        });
    config
        || std::fs::read_to_string(path.join("package.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .is_some_and(|pkg| {
                ["dependencies", "devDependencies"]
                    .iter()
                    .any(|deps| pkg[deps].get("vitest").is_some())
            })
}
