//! The target: a fresh project scaffolded by `ops init` with `@specforge/checks`, `@code/core` and the built
//! `@gsd/core`, written by one typed-edit plan (all or nothing, judged by E080), then compiled again to read its
//! computed values back (ADR 0360 D1, D5).

use serde_json::Value;
use specforge_common::{PROJECT_FILE, ProjectRoot, Severity};
use specforge_component::ComponentRuntime;
use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::edit::{self, Edit};
use specforge_ops::authoring::entity_id;
use specforge_ops::authoring::init;
use specforge_ops::view::ProjectView;
use specforge_ops::{Caller, Containment};
use specforge_project::CompiledProject;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What a target holds before the converter may scaffold it: none of these.
const SCAFFOLD: [&str; 4] = [
    PROJECT_FILE,
    init::SPEC_ROOT,
    "specforge.lock",
    ".specforge",
];
/// The files init adds entries to: a roll back gives each the bytes it held before, or removes it.
const APPENDED: [&str; 2] = [".gitignore", ".gitattributes"];

/// The work a run did: how many projects it compiled and how many edit plans it applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Work {
    pub compiles: usize,
    pub edit_plans: usize,
}

/// `to` is absent, or a directory that holds no project yet (it may hold other files: the tree whose tests the
/// proofs cite); what it holds is read without following a symbolic link. Refused when `to`, a path of [`SCAFFOLD`]
/// or one of [`APPENDED`] is a symbolic link (absorb would write through it, or a roll back remove it), or when one of
/// [`APPENDED`] is no file. The answer is what a roll back restores: the bytes of each of [`APPENDED`] there now.
pub fn check_to(to: &Path) -> Result<Scaffold, String> {
    let Ok(meta) = std::fs::symlink_metadata(to) else {
        return Ok(Scaffold {
            root: to.to_path_buf(),
            made_root: true,
            appended_before: Vec::new(),
        });
    };
    if meta.file_type().is_symlink() {
        return Err(format!(
            "--to {} is a symbolic link: name the directory it points to",
            to.display()
        ));
    }
    if !meta.is_dir() {
        return Err(format!("--to {} is not a directory", to.display()));
    }
    let links: Vec<&str> = SCAFFOLD
        .into_iter()
        .chain(APPENDED)
        .filter(|name| is_link(&to.join(name)))
        .collect();
    if !links.is_empty() {
        return Err(format!(
            "--to {} holds a symbolic link where absorb writes: {}; remove it, or convert elsewhere",
            to.display(),
            links.join(", ")
        ));
    }
    let held: Vec<&str> = SCAFFOLD
        .into_iter()
        .filter(|name| std::fs::symlink_metadata(to.join(name)).is_ok())
        .collect();
    if !held.is_empty() {
        return Err(format!(
            "--to {} already holds {}: convert into a directory with no project",
            to.display(),
            held.join(", ")
        ));
    }
    let mut appended_before = Vec::new();
    for name in APPENDED {
        let path = to.join(name);
        match std::fs::symlink_metadata(&path) {
            Err(_) => appended_before.push((name, None)),
            Ok(meta) if meta.is_file() => {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                appended_before.push((name, Some(bytes)));
            }
            Ok(_) => {
                return Err(format!(
                    "--to {} holds {name}, which is no file: init appends lines to it",
                    to.display()
                ));
            }
        }
    }
    Ok(Scaffold {
        root: to.to_path_buf(),
        made_root: false,
        appended_before,
    })
}

/// `path` is a symbolic link, dangling or not.
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

/// What a run may scaffold in `--to`, and what a roll back restores: whether the run creates the directory, and the
/// bytes each of [`APPENDED`] held before (`None`: it was not there).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaffold {
    root: PathBuf,
    made_root: bool,
    appended_before: Vec<(&'static str, Option<Vec<u8>>)>,
}

/// The scaffolded target and its last compile.
pub struct Target {
    scaffold: Scaffold,
    project: CompiledProject,
}

impl Target {
    /// `ops init` in `scaffold`'s directory (what [`check_to`] answered) named `name`, loading `@specforge/checks`,
    /// `@code/core`, `gsd_core` and `extra`, then compiled. On a failure, the directory is as it was.
    pub fn create(
        scaffold: Scaffold,
        name: &str,
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
                name: Some(name),
                version: init::DEFAULT_VERSION,
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

    /// Each create's required field it gives no value: `<kind> '<id>' <field>`.
    pub fn missing(&self, creates: &[Edit]) -> Vec<String> {
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
                if field.required && !fields.contains_key(&field.name) {
                    missing.push(format!("{kind} '{id}' {}", field.name));
                }
            }
        }
        missing
    }

    /// Each create whose id the target refuses (E013, E014): an old id that is the keyword of a kind the target
    /// declares (`blocker`, `phase`, `item`) keeps no id there, so it is named before any write.
    pub fn refused_ids(&self, creates: &[Edit]) -> Vec<String> {
        let view = ProjectView::of(&self.project);
        creates
            .iter()
            .filter_map(|create| match create {
                Edit::Create { kind, id, .. } => {
                    entity_id::validate_id(id, view.registries().kinds())
                        .err()
                        .map(|refused| format!("{kind} '{id}': {}", refused.message))
                }
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

    /// The computed value of `path` on the entity `id` (`null`: none).
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
    /// `--to` as [`check_to`] found it: the directory removed when the run made it; else each path of [`SCAFFOLD`]
    /// removed (none was there, so whatever is there the run made) and each of [`APPENDED`] given back its bytes, or
    /// removed when it was not there. Nothing is followed through a symbolic link.
    fn roll_back(self) {
        if self.made_root {
            let _ = std::fs::remove_dir_all(&self.root);
            return;
        }
        for name in SCAFFOLD {
            let path = self.root.join(name);
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
