//! `specforge install` and `uninstall` (ADR 0292): the project's contributions rendered into each host, compared with
//! the disk, and written as one change under the write lock and the write journal, or, when anything is refused,
//! not written at all. Uninstall is the same plan against a tree that renders nothing.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::drift::{self, Action, HostDisk, HostPlan, PlannedFile};
use super::port::{HostRenderer, HostTree};
use super::render::rendered;
use crate::change::locked;
use crate::view::ProjectView;
use crate::{Caller, Held, OpError, OpErrorKind, Wrote};

/// What an install or uninstall planned, and whether it wrote.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub plans: Vec<HostPlan>,
    pub wrote: Wrote,
}

impl Outcome {
    /// Whether any file was refused: nothing was written.
    pub fn refused(&self) -> bool {
        self.plans.iter().any(|p| !p.refused.is_empty())
    }

    /// Whether any file would be created, updated or removed (or was).
    pub fn changes(&self) -> bool {
        self.plans.iter().any(|p| p.changes().next().is_some())
    }

    pub fn document(&self) -> Document {
        Document {
            hosts: self
                .plans
                .iter()
                .map(|plan| HostDocument {
                    host: plan.host.as_str(),
                    files: plan
                        .files
                        .iter()
                        .map(|f| FileDocument {
                            path: f.path.clone(),
                            action: f.action.as_str(),
                        })
                        .collect(),
                    not_rendered: plan
                        .not_rendered
                        .iter()
                        .map(|n| NotRenderedDocument {
                            what: n.what.clone(),
                            why: n.why.clone(),
                        })
                        .collect(),
                })
                .collect(),
            refused: self
                .plans
                .iter()
                .flat_map(|plan| {
                    plan.refused.iter().map(|r| RefusedDocument {
                        host: plan.host.as_str(),
                        path: r.path.clone(),
                        code: r.code.as_str(),
                        message: r.message.clone(),
                    })
                })
                .collect(),
            dry_run: self.wrote.previewed(),
        }
    }
}

/// What `install --format json` answers.
#[derive(Debug, Clone, Serialize)]
pub struct Document {
    pub hosts: Vec<HostDocument>,
    pub refused: Vec<RefusedDocument>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostDocument {
    pub host: &'static str,
    pub files: Vec<FileDocument>,
    pub not_rendered: Vec<NotRenderedDocument>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDocument {
    pub path: String,
    pub action: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotRenderedDocument {
    pub what: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefusedDocument {
    pub host: &'static str,
    pub path: String,
    pub code: &'static str,
    pub message: String,
}

/// How to run: `force` rewrites or removes a file of install's that was edited since (never a foreign one);
/// `dry_run` plans and writes nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub force: bool,
    pub dry_run: bool,
}

/// The project's disk through the caller's containment.
pub(super) struct ContainedHostDisk<'a> {
    pub(super) caller: Caller<'a>,
    pub(super) sources: &'a specforge_common::SourceRoot,
}

impl ContainedHostDisk<'_> {
    fn held(&self, path: &str) -> Result<Held, String> {
        if let Some(link) = super::owned::link_in(self.sources.base(), path) {
            return Err(format!(
                "it passes through the link {}, and install never writes or removes through a link",
                link.display()
            ));
        }
        self.caller
            .containment()
            .resolve(&self.sources.path(path))
            .map_err(|e| {
                format!(
                    "it resolves to {}, outside the project",
                    e.landed().display()
                )
            })
    }
}

impl HostDisk for ContainedHostDisk<'_> {
    fn read(&self, path: &str) -> Result<Option<String>, String> {
        let bytes = self
            .held(path)?
            .read_if_there()
            .map_err(|e| e.to_string())?;
        bytes
            .map(|b| String::from_utf8(b).map_err(|_| "it is not UTF-8 text".to_string()))
            .transpose()
    }

