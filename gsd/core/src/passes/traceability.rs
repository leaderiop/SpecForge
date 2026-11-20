//! The requirement thread's gap rules. Two are off in gsd-compat and switched by their `specforge.*` setting
//! (ADR 0351): `gsd/W520`, a complete plan's commits touched every path it planned and the files it deletes are gone;
//! `gsd/E521`, a task commit names its task by its `Gsd-Task:` trailer. `gsd/W522` is on under every profile (ADR 0352
//! D7, amended by ADR 0371 D6): a code item a recorded commit changed is anchored by a plan that reaches a requirement
//! through its `requirements`.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::commits::{Unlinked, commits};
use super::plans::{children, links, of_kind, phase_of, strings, text};

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
    let mut found = plan_paths(input);
    found.extend(task_trailers(input));
    found.extend(anchored_items(input));
    found
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

/// What gsd/E521's suggestion says of the commit `id` it is on: gsd-git records a commit once, by its 12-hex name (ADR
/// 0380), so its own error stays unless a person waives it beside the record (ADR 0381). The command line is the one a
/// person types at a terminal, with the commit's own name; an agent's call of it is refused.
fn recorded(id: &str) -> String {
    format!(
        "; this commit is recorded and keeps its fields: amending it records a new commit beside it, so its own error \
         stays while `specforge.task_trailers` is on, unless a person waives it at a terminal: `specforge gsd \
         waive-trailer {id} '<reason>' <who>`"
    )
}

/// The last `segments` `/`-separated segments of `id`, the form a subject's scope writes (`v1.0/01/01-01/T1` is
/// `01-01/T1` by 2, a plan `v1.0/01/01-01` is `01-01` by 1); `None` when that is all of it (a quick plan's).
fn plan_local(id: &str, segments: usize) -> Option<&str> {
    let (at, _) = id.rmatch_indices('/').nth(segments - 1)?;
    Some(&id[at + 1..])
}

/// E521, on the commit: a recorded commit the rule of [`unlinked`](super::commits::unlinked) judges, one whose task its
/// subject's title found or one of a task type that names no task, carries no `Gsd-Task:` trailer that links it. The
/// suggestion names the trailer the later commits add, as gsd-git reads it (ADR 0380): the task's full id, or the
/// plan's full id and `/<task>`, and the plan-local form as written (`01-01/T1`). A recorded commit keeps its fields,
/// so the suggestion says its own error stays, and ends by naming the waiver a person runs with the commit's name. A
/// commit a person waived (its `waiver` is set) is not judged (ADR 0381).
fn task_trailers(input: &PassInput) -> Vec<PassDiagnostic> {
    commits(input)
        .into_iter()
        .filter_map(|commit| {
            if commit.waived {
                return None;
            }
            let (message, suggestion) = match commit.unlinked()? {
                Unlinked::ByTitle { task } => (
                    format!(
                        "commit '{}' names task '{task}' by its subject's title, not by a Gsd-Task trailer",
                        commit.id
                    ),
                    format!(
                        "end the message of each later commit of the task with the trailer 'Gsd-Task: {task}' (its \
                         full id{}), then collect{}",
                        plan_local(task, 2)
                            .map(|short| format!("; '{short}' reads the same while one plan has that name"))
                            .unwrap_or_default(),
                        recorded(&commit.id)
                    ),
                ),
                Unlinked::NoTask { plan } => (
                    format!(
                        "commit '{}' of plan '{plan}' is a task commit that names no task",
                        commit.id
                    ),
                    format!(
                        "end the message of each later task commit of the plan with the trailer 'Gsd-Task: \
                         {plan}/<task>', naming one of its tasks{}, then collect{}",
                        plan_local(plan, 1)
                            .map(|short| format!(" ('{short}/<task>' reads the same while one plan has that name)"))
                            .unwrap_or_default(),
                        recorded(&commit.id)
                    ),
                ),
            };
            Some(
                PassDiagnostic::new("E521", PassSeverity::Error, message)
                    .with_suggestion(suggestion)
                    .with_entity(&commit.id),
            )
        })
        .collect()
}

/// W522, on the first anchoring commit by id (ADR 0371 D6, amending ADR 0352 D7): an item that a recorded commit's
/// `items_changed` names, whose anchoring commits in a phase's plan all belong to plans that reach no requirement: none
/// of their `requirements` resolves to one. Commits of a quick plan are dropped, and an item with none left is not
/// judged. A commit whose `items_changed` is unwritten anchors nothing. The item need not be held.
fn anchored_items(input: &PassInput) -> Vec<PassDiagnostic> {
    let reaches = |plan: &str| {
        of_kind(input, "plan").filter(|p| p.id == plan).any(|p| {
            links(input, p, "requirements").into_iter().any(|r| {
                input
                    .entities
                    .iter()
                    .any(|e| e.id == r && e.kind == "requirement")
            })
        })
    };
    // Each item's anchors: (the anchoring commit, its plan).
    let mut anchors: BTreeMap<String, Vec<(&str, &str)>> = BTreeMap::new();
    let recorded = commits(input);
    for commit in &recorded {
        let Some(plan) = commit.plan.as_deref() else {
            continue;
        };
        for item in commit.items_changed.iter().flatten() {
            anchors
                .entry(item.clone())
                .or_default()
                .push((&commit.id, plan));
        }
    }
    anchors
        .into_iter()
        .filter_map(|(item, mut found)| {
            found.retain(|(_, plan)| phase_of(input, plan).is_some());
            if found.is_empty() || found.iter().any(|(_, plan)| reaches(plan)) {
                return None;
            }
            found.sort();
            let plans: BTreeSet<&str> = found.iter().map(|(_, plan)| *plan).collect();
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
                .with_suggestion("name the requirement it delivers in the plan's requirements")
                .with_entity(found[0].0),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
