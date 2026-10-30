//! The verify-path pass: `gsd/E320`, a verify command's `cd` or `--prefix` directory exists, and holds the manifest
//! its tool needs (`gate-verify-command-paths`, `src/verify-command-grounding.cts`).
//!
//! A compute pass writes `task.verify_paths`: the directory and manifest of each `automated` command (the text is only
//! parsed, never run). A check pass declares a read of that field (K11), so the host hands it the facts of those paths
//! and the text of the manifests. A directory not yet on disk is `pending_creation` when the plan's own task files or
//! artifact paths equal or cover it.
//!
//! The host keys a fact by `(entity, path)` and spends one text budget per read, so the compute pass writes each
//! distinct path once, on the first task (by id) that names it, and the check looks a path up whatever entity carries
//! it. A path the host did not serve (`truncated`, or a `budget` / `too_large` error) is not checked: never E320 or
//! W324.
//!
//! gsd-core modules this file ports: gate-verify-command-paths.cjs.

mod grounding;
mod manifest;

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::plans::{children, of_kind, rows, strings, text};
pub use grounding::missing_sentinel;
use grounding::{Need, Target, Verdict, resolve};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("verify_paths_compute", |p| {
        p.compute().computes(&["task.verify_paths"]).run(compute);
    });
    c.pass("verify_paths", |p| {
        p.phase("check")
            .reads("targets", FileReadSpec::field("task.verify_paths").text())
            .run(check);
    });
}

/// The `automated` commands of a task, in order.
fn commands(task: &PassEntity) -> Vec<String> {
    rows(task, "verify")
        .into_iter()
        .filter_map(|row| row.get("automated").cloned())
        .collect()
}

fn compute(input: &PassInput) -> PassOutput {
    let mut tasks: Vec<&PassEntity> = of_kind(input, "task").collect();
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    let mut named = BTreeSet::new();
    let values = tasks
        .into_iter()
        .filter_map(|task| {
            let paths: BTreeSet<String> = commands(task)
                .iter()
                .filter_map(|c| match resolve(c) {
                    Verdict::At(target) => Some(target.paths()),
                    _ => None,
                })
                .flatten()
                .filter(|path| named.insert(path.clone()))
                .collect();
            (!paths.is_empty()).then(|| PassValue {
                entity: task.id.clone(),
                field: "verify_paths".into(),
                value: paths.into_iter().collect::<Vec<_>>().into(),
            })
        })
        .collect();
    PassOutput {
        values,
        ..Default::default()
    }
}

/// What the host read of one path a task named.
struct Seen {
    exists: bool,
    /// A regular file: it has bytes, or the host named it without reading it (`denied`, `too_large`). A directory
    /// has none of these; an unreadable regular file looks like a directory (the host has no `is_dir` fact).
    file: bool,
    text: Option<String>,
    /// The host did not serve this path: it is not checked.
    skipped: bool,
}

struct Reads {
    seen: BTreeMap<String, Seen>,
    truncated: bool,
}

impl Reads {
    fn new(input: &PassInput) -> Option<Self> {
        let files = input.files.get("targets")?;
        let seen = files
            .facts
            .iter()
            .map(|f| {
                let error = f.error.as_deref();
                let seen = Seen {
                    exists: f.exists,
                    file: f.bytes.is_some() || matches!(error, Some("denied" | "too_large")),
                    text: f.text.clone(),
                    skipped: matches!(error, Some("budget" | "too_large")),
                };
                (f.path.clone(), seen)
            })
            .collect();
        Some(Self {
            seen,
            truncated: files.truncated,
        })
    }

    fn get(&self, path: &str) -> &Seen {
        self.seen
            .get(path)
            .unwrap_or(if self.truncated { &UNSERVED } else { &ABSENT })
    }
}

