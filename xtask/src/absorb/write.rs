//! The target: a fresh project scaffolded by `ops init` with `@specforge/checks`, `@code/core` and the built
//! `@gsd/core`, written by one typed-edit plan (all or nothing, judged by E080), then compiled again to read its
//! computed values back (ADR 0360 D1, D5).

use serde_json::Value;
use specforge_common::{PROJECT_FILE, ProjectConfig, ProjectRoot, Severity};
use specforge_component::ComponentRuntime;
use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::edit::{self, Edit, Input};
use specforge_ops::authoring::entity_id;
use specforge_ops::authoring::init;
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::CompiledProject;

use super::read::Project;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// What a scaffold puts in `--to` beside the spec root (or, when that is the project's root, the starter) and the
/// files init appends to: none of these may be there.
const SCAFFOLD: [&str; 3] = [PROJECT_FILE, "specforge.lock", ".specforge"];
/// The files init adds entries to: a roll back gives each the bytes it held before, or removes it.
const APPENDED: [&str; 2] = [".gitignore", ".gitattributes"];

/// The work a run did: how many projects it compiled and how many edit plans it applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Work {
    pub compiles: usize,
    pub edit_plans: usize,
}

/// `to` is absent, or a directory that holds no project yet (it may hold other files: a copy of the source's tree,
/// as the self-spec sample converts into), read without following a symbolic link. `spec_root` is the source's,
/// which the target keeps: it must not be there yet, or, when it is the project's root, `to` must hold no source the
/// target would read. Refused when `to`, or a path absorb writes, is a symbolic link (absorb would write through it,
/// or a roll back remove it), when one of [`APPENDED`] is no file, or when the spec root leaves `to`. The answer is
/// what a roll back undoes.
pub fn check_to(to: &Path, spec_root: Option<&str>) -> Result<Scaffold, String> {
    let spec_dir = spec_dir(spec_root)?;
    let mut scaffold = Scaffold {
        root: to.to_path_buf(),
        made_root: false,
        made: BTreeSet::new(),
        appended_before: Vec::new(),
    };
    if is_link(to) {
        return Err(format!(
            "--to {} is a symbolic link: name the directory it points to",
            to.display()
        ));
    }
    let Ok(meta) = std::fs::symlink_metadata(to) else {
        scaffold.made_root = true;
        return Ok(scaffold);
    };
    if !meta.is_dir() {
        return Err(format!("--to {} is not a directory", to.display()));
    }
    let links: Vec<&str> = APPENDED
        .into_iter()
        .filter(|name| is_link(&to.join(name)))
        .collect();
    if !links.is_empty() {
        return Err(format!(
            "--to {} holds a symbolic link where absorb writes: {}; remove it, or convert elsewhere",
            to.display(),
            links.join(", ")
        ));
    }
    let mut paths: Vec<PathBuf> = SCAFFOLD.into_iter().map(PathBuf::from).collect();
    paths.push(
        spec_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(init::STARTER_NAME)),
    );
    scaffold.claim(&paths)?;
    if spec_dir.is_none() {
        let sources = ProjectConfig::default().sources(to).files();
        if !sources.is_empty() {
            let shown: Vec<String> = sources
                .iter()
                .take(SHOWN)
                .map(|file| file.strip_prefix(to).unwrap_or(file).display().to_string())
                .collect();
            return Err(format!(
                "--to {} holds {} spec file(s) the target would read as its own (the source's spec root is the \
                 project's root): {}",
                to.display(),
                sources.len(),
                shown.join(", ")
            ));
        }
    }
    for name in APPENDED {
        let path = to.join(name);
        match std::fs::symlink_metadata(&path) {
            Err(_) => scaffold.appended_before.push((name, None)),
            Ok(meta) if meta.is_file() => {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                scaffold.appended_before.push((name, Some(bytes)));
            }
            Ok(_) => {
                return Err(format!(
                    "--to {} holds {name}, which is no file: init appends lines to it",
                    to.display()
                ));
            }
        }
    }
    Ok(scaffold)
}

/// How many of the files a refusal names.
const SHOWN: usize = 5;

/// The directory `spec_root` names, relative to the project's root; `None` when it is that root (unset, empty, `.`).
/// Refused when it is absolute or leaves the root.
fn spec_dir(spec_root: Option<&str>) -> Result<Option<PathBuf>, String> {
    let mut dir = PathBuf::new();
    for part in spec_root
        .map(Path::new)
        .into_iter()
        .flat_map(Path::components)
    {
        match part {
            Component::Normal(part) => dir.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "the source's spec_root '{}' leaves its project's directory: absorb keeps it in the target",
                    spec_root.unwrap_or_default()
                ));
            }
        }
    }
    Ok((!dir.as_os_str().is_empty()).then_some(dir))
}

/// `path` is a symbolic link, dangling or not: the one test of every path absorb writes (`--to`, the scaffold's paths,
/// what init appends to, the ledger), so a run never writes through a link, nor a roll back removes one.
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

