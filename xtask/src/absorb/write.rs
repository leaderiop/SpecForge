//! The target: a fresh project scaffolded by `ops init` with `@specforge/checks`, `@code/core` and the built
//! `@gsd/core`, written by one typed-edit plan (all or nothing, judged by E080), then compiled again to read its
//! computed values back (ADR 0360 D1, D5).

use serde_json::Value;
use specforge_common::{PROJECT_FILE, ProjectRoot, Severity};
use specforge_component::ComponentRuntime;
use specforge_ops::asking::Asking;
use specforge_ops::authoring::apply::Origin;
use specforge_ops::authoring::edit::{self, Edit};
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
/// The files init adds entries to, which the run removes on failure only when it created them.
const APPENDED: [&str; 2] = [".gitignore", ".gitattributes"];

/// The work a run did: how many projects it compiled and how many edit plans it applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Work {
    pub compiles: usize,
    pub edit_plans: usize,
}

/// `to` is absent, or a directory that holds no project yet (it may hold other files: the tree whose tests the
/// proofs cite).
pub fn check_to(to: &Path) -> Result<(), String> {
    if !to.exists() {
        return Ok(());
    }
    if !to.is_dir() {
        return Err(format!("--to {} is not a directory", to.display()));
    }
    let held: Vec<&str> = SCAFFOLD
        .into_iter()
        .filter(|name| to.join(name).exists())
        .collect();
    if held.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "--to {} already holds {}: convert into a directory with no project",
            to.display(),
            held.join(", ")
        ))
    }
}

/// What a run scaffolds: its root, whether the run created it, and which of [`APPENDED`] were there before.
struct Scaffold {
    root: PathBuf,
    made_root: bool,
    appended_before: Vec<&'static str>,
}

/// The scaffolded target and its last compile.
pub struct Target {
    scaffold: Scaffold,
    project: CompiledProject,
}

impl Target {
    /// `ops init` at `to` named `name`, loading `@specforge/checks`, `@code/core`, `gsd_core` and `extra`, then
    /// compiled.
    pub fn create(
        to: &Path,
        name: &str,
        gsd_core: &Path,
        extra: &[String],
        work: &mut Work,
    ) -> Result<Target, String> {
        let made_root = !to.exists();
        std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
        let scaffold = Scaffold {
            root: to.to_path_buf(),
            made_root,
            appended_before: APPENDED
                .into_iter()
                .filter(|name| to.join(name).exists())
                .collect(),
        };
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
    fn roll_back(self) {
        if self.made_root {
            let _ = std::fs::remove_dir_all(&self.root);
            return;
        }
        for name in SCAFFOLD {
            let path = self.root.join(name);
            if path.is_dir() {
                let _ = std::fs::remove_dir_all(&path);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        for name in APPENDED {
            if !self.appended_before.contains(&name) {
                let _ = std::fs::remove_file(self.root.join(name));
            }
        }
    }
}
