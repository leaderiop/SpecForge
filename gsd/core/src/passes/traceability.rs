//! The requirement thread's gap rules. Three are off in gsd-compat and switched by their `specforge.*` setting
//! (ADR 0351): `gsd/E519`, a truth of a phase's plan proves one of its plan's requirements; `gsd/W520`, a complete
//! plan's commits touched every path it planned and the files it deletes are gone; `gsd/E521`, a task commit names its
//! task by its `Gsd-Task:` trailer. `gsd/W522` is on under every profile (ADR 0352 D7): a code item a task or a commit
//! names is anchored by a plan that reaches a requirement.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::commits::{CommitType, commits};
use super::plans::{children, link, links, of_kind, phase_of, strings, text};
use crate::collectors::LinkedBy;

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
    found.extend(anchored_items(input));
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
/// or whose `files_deleted` the host serves as still present (its fact found by the plan, named as the host
/// normalised it). A path the host did not serve is not judged; a missing planned file is E016's, never this rule's.
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
            // The fact of each `files_deleted` path, found by its plan and named as the host wrote it.
            let present: BTreeSet<&str> = deleted
                .into_iter()
                .flat_map(|d| &d.facts)
                .filter(|f| f.exists && f.entity.as_deref() == Some(plan.id.as_str()))
                .map(|f| f.path.as_str())
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
/// task, carries no `Gsd-Task:` trailer that links it. A commit with no type, or a `docs` one (even one whose subject
/// matches a task's title), is not judged.
fn task_trailers(input: &PassInput) -> Vec<PassDiagnostic> {
    commits(input)
        .into_iter()
        .filter_map(|commit| {
            // The collector records only a commit that names a plan; a plan's metadata commit is `docs`.
            let plan = commit.plan.as_deref()?;
            if commit.kind == Some(CommitType::Docs) {
                return None;
            }
            let message = match (&commit.task, commit.linked_by) {
                (Some(task), Some(LinkedBy::Title)) => format!(
                    "commit '{}' names task '{task}' by its subject's title, not by a Gsd-Task trailer",
                    commit.id
                ),
                (None, _) if commit.kind.is_some_and(CommitType::is_task) => format!(
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

/// W522, on the first anchoring task by id (else commit): an item that `task.items` (its plan the task's) or
/// `commit.items_changed` (the commit's plan) names, whose anchors in a phase's plan all sit in plans that reach no
/// requirement: none of their `requirements` resolves to one, and no truth of theirs names one that resolves. Anchors
/// in a quick plan are dropped; an item with none left is not judged. The item need not be held.
fn anchored_items(input: &PassInput) -> Vec<PassDiagnostic> {
    let is = |id: &str, kind: &str| input.entities.iter().any(|e| e.id == id && e.kind == kind);
    let requirement = |id: Option<&str>| id.is_some_and(|r| is(r, "requirement"));
    let reaches = |plan: &str| {
        of_kind(input, "plan").filter(|p| p.id == plan).any(|p| {
            links(input, p, "requirements")
                .into_iter()
                .any(|r| requirement(Some(r)))
        }) || of_kind(input, "truth").any(|t| {
            link(input, t, "plan") == Some(plan) && requirement(link(input, t, "requirement"))
        })
    };
    // Each item's anchors: (a commit's, the anchoring entity, its plan); a task sorts before a commit.
    let mut anchors: BTreeMap<String, Vec<(bool, &str, &str)>> = BTreeMap::new();
    for task in of_kind(input, "task") {
        let Some(plan) = link(input, task, "plan") else {
            continue;
        };
        for item in strings(task, "items") {
            anchors
                .entry(item)
                .or_default()
                .push((false, &task.id, plan));
        }
    }
    let recorded = commits(input);
    for commit in &recorded {
        let Some(plan) = commit.plan.as_deref() else {
            continue;
        };
        for item in &commit.items_changed {
            anchors
                .entry(item.clone())
                .or_default()
                .push((true, &commit.id, plan));
        }
    }
    anchors
        .into_iter()
        .filter_map(|(item, mut found)| {
            found.retain(|(_, _, plan)| phase_of(input, plan).is_some());
            if found.is_empty() || found.iter().any(|(_, _, plan)| reaches(plan)) {
                return None;
            }
            found.sort();
            let plans: BTreeSet<&str> = found.iter().map(|(_, _, plan)| *plan).collect();
            let named: Vec<String> = plans.iter().map(|p| format!("'{p}'")).collect();
            let (noun, verb) = if plans.len() == 1 {
                ("plan", "reaches")
            } else {
                ("plans", "reach")
            };
            Some(
                PassDiagnostic::new(
                    "W522",
                    PassSeverity::Warning,
                    format!(
                        "item `{item}` is anchored only by {noun} {}, which {verb} no requirement",
                        named.join(", ")
                    ),
                )
                .with_suggestion(
                    "name the requirement it delivers in the plan's requirements, or a truth's requirement",
                )
                .with_entity(found[0].1),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