/// A file absorb writes outside the scaffold, the ledger (`<to>/absorb-ledger.json` or `--ledger`'s) or the selection
/// (`--selection`'s), as [`output`] will write it, `flag` naming it: refused when it is a symbolic link or there and no
/// file. A file already there is replaced; the directories above it are the user's.
pub fn check_output(flag: &str, path: &Path) -> Result<(), String> {
    if is_link(path) {
        return Err(format!(
            "the {flag} {} is a symbolic link: absorb never writes through one; remove it, or name another --{flag}",
            path.display()
        ));
    }
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => Err(format!("the {flag} {} is no file", path.display())),
        _ => Ok(()),
    }
}

/// Write `text` at `path`, checked again as [`check_output`] does, so a link made since is not written through either;
/// the directories above it are made.
pub fn output(flag: &str, path: &Path, text: &str) -> Result<(), String> {
    check_output(flag, path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// What a run may write in `--to`, and what a roll back undoes: whether the run creates the directory, every path
/// below it the run creates (none was there), and the bytes each of [`APPENDED`] held before (`None`: not there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaffold {
    root: PathBuf,
    made_root: bool,
    made: BTreeSet<PathBuf>,
    appended_before: Vec<(&'static str, Option<Vec<u8>>)>,
}

impl Scaffold {
    /// Claim `paths` (relative to the root) for the run: each, and each directory above it, that is not there yet is
    /// the run's to remove on a roll back. Refused, naming every one, when one is there already (a directory above
    /// one may be) or a symbolic link is among them. A directory the run makes claims nothing.
    pub fn claim(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        if self.made_root {
            return Ok(());
        }
        let (mut held, mut links) = (BTreeSet::new(), BTreeSet::new());
        for path in paths {
            let mut at = PathBuf::new();
            let parts: Vec<Component> = path.components().collect();
            for (n, part) in parts.iter().enumerate() {
                at.push(part);
                if is_link(&self.root.join(&at)) {
                    links.insert(at.display().to_string());
                    continue;
                }
                match std::fs::symlink_metadata(self.root.join(&at)) {
                    Err(_) => {
                        self.made.insert(at.clone());
                    }
                    Ok(meta) if meta.is_dir() && n + 1 < parts.len() => {}
                    Ok(_) => {
                        held.insert(at.display().to_string());
                    }
                }
            }
        }
        let list = |set: BTreeSet<String>| set.into_iter().collect::<Vec<_>>().join(", ");
        if !links.is_empty() {
            return Err(format!(
                "--to {} holds a symbolic link where absorb writes: {}; remove it, or convert elsewhere",
                self.root.display(),
                list(links)
            ));
        }
        if !held.is_empty() {
            return Err(format!(
                "--to {} already holds {}: convert into a directory with no project",
                self.root.display(),
                list(held)
            ));
        }
        Ok(())
    }
}

/// The scaffolded target and its last compile.
pub struct Target {
    scaffold: Scaffold,
    project: CompiledProject,
}

impl Target {
    /// `ops init` in `scaffold`'s directory (what [`check_to`] answered and the plan claimed) as `project` says
    /// (the source's name, version and spec root), loading `@specforge/checks`, `@code/core`, `gsd_core` and `extra`,
    /// then compiled. On a failure, the directory is as it was.
    pub fn create(
        scaffold: Scaffold,
        project: &Project,
        gsd_core: &Path,
        extra: &[String],
        work: &mut Work,
    ) -> Result<Target, String> {
        let to = scaffold.root.clone();
        let to = to.as_path();
        std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
        let mut extensions = vec![
            "@specforge/checks".to_string(),
            "@code/core".to_string(),
            gsd_core.display().to_string(),
        ];
        extensions.extend(extra.iter().cloned());
        // As `specforge init` does: the component is read where it was built, outside the new project.
        let containment = Containment::unbounded();
        let runtime = ComponentRuntime::with_user_cache();
        let outcome = init::init(
            &init::Request {
                dir: to,
                name: Some(&project.name),
                version: project.version.as_deref().unwrap_or(init::DEFAULT_VERSION),
                spec_root: project.spec_root.as_deref(),
                extensions: &extensions,
                caller: Caller::new(&containment, Asking::Nobody),
            },
            &runtime,
        );
        if let Err(refusal) = outcome {
            scaffold.roll_back();
            return Err(format!(
                "init {}: {} {}",
                to.display(),
                refusal.error.code,
                refusal.error.message
            ));
        }
        match compile(to, work) {
            Ok(project) => Ok(Target { scaffold, project }),
            Err(why) => {
                scaffold.roll_back();
                Err(why)
            }
        }
    }

    /// Each create's required field it gives no value, or an empty one (a blank text, an empty list): (kind, path,
    /// field), a scoped create named by its path.
    pub fn missing(&self, creates: &[Edit]) -> Vec<(String, String, String)> {
        let view = ProjectView::of(&self.project);
        let mut missing = Vec::new();
        for create in creates {
            let Edit::Create {
                kind, id, fields, ..
            } = create
            else {
                continue;
            };
            for field in edit::fields_of(&view, kind) {
                if field.required && fields.get(&field.name).is_none_or(is_empty) {
                    let path = path_of(&view, kind, id, fields);
                    missing.push((kind.clone(), path, field.name.clone()));
                }
            }
        }
        missing
    }

    /// Each create whose id the target refuses (E013, E014): an old id that is the keyword of a kind the target
    /// declares (`blocker`, `phase`, `item`) keeps no id there, so it is named, by its path, before any write.
    pub fn refused_ids(&self, creates: &[Edit]) -> Vec<String> {
        let view = ProjectView::of(&self.project);
        creates
            .iter()
            .filter_map(|create| match create {
                Edit::Create {
                    kind, id, fields, ..
                } => entity_id::validate_id(id, view.registries().kinds())
                    .err()
                    .map(|refused| {
                        let path = path_of(&view, kind, id, fields);
                        format!("{kind} '{path}': {}", refused.message)
                    }),
                _ => None,
            })
            .collect()
    }

    /// The creates, written as one plan, then the target compiled again.
    pub fn apply(&mut self, creates: &[Edit], work: &mut Work) -> Result<(), String> {
        let containment = Containment::within(&self.scaffold.root);
        let view = ProjectView::of(&self.project);
        let applied = edit::run(
            &view,
            creates,
            false,
            &Origin::cli(false),
            Caller::new(&containment, Asking::Nobody),
        );
        work.edit_plans += 1;
        if let Err(refusal) = applied {
            let data = refusal
                .error
                .data
                .map(|d| format!("\n{d}"))
                .unwrap_or_default();
            return Err(format!(
                "the edit plan was refused: {} {}{data}",
                refusal.error.code, refusal.error.message
            ));
        }
        self.project = compile(&self.scaffold.root, work)?;
        Ok(())
    }

    /// Its last compile.
    pub fn project(&self) -> &CompiledProject {
        &self.project
    }

    /// The computed value of `path` on the entity whose path is `id` (`null`: none).
    pub fn computed(&self, id: &str, path: &str) -> Value {
        self.project
            .computed()
            .get(id, path)
            .map_or(Value::Null, |v| v.as_value().clone())
    }

    /// What the target reports: (errors, warnings, infos), and each error and warning as
    /// `<code> <file>:<line> <message>`.
    pub fn report(&self) -> ([usize; 3], Vec<String>) {
        let mut counts = [0; 3];
        let mut lines = Vec::new();
        for d in self.project.diagnostics() {
            let slot = match d.severity {
                Severity::Error => 0,
                Severity::Warning => 1,
                Severity::Info => 2,
            };
            counts[slot] += 1;
            if slot == 2 {
                continue;
            }
            let at = d
                .span
                .as_ref()
                .map(|s| format!("{}:{}", s.file, s.start_line))
                .unwrap_or_default();
            lines.push(format!("{} {at} {}", d.code, d.message));
        }
        (counts, lines)
    }

    /// Remove what the run scaffolded: the directory when it made it, else the project's files.
    pub fn roll_back(self) {
        self.scaffold.roll_back();
    }
}

/// Whether `input` gives no value: a blank text or an empty list.
fn is_empty(input: &Input) -> bool {
    match input {
        Input::One(text) => text.trim().is_empty(),
        Input::Many(items) => items.is_empty(),
        _ => false,
    }
}

/// The path of a create of `kind` named `id` writing `fields`, as the target places it (ADR 0284): its scope field's
/// value, `/`, its id, for a kind the target scopes; its id otherwise.
fn path_of(view: &ProjectView, kind: &str, id: &str, fields: &BTreeMap<String, Input>) -> String {
    let scopes = &view.registries().graph_inputs().scopes;
    match scopes.get(kind).and_then(|field| fields.get(field)) {
        Some(Input::One(scope)) => format!("{scope}/{id}"),
        _ => id.to_string(),
    }
}

/// The project at `root`, compiled with the builtins and components it names.
fn compile(root: &Path, work: &mut Work) -> Result<CompiledProject, String> {
    let root = ProjectRoot::of(root).map_err(|e| format!("{}: {e:?}", root.display()))?;
    work.compiles += 1;
    Ok(CompiledProject::compile(
        &root,
        Some(Arc::new(ComponentRuntime::with_user_cache())),
    ))
}

impl Scaffold {
    /// `--to` as [`check_to`] found it: the directory removed when the run made it; else each path the run claimed
    /// removed, deepest first (none was there, so whatever is there the run made), and each of [`APPENDED`] given back
    /// its bytes, or removed when it was not there. Nothing is followed through a symbolic link.
    fn roll_back(self) {
        if self.made_root {
            let _ = std::fs::remove_dir_all(&self.root);
            return;
        }
        let mut made: Vec<&PathBuf> = self.made.iter().collect();
        made.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        for path in made {
            let path = self.root.join(path);
            match std::fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() => {
                    let _ = std::fs::remove_dir_all(&path);
                }
                Ok(_) => {
                    let _ = std::fs::remove_file(&path);
                }
                Err(_) => {}
            }
        }
        for (name, before) in self.appended_before {
            let path = self.root.join(name);
            let _ = match before {
                Some(bytes) => std::fs::write(&path, bytes),
                None => std::fs::remove_file(&path),
            };
        }
    }
}