static ABSENT: Seen = Seen {
    exists: false,
    file: false,
    text: None,
    skipped: false,
};
static UNSERVED: Seen = Seen {
    exists: false,
    file: false,
    text: None,
    skipped: true,
};

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let Some(reads) = Reads::new(input) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for task in of_kind(input, "task") {
        let declared = declared_paths(input, crate::digest::scope_of(&task.id));
        for command in commands(task) {
            let said = |code: &str, severity, what: String| {
                PassDiagnostic::new(
                    code,
                    severity,
                    format!("task '{}' verifies with `{command}`, {what}", task.id),
                )
                .with_entity(&task.id)
            };
            let warn = |what: String| {
                said("W324", PassSeverity::Warning, what)
                    .with_suggestion("name the directory literally, inside the project")
            };
            match resolve(&command) {
                Verdict::Skip => {}
                Verdict::Dynamic => found.push(warn(
                    "whose directory is computed at run time (`$`, `*`, `~`), so it is not checked".into(),
                )),
                Verdict::Outside => found.push(warn(
                    "which runs outside the project: it differs between worktrees, so it is not checked".into(),
                )),
                Verdict::At(target) => {
                    if let Some(why) = broken(&reads, &target, &declared) {
                        found.push(
                            said("E320", PassSeverity::Error, format!("but {why}")).with_suggestion(
                                "fix the path, or list the directory in the plan's task files so it counts as created",
                            ),
                        );
                    } else if let Some(why) = manifest_problem(&reads, &target) {
                        found.push(warn(why));
                    }
                }
            }
        }
    }
    found
}

/// Why the command cannot run, when it cannot.
fn broken(reads: &Reads, target: &Target, declared: &[String]) -> Option<String> {
    let dir = reads.get(&target.dir);
    if !target.dir.is_empty() && dir.skipped {
        return None;
    }
    if !target.dir.is_empty() && !(dir.exists && !dir.file) {
        let pending = declared.iter().any(|d| {
            d == &target.dir
                || d.strip_prefix(target.dir.as_str())
                    .is_some_and(|r| r.starts_with('/'))
        });
        return (!pending).then(|| format!("'{}' is not a directory", target.dir));
    }
    let manifest = target.manifest()?;
    let there = reads.get(&manifest);
    (!there.skipped && !(there.exists && there.file)).then(|| {
        let name = if target.need == Need::Makefile {
            "Makefile"
        } else {
            "package.json"
        };
        format!(
            "'{}' has no {name}",
            if target.dir.is_empty() {
                "."
            } else {
                &target.dir
            }
        )
    })
}

/// A `package.json` that cannot be read as an object, or that has no script the command runs.
fn manifest_problem(reads: &Reads, target: &Target) -> Option<String> {
    let Need::Package(script) = &target.need else {
        return None;
    };
    let path = target.manifest()?;
    let manifest = reads.get(&path);
    // Not there yet (the directory is pending creation), or not served by the host: not checked.
    if !manifest.exists || manifest.skipped {
        return None;
    }
    let Some(names) = manifest.text.as_deref().and_then(manifest::scripts) else {
        return Some(format!("but '{path}' cannot be read as a manifest"));
    };
    let script = script.as_deref()?;
    (!names.iter().any(|n| n == script)).then(|| format!("but '{path}' has no script '{script}'"))
}

/// What the plan says it creates: its tasks' `files` and its artifacts' paths.
fn declared_paths(input: &PassInput, plan: &str) -> Vec<String> {
    let strip = |p: &str| p.trim().trim_start_matches("./").replace('\\', "/");
    let tasks = children(input, plan, "task")
        .into_iter()
        .flat_map(|t| strings(t, "files"))
        .flat_map(|f| {
            f.split([',', ' ', '\t', '\n', '\r'])
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
    let artifacts = children(input, plan, "artifact")
        .into_iter()
        .filter_map(|a| text(a, "path").map(str::to_string));
    tasks
        .chain(artifacts)
        .map(|p| strip(&p))
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests;
