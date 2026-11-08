//! The requirement thread's gap rules (ADR 0351), each off in gsd-compat and switched by its `specforge.*` setting:
//! `gsd/E519`, a truth of a phase's plan proves one of its plan's requirements; `gsd/W520`, a complete plan's commits
//! touched every path it planned and the files it deletes are gone; `gsd/E521`, a task commit names its task by its
//! `Gsd-Task:` trailer.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits::commits;
use super::plans::{children, link, links, of_kind, strings, text};

/// The executor's task commit types (`gsd-executor.md`'s commit step), `docs` left out: a plan's metadata commit is
/// `docs` and names no task by design.
const TASK_TYPES: &[&str] = &["feat", "fix", "test", "refactor", "perf", "style", "chore"];

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("traceability", |p| {
        p.phase("check")
            .input(Input::files(
                "deleted",
                FileReadSpec::field("plan.files_deleted"),
            ))
            .run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = truth_requirements(input);
    found.extend(plan_paths(input));
    found.extend(task_trailers(input));
    found
}

/// E519, on the truth: a truth whose plan is in a phase (not a quick plan) names a requirement, and one its plan
/// claims in `requirements`.
fn truth_requirements(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "truth")
        .filter_map(|truth| {
            let plan = input
                .entities
                .iter()
                .find(|e| Some(e.id.as_str()) == link(input, truth, "plan"))?;
            link(input, plan, "phase")?;
            let (message, suggestion) = match link(input, truth, "requirement") {
                None => (
                    format!("truth '{}' names no requirement it proves", truth.id),
                    "set its requirement to the one of its plan's requirements it proves".to_string(),
                ),
                Some(requirement) => {
                    let claimed: BTreeSet<&str> =
                        links(input, plan, "requirements").into_iter().collect();
                    if claimed.contains(requirement) {
                        return None;
                    }
                    (
                        format!(
                            "truth '{}' proves '{requirement}', which is not a requirement of plan '{}'",
                            truth.id, plan.id
                        ),
                        format!(
                            "name one of the plan's requirements, or add '{requirement}' to the plan's requirements"
                        ),
                    )
                }
            };
            Some(
                PassDiagnostic::new("E519", PassSeverity::Error, message)
                    .with_suggestion(suggestion)
                    .with_entity(&truth.id),
            )
        })
        .collect()
}

/// A path as a plan writes it, compared as the commits' are: trimmed, a leading `./` dropped.
fn plain(path: &str) -> &str {
    let path = path.trim();
    path.strip_prefix("./").unwrap_or(path)
}

/// W520, on the plan: a complete plan with a recorded commit (`files_touched` written) whose commits touched none of
/// a planned path (`files_modified`, and each of its artifacts' `path`; a directory is touched by a file under it),
/// or whose `files_deleted` the host serves as still present. A path the host did not serve is not judged; a missing
/// planned file is E016's, never this rule's.
fn plan_paths(input: &PassInput) -> Vec<PassDiagnostic> {
    let deleted = input.files.get("deleted");
    of_kind(input, "plan")
        .filter(|plan| text(plan, "status") == Some("complete"))
        .filter(|plan| plan.values.contains_key("files_touched"))
        .flat_map(|plan| {
            let touched = strings(plan, "files_touched");
            let covered = |planned: &str| {
                let dir = format!("{}/", planned.trim_end_matches('/'));
                touched
                    .iter()
                    .any(|t| plain(t) == planned || plain(t).starts_with(&dir))
            };
            let planned: BTreeSet<String> = strings(plan, "files_modified")
                .iter()
                .map(|p| plain(p).to_string())
                .chain(
                    children(input, &plan.id, "artifact")
                        .into_iter()
                        .filter_map(|a| text(a, "path").map(|p| plain(p).to_string())),
                )
                .filter(|p| !p.is_empty())
                .collect();
            let untouched: Vec<&str> = planned
                .iter()
                .map(String::as_str)
                .filter(|p| !covered(p))
                .collect();
            let present: BTreeSet<String> = strings(plan, "files_deleted")
                .iter()
                .map(|p| plain(p).to_string())
                .filter(|p| {
                    deleted.is_some_and(|d| d.facts.iter().any(|f| f.path == *p && f.exists))
                })
                .collect();
            let mut found = Vec::new();
            if !untouched.is_empty() {
                found.push(
                    PassDiagnostic::new(
                        "W520",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' is complete, but none of its commits touched: {}",
                            plan.id,
                            untouched.join(", ")
                        ),
                    )
                    .with_suggestion("commit the planned work, or take the paths it did not need out of the plan")
                    .with_entity(&plan.id),
                );
            }
            if !present.is_empty() {
                found.push(
                    PassDiagnostic::new(
                        "W520",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' is complete, but files it deletes are still present: {}",
                            plan.id,
                            present.into_iter().collect::<Vec<_>>().join(", ")
                        ),
                    )
                    .with_suggestion("delete them and commit, or take them out of files_deleted")
                    .with_entity(&plan.id),
                );
            }
            found
        })
        .collect()
}

/// E521, on the commit: a recorded commit whose task its subject's title found, or one of a task type that names no
/// task, carries no `Gsd-Task:` trailer that links it. A commit with no type, or a `docs` one, is not judged.
fn task_trailers(input: &PassInput) -> Vec<PassDiagnostic> {
    commits(input)
        .into_iter()
        .filter_map(|commit| {
            // The collector records only a commit that names a plan.
            let plan = commit.plan.as_deref()?;
            let message = match (&commit.task, commit.linked_by.as_deref()) {
                (Some(task), Some("title")) => format!(
                    "commit '{}' names task '{task}' by its subject's title, not by a Gsd-Task trailer",
                    commit.id
                ),
                (None, _) if TASK_TYPES.contains(&commit.kind.as_str()) => format!(
                    "commit '{}' of plan '{plan}' is a task commit that names no task",
                    commit.id
                ),
                _ => return None,
            };
            Some(
                PassDiagnostic::new("E521", PassSeverity::Error, message)
                    .with_suggestion("end the message with the trailer 'Gsd-Task: <plan>/<task>', then collect")
                    .with_entity(&commit.id),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
