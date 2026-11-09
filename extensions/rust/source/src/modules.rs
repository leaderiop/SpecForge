//! A target's module tree by rustc's rules (ADR 0348 D6), walked breadth-first from the target's root, each round's
//! new files parsed by the host in one request.
//!
//! - A file is *mod-rs* when it is a crate root, a `mod.rs`, or was reached through a `path` attribute: the files of
//!   its child modules live in its own directory. Any other file (`src/a.rs`) keeps them in a directory of its stem
//!   (`src/a/`).
//! - `mod b;` is `<dir>/b.rs`, else `<dir>/b/mod.rs`; inside an inline `mod x { … }` the directory gains `x/` (a raw
//!   `r#x` too), or `d/` when the inline module has its own `#[path = "d"]`.
//! - `#[path = "p"]` is relative to the declaring file's directory outside an inline module, and to the inline
//!   module's directory inside one. Every `path` a `#[cfg_attr(…, path = "…")]` names is followed too: its files'
//!   items are the module's (cfg twins).
//!
//! A file outside the project, or absent, keeps its `mod` item and no items: rustc owns that error, and a cfg-gated
//! module's file may be absent by design. A file already in a target's tree is not walked again (a `#[path]` loop
//! back to an ancestor): its `mod` item stays, and the walk is bounded by the project's files.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use specforge_code_facts::{crate_segment, Definition, Edition, PackageId, Target};
use specforge_extension_sdk::host::{
    self, NotParsed, SyntaxFile, SyntaxFiles, SyntaxOutcome, SyntaxRequest, SyntaxSource,
};
use specforge_extension_sdk::prelude::Fact;

use crate::items::{self, Child};
use crate::QUERY;

/// One cargo target as a crate: the package it is of, its crate segment, its edition and its root file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Crate {
    pub package: PackageId,
    pub segment: String,
    pub edition: Edition,
    pub root: String,
}

/// Every crate the served `package` facts' targets are, in their order. A cargo package fact that does not read
/// back is the collector's error: `@rust/cargo` wrote it.
pub(crate) fn crates(facts: &[Fact]) -> Result<Vec<Crate>, String> {
    let mut crates = Vec::new();
    for fact in facts.iter().filter(|f| f.kind == "package") {
        let package = PackageId::from_str(&fact.id).map_err(|e| format!("{}: {e}", fact.id))?;
        if !fact.id.starts_with("pkg:cargo/") {
            continue;
        }
        let edition = Edition::cargo(fact.fields.get("edition").and_then(|e| e.as_str()))
            .map_err(|e| format!("{}: {e}", fact.id))?;
        let targets = fact
            .fields
            .get("targets")
            .and_then(|t| t.as_array())
            .map_or(&[][..], Vec::as_slice);
        for written in targets {
            let target = written
                .as_str()
                .ok_or_else(|| format!("{}: a target is no text: {written}", fact.id))
                .and_then(|text| Target::read(text).map_err(|e| format!("{}: {e}", fact.id)))?;
            crates.push(Crate {
                segment: crate_segment(&package, target.kind, &target.name),
                package: package.clone(),
                edition,
                root: target.root,
            });
        }
    }
    Ok(crates)
}

/// A module whose file is to be read: the crate it is of, its module path, and its candidate files in order (the
/// first that parses is the module's; `b.rs` before `b/mod.rs`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Module {
    pub krate: usize,
    pub path: Vec<String>,
    pub candidates: Vec<ModuleFile>,
}

/// A candidate file of a module and the directory its child modules' files live in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ModuleFile {
    pub file: String,
    pub children: String,
}

impl ModuleFile {
    /// `file` reached as a mod-rs file (a crate root, a `mod.rs`, a `path` attribute's): its children beside it.
    pub(crate) fn mod_rs(file: String) -> ModuleFile {
        let children = parent(&file).to_string();
        ModuleFile { file, children }
    }
}

/// The files `paths`, parsed by the host's reader with the pack's query in one request, in the order asked.
fn parse(paths: &[String]) -> Result<Vec<SyntaxFile>, String> {
    host::syntax::parse(&SyntaxRequest {
        source: SyntaxSource::Worktree {
            root: "project".to_string(),
        },
        files: SyntaxFiles::Paths(paths.to_vec()),
        language: "rust".to_string(),
        query: QUERY.to_string(),
    })
    .map(|answer| answer.files)
    .map_err(|e| format!("the host's source reader refused rust-source's request: {e:?}"))
}

