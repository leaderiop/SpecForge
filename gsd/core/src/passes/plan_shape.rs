//! The plan-shape pass: the rules that read one plan with its tasks, checkpoints and must-haves. `gsd/W323` (its
//! name), `gsd/E302` (its requirements), `gsd/E303` (a task's parts), `gsd/W304` and `gsd/E322` (how many tasks), `gsd/E307` (a checkpoint's
//! options; its required fields are declarative, `rules.rs`), `gsd/W308` (a one-way door) and `gsd/W309` (its
//! must-haves). Each reads what GSD's `verify plan-structure` and the plan checker's dimensions read.
//!
//! Also ports gsd-core bin/lib modules: edge-probe.cjs, gate-prohibition-enforcement.cjs, probe-core.cjs,
//! prohibition-enforcement.cjs.

mod checkpoint;

use specforge_extension_sdk::prelude::*;

use crate::digest::scope_of;

use super::plans::{block, children, name_of, of_kind, phase_of, strings, text};

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("plan_shape", |p| {
        p.phase("check").run(check);
    });
}

fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = names(input);
    found.extend(requirements(input));
    found.extend(task_parts(input));
    found.extend(task_counts(input));
    found.extend(checkpoint::check(input));
    found.extend(one_way_doors(input));
    found.extend(must_haves(input));
    found
}

/// GSD's quick id: `YYMMDD-xxx`, six digits, a dash, three base-36 digits (`src/init.cts`, lowercase).
fn is_quick_id(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[..6].iter().all(u8::is_ascii_digit)
        && bytes[6] == b'-'
        && bytes[7..]
            .iter()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase())
}

/// A phase plan is `<phase>-<NN>`: the phase's name, a dash, and two or more digits (`02.1-01` in phase `02.1`).
fn is_phase_plan_name(name: &str, phase: &str) -> bool {
    name.strip_prefix(phase)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|number| number.len() >= 2 && number.bytes().all(|b| b.is_ascii_digit()))
}

/// W323: a plan is named `<phase>-<NN>` in its phase, and by its quick id without one. A forgotten `phase` would
/// otherwise silently make a quick plan (the trap `gsd/E206` closes for a research).
fn names(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .filter_map(|plan| {
            let name = name_of(&plan.id);
            match phase_of(input, &plan.id) {
                Some(phase) if !is_phase_plan_name(name, name_of(phase)) => Some(
                    PassDiagnostic::new(
                        "W323",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' is in phase '{phase}' but is not named {}-NN",
                            plan.id,
                            name_of(phase)
                        ),
                    )
                    .with_suggestion(format!(
                        "name it {}-NN (two or more digits), or put it in the phase its name says",
                        name_of(phase)
                    )),
                ),
                Some(_) => None,
                None if plan.fields.contains_key("phase") || is_quick_id(name) => None,
                None => Some(
                    PassDiagnostic::new(
                        "W323",
                        PassSeverity::Warning,
                        format!(
                            "plan '{}' has no phase and is not named by a quick id (YYMMDD-xxx)",
                            plan.id
                        ),
                    )
                    .with_suggestion("add its phase, or name it by its quick id"),
                ),
            }
            .map(|d| d.with_entity(&plan.id))
        })
        .collect()
}

/// E302: a phase plan names at least one requirement (GSD's plan template makes `requirements` non-empty, and the
/// checker's D1 blocks a plan that covers none). A quick plan is exempt: it names no roadmap requirement. It cannot
/// be declarative: no declarative check reads "the field `phase` is unset".
fn requirements(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .filter(|plan| {
            plan.fields.contains_key("phase") && strings(plan, "requirements").is_empty()
        })
        .map(|plan| {
            PassDiagnostic::new(
                "E302",
                PassSeverity::Error,
                format!("plan '{}' names no requirement", plan.id),
            )
            .with_suggestion("list the requirements the plan delivers in requirements")
            .with_entity(&plan.id)
        })
        .collect()
}

