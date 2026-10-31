//! The execute pass: `gsd/E401` (a summary's requirements are its plan's), `gsd/W403` (a `tdd` task has a RED commit
//! before its GREEN one), `gsd/W407` (an executed task has a commit) and `gsd/W408` (a deliverable's checks pass, or a
//! person judged it). `gsd/E402` is declarative (`rules.rs`) and `gsd/E405` is the schema-drift pass. The pass reads
//! the test report only for the checks' computed status, so a project with no report is not judged on them.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits::{Commit, commits};
use super::plans::{children, name_of, of_kind, rows, text};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("execute", |p| {
        p.phase("check").reads_tests().run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = requirements(input);
    let commits = commits(input);
    found.extend(red_before_green(input, &commits));
    found.extend(task_commits(input, &commits));
    found.extend(deliverables(input));
    found
}

/// The targets of the resolved `label` edges of `source`.
fn targets<'a>(input: &'a PassInput, source: &str, label: &str) -> BTreeSet<&'a str> {
    input
        .edges
        .iter()
        .filter(|e| e.source == source && e.label == label)
        .map(|e| e.target.as_str())
        .collect()
}

fn list(ids: &[&&str]) -> String {
    ids.iter().map(|i| **i).collect::<Vec<_>>().join(", ")
}

/// E401: a summary's `requirements_completed`, as a set of resolved ids, is its plan's `requirements`
/// (`execute-plan`'s check). A quick plan's summary with none on either side is clean.
fn requirements(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "summary")
        .filter_map(|summary| {
            let plan = input
                .edges
                .iter()
                .find(|e| e.source == summary.id && e.label == "plan")?
                .target
                .as_str();
            let completed = targets(input, &summary.id, "requirements_completed");
            let planned = targets(input, plan, "requirements");
            let missing: Vec<&&str> = planned.difference(&completed).collect();
            let extra: Vec<&&str> = completed.difference(&planned).collect();
            if missing.is_empty() && extra.is_empty() {
                return None;
            }
            let mut said = Vec::new();
            if !missing.is_empty() {
                said.push(format!("missing {}", list(&missing)));
            }
            if !extra.is_empty() {
                said.push(format!("not in the plan: {}", list(&extra)));
            }
            Some(
                PassDiagnostic::new(
                    "E401",
                    PassSeverity::Error,
                    format!(
                        "summary '{}' lists requirements_completed that differ from plan '{plan}' ({})",
                        summary.id,
                        said.join("; ")
                    ),
                )
                .with_suggestion("list exactly the requirements the plan names")
                .with_entity(&summary.id),
            )
        })
        .collect()
}