/// Every definition of every crate's module tree, its files parsed one round at a time.
pub(crate) fn walk(crates: &[Crate]) -> Result<Vec<Definition>, String> {
    let mut answers: BTreeMap<String, SyntaxOutcome> = BTreeMap::new();
    // A file is walked once per crate: a `#[path]` back to an ancestor keeps its `mod` item and ends there.
    let mut seen: BTreeSet<(usize, String)> = BTreeSet::new();
    let mut definitions = Vec::new();
    let mut round: Vec<Module> = crates
        .iter()
        .enumerate()
        .filter_map(|(krate, c)| {
            Some(Module {
                krate,
                path: Vec::new(),
                candidates: vec![ModuleFile::mod_rs(joined("", &c.root)?)],
            })
        })
        .collect();
    while !round.is_empty() {
        let new: BTreeSet<String> = round
            .iter()
            .flat_map(|m| &m.candidates)
            .map(|c| c.file.clone())
            .filter(|file| !answers.contains_key(file))
            .collect();
        answers.extend(all_parsed(new.into_iter().collect())?);
        let mut next = Vec::new();
        for module in round {
            let Some((candidate, matches)) =
                module
                    .candidates
                    .iter()
                    .find_map(|c| match answers.get(&c.file) {
                        Some(SyntaxOutcome::Parsed { matches, .. }) => Some((c, matches)),
                        _ => None,
                    })
            else {
                continue;
            };
            if !seen.insert((module.krate, candidate.file.clone())) {
                continue;
            }
            let krate = &crates[module.krate];
            for child in items::read(krate, &module.path, candidate, matches, &mut definitions) {
                next.extend(child_modules(module.krate, &module.path, candidate, child));
            }
        }
        round = next;
    }
    Ok(definitions)
}

/// The module `child` declares in `file` (of the module `path`): one with rustc's two candidate files, or one per
/// file its `path` attributes name (each `cfg_attr` twin is the module).
fn child_modules(krate: usize, path: &[String], file: &ModuleFile, child: Child) -> Vec<Module> {
    let mut module_path = path.to_vec();
    module_path.extend(child.inline.iter().cloned());
    module_path.push(child.name.clone());
    // The directory an inline module's child files and its `path` attributes start from: each inline module's own
    // `path`, else its name; one that leaves the project reaches no file.
    let Some(inline_dir) = child
        .dirs
        .iter()
        .try_fold(file.children.clone(), |dir, named| joined(&dir, named))
    else {
        return Vec::new();
    };
    let module = |candidates| Module {
        krate,
        path: module_path.clone(),
        candidates,
    };
    if child.paths.is_empty() {
        let dir = join_dir(&inline_dir, &child.name);
        return vec![module(vec![
            ModuleFile {
                file: format!("{}.rs", join_dir(&inline_dir, &child.name)),
                children: dir.clone(),
            },
            ModuleFile {
                file: join_dir(&dir, "mod.rs"),
                children: dir,
            },
        ])];
    }
    let base = if child.inline.is_empty() {
        parent(&file.file).to_string()
    } else {
        inline_dir
    };
    child
        .paths
        .iter()
        .filter_map(|p| joined(&base, p))
        .map(|file| module(vec![ModuleFile::mod_rs(file)]))
        .collect()
}

/// `paths` parsed until no answer says the cap stopped it: each round asks again for the files the last left
/// unparsed. A file whose matches alone pass the cap is the collector's error, never an item dropped.
fn all_parsed(mut paths: Vec<String>) -> Result<BTreeMap<String, SyntaxOutcome>, String> {
    let mut answers = BTreeMap::new();
    while !paths.is_empty() {
        let mut again = Vec::new();
        for file in parse(&paths)? {
            match file.outcome {
                SyntaxOutcome::NotParsed {
                    reason: NotParsed::Capped,
                } => again.push(file.path),
                SyntaxOutcome::NotParsed {
                    reason: NotParsed::Missing | NotParsed::Denied,
                } => {
                    answers.insert(file.path, file.outcome);
                }
                SyntaxOutcome::NotParsed { reason } => {
                    return Err(format!(
                        "{}: the host's reader did not parse it ({}); rust-source reads every module or none",
                        file.path,
                        reason.name()
                    ));
                }
                parsed => {
                    answers.insert(file.path, parsed);
                }
            }
        }
        if !again.is_empty() && again.len() == paths.len() {
            return Err(format!(
                "{}: its matches alone pass the host's answer cap",
                again[0]
            ));
        }
        paths = again;
    }
    Ok(answers)
}

/// The directory of the project-relative `file`: `""` at the root.
fn parent(file: &str) -> &str {
    file.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// `name` under the project-relative directory `dir`.
fn join_dir(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// `path` (relative, `/` or `\` separated) under the project-relative `dir`, its `.` and `..` resolved; `None` when it
/// leaves the project or is absolute.
pub(crate) fn joined(dir: &str, path: &str) -> Option<String> {
    if path.starts_with(['/', '\\']) || path.contains(':') {
        return None;
    }
    let mut segments: Vec<&str> = Vec::new();
    for segment in dir.split('/').chain(path.split(['/', '\\'])) {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            segment => segments.push(segment),
        }
    }
    Some(segments.join("/"))
}