/// E303: an `auto`, `tdd` or `tracer` task has files and a verify of some form (`action` and `done` are required
/// fields, so the host reports those). `manual` is exempt. GSD's `verify plan-structure` warns on a missing
/// `<verify>`, `<files>` and `<done>` and errors on a missing `<action>`; the plan checker's D2 blocks all four, and
/// that is the severity kept here.
fn task_parts(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "task")
        .filter(|t| {
            matches!(
                t.fields.get("type").map(String::as_str),
                Some("auto" | "tdd" | "tracer")
            )
        })
        .filter_map(|task| {
            let mut missing = Vec::new();
            if strings(task, "files").is_empty() {
                missing.push("files");
            }
            let verified = task
                .values
                .get("verify")
                .and_then(|v| v.as_array())
                .is_some_and(|v| !v.is_empty())
                || text(task, "verify_text").is_some()
                || text(task, "verify_human").is_some();
            if !verified {
                missing.push("verify (verify, verify_text or verify_human)");
            }
            (!missing.is_empty()).then(|| {
                PassDiagnostic::new(
                    "E303",
                    PassSeverity::Error,
                    format!(
                        "task '{}' of type {} has no {}",
                        task.id,
                        task.fields.get("type").map_or("", String::as_str),
                        missing.join(" and no ")
                    ),
                )
                .with_suggestion(
                    "an auto, tdd or tracer task names the files it touches and how it is verified",
                )
                .with_entity(&task.id)
            })
        })
        .collect()
}

/// W304 and E322: `n` is the plan's tasks plus its checkpoints, as GSD's `task_count`. Four is a warning (D5's
/// target is 2-3), five or more a blocker; never both.
fn task_counts(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .filter_map(|plan| {
            let n = children(input, &plan.id, "task").len()
                + children(input, &plan.id, "checkpoint").len();
            let (code, severity, message) = match n {
                4 => (
                    "W304",
                    PassSeverity::Warning,
                    format!("plan '{}' has 4 tasks; 2-3 is the target", plan.id),
                ),
                5.. => (
                    "E322",
                    PassSeverity::Error,
                    format!("plan '{}' has {n} tasks; a plan has at most 4", plan.id),
                ),
                _ => return None,
            };
            Some(
                PassDiagnostic::new(code, severity, message)
                    .with_suggestion(
                        "split the plan: each plan stays within about 50% of a context window",
                    )
                    .with_entity(&plan.id),
            )
        })
        .collect()
}

/// The number in a task's name (`T12` is 12), the order of the tasks of a plan.
fn number(name: &str) -> Option<u64> {
    name.trim_start_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()
}

/// W308: a task rated `one_way` has an earlier decision checkpoint in its plan, or names the decision that settles
/// it. A checkpoint without `after` stands before the first task. GSD warns, "never error" (`src/verify.cts:1397`).
fn one_way_doors(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .flat_map(|plan| {
            // A decision checkpoint's position: after task N of this plan. One whose `after` is a task of another plan
            // (E307) or has no number stands before nothing.
            let decisions: Vec<Option<u64>> = children(input, &plan.id, "checkpoint")
                .into_iter()
                .filter(|c| c.fields.get("kind").map(String::as_str) == Some("decision"))
                .map(|c| {
                    match input
                        .edges
                        .iter()
                        .find(|e| e.source == c.id && e.label == "after")
                    {
                        Some(e) if scope_of(&e.target) == plan.id => number(name_of(&e.target)),
                        Some(_) => None,
                        None => Some(0),
                    }
                })
                .collect();
            children(input, &plan.id, "task")
                .into_iter()
                .filter(|task| {
                    let door = block(task, "reversibility");
                    door.get("rating").map(String::as_str) == Some("one_way")
                        && door.get("settled_by").is_none_or(|s| s.is_empty())
                })
                .filter(|task| {
                    let at = number(name_of(&task.id));
                    // A checkpoint "after task 0" stands before every task.
                    !decisions
                        .iter()
                        .any(|after| matches!((after, at), (Some(after), Some(at)) if *after < at))
                })
                .map(|task| {
                    PassDiagnostic::new(
                        "W308",
                        PassSeverity::Warning,
                        format!(
                            "task '{}' rated one_way has no preceding checkpoint:decision — a one-way door must be confirmed before the agent walks through it (unless the decision is already settled in CONTEXT.md, an ingested ADR, or an answered checkpoint)",
                            task.id
                        ),
                    )
                    .with_suggestion("add a decision checkpoint before the task, or name the decision that settles it in reversibility.settled_by")
                    .with_entity(&task.id)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// W309: a plan has a truth and an artifact.
fn must_haves(input: &PassInput) -> Vec<PassDiagnostic> {
    of_kind(input, "plan")
        .filter_map(|plan| {
            let missing: Vec<&str> = ["truth", "artifact"]
                .into_iter()
                .filter(|kind| children(input, &plan.id, kind).is_empty())
                .collect();
            (!missing.is_empty()).then(|| {
                PassDiagnostic::new(
                    "W309",
                    PassSeverity::Warning,
                    format!("plan '{}' has no {}", plan.id, missing.join(" and no ")),
                )
                .with_suggestion("a plan states what must be true and which files carry it: add a truth and an artifact")
                .with_entity(&plan.id)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
