//! The verify-path pass: `gsd/E320`, a verify command's `cd` or `--prefix` directory exists, and holds the manifest
//! its tool needs (`gate-verify-command-paths`, `src/verify-command-grounding.cts`).
//!
//! A compute pass writes `task.verify_paths`: the directory and manifest of each `automated` command (the text is only
//! parsed, never run). A check pass declares a read of that field (K11), so the host hands it the facts of those paths
//! and the text of the manifests. A directory not yet on disk is `pending_creation` when the plan's own task files or
//! artifact paths equal or cover it.

mod grounding;

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

/// `true` when the disk says `path` is there, as a directory (no bytes were read) when `dir`.
fn present(input: &PassInput, task: &str, path: &str, dir: bool) -> bool {
    input
        .files
        .get("targets")
        .into_iter()
        .flat_map(|f| &f.facts)
        .find(|f| f.entity.as_deref() == Some(task) && f.path == path)
        .is_some_and(|f| f.exists && (f.bytes.is_none() == dir))
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    if !input.files.contains_key("targets") {
        return Vec::new();
    }
    let mut found = Vec::new();
    for task in of_kind(input, "task") {
        let plan = crate::digest::scope_of(&task.id);
        let declared = declared_paths(input, plan);
        for command in commands(task) {
            let Verdict::At(target) = resolve(&command) else {
                continue;
            };
            if let Some(why) = broken(input, &task.id, &target, &declared) {
                found.push(
                    PassDiagnostic::new("E320", PassSeverity::Error, format!(
                        "task '{}' verifies with `{command}`, but {why}", task.id
                    ))
                    .with_suggestion("fix the path, or list the directory in the plan's task files so it counts as created")
                    .with_entity(&task.id),
                );
            }
        }
    }
    found
}

/// Why the command cannot run, when it cannot.
fn broken(input: &PassInput, task: &str, target: &Target, declared: &[String]) -> Option<String> {
    if !target.dir.is_empty() && !present(input, task, &target.dir, true) {
        let pending = declared.iter().any(|d| {
            d == &target.dir
                || d.strip_prefix(target.dir.as_str())
                    .is_some_and(|r| r.starts_with('/'))
        });
        return (!pending).then(|| format!("'{}' is not a directory", target.dir));
    }
    let manifest = target.manifest()?;
    (!present(input, task, &manifest, false)).then(|| {
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
