//! The verify-path pass: `gsd/E320`, a verify command's `cd` or `--prefix` directory exists, and holds the manifest
//! its tool needs (`gate-verify-command-paths`, `src/verify-command-grounding.cts`).
//!
//! A compute pass writes `task.verify_paths`: the directory and manifest of each `automated` command (the text is only
//! parsed, never run). A check pass declares a read of that field (K11), so the host hands it the facts of those paths
//! and the text of the manifests. A directory not yet on disk is `pending_creation` when the plan's own task files or
//! artifact paths equal or cover it.

mod grounding;
mod manifest;

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::plans::{children, of_kind, rows, strings, text};
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
    let values = of_kind(input, "task")
        .filter_map(|task| {
            let paths: BTreeSet<String> = commands(task)
                .iter()
                .filter_map(|c| match resolve(c) {
                    Verdict::At(target) => Some(target.paths()),
                    _ => None,
                })
                .flatten()
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
    /// A regular file was read (a directory has no bytes).
    file: bool,
    text: Option<String>,
}

fn seen(input: &PassInput, task: &str, path: &str) -> Seen {
    let found = input
        .files
        .get("targets")
        .into_iter()
        .flat_map(|f| &f.facts)
        .find(|f| f.entity.as_deref() == Some(task) && f.path == path);
    Seen {
        exists: found.is_some_and(|f| f.exists),
        file: found.is_some_and(|f| f.bytes.is_some()),
        text: found.and_then(|f| f.text.clone()),
    }
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    if !input.files.contains_key("targets") {
        return Vec::new();
    }
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
                    if let Some(why) = broken(input, &task.id, &target, &declared) {
                        found.push(
                            said("E320", PassSeverity::Error, format!("but {why}")).with_suggestion(
                                "fix the path, or list the directory in the plan's task files so it counts as created",
                            ),
                        );
                    } else if let Some(why) = manifest_problem(input, &task.id, &target) {
                        found.push(warn(why));
                    }
                }
            }
        }
    }
    found
}

/// Why the command cannot run, when it cannot.
fn broken(input: &PassInput, task: &str, target: &Target, declared: &[String]) -> Option<String> {
    let dir = seen(input, task, &target.dir);
    if !target.dir.is_empty() && !(dir.exists && !dir.file) {
        let pending = declared.iter().any(|d| {
            d == &target.dir
                || d.strip_prefix(target.dir.as_str())
                    .is_some_and(|r| r.starts_with('/'))
        });
        return (!pending).then(|| format!("'{}' is not a directory", target.dir));
    }
    let manifest = target.manifest()?;
    let there = seen(input, task, &manifest);
    (!(there.exists && there.file)).then(|| {
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
fn manifest_problem(input: &PassInput, task: &str, target: &Target) -> Option<String> {
    let Need::Package(script) = &target.need else {
        return None;
    };
    let path = target.manifest()?;
    let manifest = seen(input, task, &path);
    // Not there yet: the directory is pending creation.
    if !manifest.exists {
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
        .flat_map(|t| strings(t, "files"));
    let artifacts = children(input, plan, "artifact")
        .into_iter()
        .filter_map(|a| text(a, "path").map(str::to_string));
    tasks.chain(artifacts).map(|p| strip(&p)).collect()
}

#[cfg(test)]
mod tests;
