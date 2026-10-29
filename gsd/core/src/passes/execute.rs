//! The execute pass: `gsd/E401` (a summary's requirements are its plan's) and `gsd/E403` (a `tdd` task has a RED commit
//! before its GREEN one). `gsd/E402` is declarative (`rules.rs`) and `gsd/E405` is the schema-drift pass.

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::commits::{Commit, commits};
use super::plans::{children, name_of, of_kind, text};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("execute", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = requirements(input);
    found.extend(red_before_green(input, &commits(input)));
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

/// E403: for each `tdd` task (its own `type`, or its plan's), a recorded `feat` or `fix` commit that names it comes
/// after a `test` commit that names it. A commit that names only the plan counts for the plan's `tdd` task when it
/// has exactly one. The RED run's failure is not checked: upstream reads a run record, a commit is all there is here.
fn red_before_green(input: &PassInput, commits: &[Commit]) -> Vec<PassDiagnostic> {
    let mut found = Vec::new();
    for plan in of_kind(input, "plan") {
        let tdd_plan = text(plan, "type") == Some("tdd");
        let tdd: Vec<&PassEntity> = children(input, &plan.id, "task")
            .into_iter()
            .filter(|t| tdd_plan || text(t, "type") == Some("tdd"))
            .collect();
        for task in &tdd {
            let named: Vec<&Commit> = commits
                .iter()
                .filter(|c| {
                    c.task.as_deref() == Some(task.id.as_str())
                        || (tdd.len() == 1
                            && c.task.is_none()
                            && c.plan.as_deref() == Some(plan.id.as_str()))
                })
                .collect();
            let Some(green) = named
                .iter()
                .position(|c| matches!(c.kind.as_str(), "feat" | "fix"))
            else {
                continue;
            };
            if named[..green].iter().any(|c| c.kind == "test") {
                continue;
            }
            found.push(
                PassDiagnostic::new(
                    "E403",
                    PassSeverity::Error,
                    format!(
                        "task '{}' has a GREEN commit '{}' with no RED `test(…)` commit before it",
                        task.id,
                        name_of(&named[green].id)
                    ),
                )
                .with_suggestion("commit the failing test first, then the implementation")
                .with_entity(&task.id),
            );
        }
    }
    found
}

#[cfg(test)]
mod tests;