    fn files_under(&self, dir: &str) -> Vec<String> {
        fn walk(real: &Path, at: &str, out: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(real) else {
                return;
            };
            let mut names: Vec<_> = entries.filter_map(Result::ok).collect();
            names.sort_by_key(|e| e.file_name());
            for entry in names {
                let name = entry.file_name().to_string_lossy().to_string();
                let child = format!("{at}/{name}");
                match entry.file_type() {
                    Ok(t) if t.is_dir() => walk(&entry.path(), &child, out),
                    Ok(t) if t.is_file() => out.push(child),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        if let Ok(held) = self.held(dir) {
            walk(&held.real, dir, &mut out);
        }
        out
    }
}

/// Install the project's contributions into the hosts `renderers` serve.
pub fn install(
    view: &ProjectView<'_>,
    renderers: &[&dyn HostRenderer],
    options: Options,
    caller: Caller<'_>,
) -> Result<Outcome, OpError> {
    let mut trees = Vec::new();
    for renderer in renderers {
        trees.push(renderer.render(&rendered(view, renderer.host())?));
    }
    run(view, renderers, &trees, options, caller, "install")
}

/// Remove exactly what install marked from the hosts `renderers` serve.
pub fn uninstall(
    view: &ProjectView<'_>,
    renderers: &[&dyn HostRenderer],
    options: Options,
    caller: Caller<'_>,
) -> Result<Outcome, OpError> {
    let trees = vec![HostTree::default(); renderers.len()];
    run(view, renderers, &trees, options, caller, "uninstall")
}

fn run(
    view: &ProjectView<'_>,
    renderers: &[&dyn HostRenderer],
    trees: &[HostTree],
    options: Options,
    caller: Caller<'_>,
    command: &str,
) -> Result<Outcome, OpError> {
    view.project_root()?;
    // ADR 0306: a specforge.json the compile cannot use loads no extension, so the plan would strip every host file
    // it contributed. Any problem refuses: a mistyped `spec_root` or `settings` changes what install writes.
    if let Some(problem) = view.env().config_problems().first() {
        return Err(OpError::of(
            OpErrorKind::SchemaMismatch,
            problem.diagnostic(),
        ));
    }
    let sources = view.env().sources().clone();
    let disk = ContainedHostDisk {
        caller,
        sources: &sources,
    };
    let plan_all = || -> Vec<HostPlan> {
        renderers
            .iter()
            .zip(trees)
            .map(|(renderer, tree)| drift::plan(*renderer, tree, &disk, options.force, command))
            .collect()
    };
    if options.dry_run {
        return Ok(Outcome {
            plans: plan_all(),
            wrote: Wrote::Previewed,
        });
    }
    // The plan is read under the write lock, after an interrupted write was rolled back, so it is made over the
    // disk as the rollback left it. A refusal on any host, or nothing to change, stages nothing.
    let plans = std::cell::RefCell::new(Vec::new());
    let writes = locked::commit(
        &sources,
        "cannot write the host files",
        caller,
        true,
        || {
            *plans.borrow_mut() = plan_all();
            Ok(())
        },
        |stager| {
            let plans = plans.borrow();
            if plans.iter().any(|p| !p.refused.is_empty()) {
                return Ok(());
            }
            for file in plans.iter().flat_map(HostPlan::changes) {
                let before = file.before.as_ref().map(|b| b.as_bytes().to_vec());
                match (&file.after, before) {
                    (Some(after), before) => {
                        stager.write(&file.path, before, after.clone().into_bytes())?;
                    }
                    (None, Some(before)) => stager.remove(&file.path, before)?,
                    (None, None) => {}
                }
            }
            Ok(())
        },
    )?;
    let plans = plans.into_inner();
    let changes: Vec<&PlannedFile> = plans.iter().flat_map(HostPlan::changes).collect();
    if plans.iter().all(|p| p.refused.is_empty()) {
        remove_empty_dirs(&sources, caller, &changes);
    }
    Ok(Outcome {
        plans,
        wrote: Wrote::Ran(writes),
    })
}

/// The directories a removal left empty go, deepest first, up to the project root.
fn remove_empty_dirs(
    sources: &specforge_common::SourceRoot,
    caller: Caller<'_>,
    changes: &[&PlannedFile],
) {
    let mut dirs: Vec<String> = Vec::new();
    for file in changes.iter().filter(|f| f.action == Action::Remove) {
        let mut at = file.path.as_str();
        // `.claude`, `.codex` and `.cursor` themselves are the user's: only the layout directories below them go.
        while let Some((parent, _)) = at.rsplit_once('/').filter(|(p, _)| p.contains('/')) {
            if !dirs.iter().any(|d| d == parent) {
                dirs.push(parent.to_string());
            }
            at = parent;
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.matches('/').count()));
    let real: Vec<PathBuf> = dirs
        .iter()
        .filter_map(|d| caller.containment().resolve(&sources.path(d)).ok())
        .map(|held| held.real)
        .collect();
    Held::remove_empty(&real);
}