/// W403: for each `tdd` task (its own `type`, or its plan's), a recorded `feat` or `fix` commit that names it descends
/// from a `test` commit that names it (#5275): by the commits' recorded ancestry (`descends_from`), or, when no commit
/// of the task has any (recorded before the collector read parents), by time. A commit that names only the plan counts
/// for the plan's `tdd` task when it is the plan's only task (GSD's own wording names the plan, not the task). It is a
/// warning: the order is a fact of history that no later commit can repair, so it must never refuse the collector's
/// write. The RED run's failure is not checked: upstream reads a run record, a commit is all there is here.
fn red_before_green(input: &PassInput, commits: &[Commit]) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for plan in of_kind(input, "plan") {
        let tdd_plan = text(plan, "type") == Some("tdd");
        let tdd: Vec<&PassEntity> = children(input, &plan.id, "task")
            .into_iter()
            .filter(|t| tdd_plan || text(t, "type") == Some("tdd"))
            .collect();
        let alone = children(input, &plan.id, "task").len() == 1;
        for task in &tdd {
            let named: Vec<&Commit> = commits
                .iter()
                .filter(|c| {
                    c.task.as_deref() == Some(task.id.as_str())
                        || (alone
                            && c.task.is_none()
                            && c.plan.as_deref() == Some(plan.id.as_str()))
                })
                .collect();
            let is_green = |c: &Commit| matches!(c.kind.as_str(), "feat" | "fix");
            let Some(green) = named.iter().position(|c| is_green(c)) else {
                continue;
            };
            let reds: Vec<&str> = named
                .iter()
                .filter(|c| c.kind == "test")
                .map(|c| c.id.as_str())
                .collect();
            let follows = if named.iter().any(|c| c.descends_from.is_some()) {
                named.iter().filter(|c| is_green(c)).any(|g| {
                    g.descends_from
                        .iter()
                        .flatten()
                        .any(|a| reds.contains(&a.as_str()))
                })
            } else {
                named[..green].iter().any(|c| c.kind == "test")
            };
            if follows {
                continue;
            }
            let hash = name_of(&named[green].id);
            let message = if reds.is_empty() {
                format!(
                    "task '{}' has a GREEN commit '{hash}' with no RED `test(…)` commit before it",
                    task.id
                )
            } else {
                format!(
                    "GREEN '{hash}' does not descend from any RED (`test`) commit of task '{}' (green_before_red)",
                    task.id
                )
            };
            found.push(
                PassDiagnostic::new("W403", PassSeverity::Warning, message)
                    .with_suggestion("commit the failing test first, then the implementation")
                    .with_entity(&task.id),
            );
        }
    }
    found
}

/// W407: a task of a `complete` plan that no recorded commit names (a halted plan's later tasks never ran). A `manual` task is a human's
/// step and leaves no commit. The host runs it only while `specforge.task_commits` is on.
fn task_commits(input: &PassInput, commits: &[Commit]) -> Vec<PassDiagnostic> {
    of_kind(input, "task")
        .filter(|task| text(task, "type") != Some("manual"))
        .filter(|task| {
            let plan = crate::digest::scope_of(&task.id);
            let executed = of_kind(input, "plan")
                .find(|p| p.id == plan)
                .and_then(|p| text(p, "status"))
                .is_some_and(|s| s == "complete");
            executed
                && !commits
                    .iter()
                    .any(|c| c.task.as_deref() == Some(task.id.as_str()))
        })
        .map(|task| {
            PassDiagnostic::new(
                "W407",
                PassSeverity::Warning,
                format!(
                    "task '{}' of a complete plan has no recorded commit",
                    task.id
                ),
            )
            .with_suggestion(
                "commit it as `type(plan-id): task title`, then run the gsd-git collector",
            )
            .with_entity(&task.id)
        })
        .collect()
}

/// W408, per deliverable: one that is not human-judged has checks and none failed or went unrun; one that is has a
/// rationale. A check with no status (no report) is not judged: GSD's auto-pass rule (`summary.md:195`).
fn deliverables(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "deliverable")
        .filter_map(|d| {
            let problem = if text(d, "human_judgment") == Some("true") {
                text(d, "rationale")
                    .is_none()
                    .then(|| "is human-judged but gives no rationale".to_string())?
            } else {
                let checks = rows(d, "checks");
                if checks.is_empty() {
                    "has no checks and is not human-judged".to_string()
                } else {
                    let bad: Vec<String> = checks
                        .iter()
                        .filter(|c| {
                            matches!(
                                c.get("status").map(String::as_str),
                                Some("fail" | "unknown")
                            )
                        })
                        .map(|c| {
                            format!(
                                "'{}' is {}",
                                c.get("ref").map_or("", String::as_str),
                                c["status"]
                            )
                        })
                        .collect();
                    if bad.is_empty() {
                        return None;
                    }
                    format!("has checks that do not pass: {}", bad.join(", "))
                }
            };
            Some(
                PassDiagnostic::new(
                    "W408",
                    PassSeverity::Warning,
                    format!("deliverable '{}' {problem}", d.id),
                )
                .with_suggestion("make the check pass, or set human_judgment with a rationale")
                .with_entity(&d.id),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
