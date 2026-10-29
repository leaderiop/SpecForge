//! The computed values of execution: a plan's status, a task's status, a summary's duration, actuals and key files,
//! and a deliverable check's status.
//!
//! Plan status follows GSD's executor (`spec/plan.md`): `complete` or `halted` from its summary; `blocked` when a halted
//! plan it depends on (at any depth, in its own phase) or a dependency cycle stops it (`halt`); `executing` when a
//! recorded commit names it and it has no summary; `ready` otherwise. `draft` and `checked` need the plan-check
//! verdicts (r40). A value is absent where its inputs are: a summary with only `started` has no `duration`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::halt::{self, Node};
use crate::digest::scope_of;
use crate::passes::commits::Commit;
use crate::passes::plans::{of_kind, rows, text};
use crate::time::seconds;

pub fn value(entity: &str, field: &str, value: impl Into<Value>) -> PassValue {
    PassValue {
        entity: entity.to_string(),
        field: field.to_string(),
        value: value.into(),
    }
}

/// The summary of each plan, by plan id: the first by id when a plan has several (GSD has one file per plan).
pub fn summaries(input: &PassInput) -> BTreeMap<&str, &PassEntity> {
    let mut found: BTreeMap<&str, &PassEntity> = BTreeMap::new();
    let mut all: Vec<&PassEntity> = of_kind(input, "summary").collect();
    all.sort_by(|a, b| a.id.cmp(&b.id));
    for summary in all {
        found.entry(scope_of(&summary.id)).or_insert(summary);
    }
    found
}

/// A summary's `status`, case-folded: GSD reads it that way (`plan-dependency-graph.cts:47-60`).
fn status_of(summary: &PassEntity) -> String {
    text(summary, "status").unwrap_or_default().to_lowercase()
}

/// The status of every plan, by plan id.
pub fn plan_statuses<'a>(
    input: &'a PassInput,
    commits: &[Commit],
) -> BTreeMap<&'a str, &'static str> {
    let summaries = summaries(input);
    let mut phases: BTreeMap<&str, Vec<&PassEntity>> = BTreeMap::new();
    for plan in of_kind(input, "plan") {
        phases.entry(scope_of(&plan.id)).or_default().push(plan);
    }
    let mut blocked: BTreeSet<&str> = BTreeSet::new();
    for plans in phases.values() {
        let in_phase: BTreeSet<&str> = plans.iter().map(|p| p.id.as_str()).collect();
        let nodes: Vec<Node> = plans
            .iter()
            .map(|plan| Node {
                id: &plan.id,
                depends_on: input
                    .edges
                    .iter()
                    .filter(|e| e.label == "depends_on" && e.source == plan.id)
                    .map(|e| e.target.as_str())
                    .filter(|t| in_phase.contains(t))
                    .collect(),
                halted: summaries
                    .get(plan.id.as_str())
                    .is_some_and(|s| status_of(s) == "halted"),
            })
            .collect();
        blocked.extend(halt::blocked(&nodes));
    }
    let named: BTreeSet<&str> = commits.iter().filter_map(|c| c.plan.as_deref()).collect();
    of_kind(input, "plan")
        .map(|plan| {
            let id = plan.id.as_str();
            let status = match summaries.get(id).map(|s| status_of(s)).as_deref() {
                Some("halted") => "halted",
                Some("complete") => "complete",
                _ if blocked.contains(id) => "blocked",
                _ if named.contains(id) => "executing",
                _ => "ready",
            };
            (id, status)
        })
        .collect()
}

/// The paths a plan's commits created, and those they only modified, sorted.
fn key_files(commits: &[&Commit]) -> (Vec<String>, Vec<String>) {
    let created: BTreeSet<&String> = commits.iter().flat_map(|c| &c.created).collect();
    let touched: BTreeSet<&String> = commits.iter().flat_map(|c| &c.files).collect();
    (
        created.iter().map(|p| (*p).clone()).collect(),
        touched.difference(&created).map(|p| (*p).clone()).collect(),
    )
}

/// `completed` − `started` as GSD writes it: `"{N} min"` under an hour, `"{H}h {M}m"` from there on.
fn duration(summary: &PassEntity) -> Option<String> {
    let started = seconds(text(summary, "started")?)?;
    let completed = seconds(text(summary, "completed")?)?;
    let minutes = completed.checked_sub(started).filter(|s| *s >= 0)? / 60;
    Some(if minutes < 60 {
        format!("{minutes} min")
    } else {
        format!("{}h {}m", minutes / 60, minutes % 60)
    })
}

/// A check's status from the report: `fail` when a test named by its `ref` failed, `pass` when one passed, `unknown`
/// when none was recorded. No report, no status.
fn check_statuses(input: &PassInput, deliverable: &PassEntity) -> Vec<PassValue> {
    let Some(report) = input.test_results.as_ref() else {
        return Vec::new();
    };
    let tests = report.of(&deliverable.id);
    rows(deliverable, "checks")
        .iter()
        .enumerate()
        .filter_map(|(i, check)| {
            let reference = check.get("ref")?;
            let named = || tests.iter().filter(|t| &t.name == reference);
            let status = if named().any(|t| t.status == TestStatus::Failed) {
                "fail"
            } else if named().any(|t| t.status == TestStatus::Passed) {
                "pass"
            } else {
                "unknown"
            };
            Some(value(
                &deliverable.id,
                &format!("checks[{i}].status"),
                status,
            ))
        })
        .collect()
}

pub fn values(
    input: &PassInput,
    commits: &[Commit],
    statuses: &BTreeMap<&str, &'static str>,
) -> Vec<PassValue> {
    let mut out: Vec<PassValue> = statuses
        .iter()
        .map(|(plan, status)| value(plan, "status", *status))
        .collect();
    for task in of_kind(input, "task") {
        let named = commits
            .iter()
            .any(|c| c.task.as_deref() == Some(task.id.as_str()));
        let done = named || statuses.get(scope_of(&task.id)) == Some(&"complete");
        out.push(value(
            &task.id,
            "status",
            if done { "done" } else { "pending" },
        ));
    }
    for summary in of_kind(input, "summary") {
        let plan = scope_of(&summary.id);
        let of_plan: Vec<&Commit> = commits
            .iter()
            .filter(|c| c.plan.as_deref() == Some(plan))
            .collect();
        if let Some(duration) = duration(summary) {
            out.push(value(&summary.id, "duration", duration));
        }
        let tasks = of_kind(input, "task")
            .filter(|t| scope_of(&t.id) == plan)
            .count();
        out.push(value(
            &summary.id,
            "actuals",
            json!({ "tasks": tasks, "commits": of_plan.len() }),
        ));
        let (created, modified) = key_files(&of_plan);
        if !created.is_empty() {
            out.push(value(&summary.id, "files_created", created));
        }
        if !modified.is_empty() {
            out.push(value(&summary.id, "files_modified", modified));
        }
    }
    for deliverable in of_kind(input, "deliverable") {
        out.extend(check_statuses(input, deliverable));
    }
    out
}

#[cfg(test)]
mod tests;
