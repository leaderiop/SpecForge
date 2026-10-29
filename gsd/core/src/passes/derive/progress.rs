//! The computed values of progress (`spec/project.md`): a requirement's status, a phase's flags, count and status
//! ladder, a milestone's state, stats, git range and progress, the project's lists and update time, a workstream's and
//! a context's status, and a retrospective's counts.
//!
//! The phase ladder (`ladder`) reads each phase's verification, so a milestone's `progress.phases_complete` counts the
//! phases whose verification passed.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::execute::{summaries, value};
use super::ladder;
use super::verify::Verified;
use crate::passes::commits::{Commit, target};
use crate::passes::plans::{name_of, of_kind, phase_of, rows, text};
use crate::time::seconds;

type Statuses<'a> = BTreeMap<&'a str, &'static str>;

/// A requirement is `complete` once a plan naming it is (GSD's `requirements mark-complete`), `blocked` when every
/// plan naming it is halted or blocked, `in_progress` when one is executing, else `pending`.
fn requirement_status(plans: &[&str], statuses: &Statuses) -> &'static str {
    let of = |plan: &&str| statuses.get(plan).copied().unwrap_or("ready");
    if plans.iter().any(|p| of(p) == "complete") {
        "complete"
    } else if !plans.is_empty() && plans.iter().all(|p| matches!(of(p), "halted" | "blocked")) {
        "blocked"
    } else if plans.iter().any(|p| of(p) == "executing") {
        "in_progress"
    } else {
        "pending"
    }
}

/// `min(plans_done/plans, phases_done/phases)` as a percent; a side with nothing counts 1 and nothing at all has no
/// percent (`computeProgressPercent`, `state-document.cts:861-878`).
fn percent(plans_done: usize, plans: usize, phases_done: usize, phases: usize) -> Option<i64> {
    if plans == 0 && phases == 0 {
        return None;
    }
    let fraction = |done: usize, total: usize| {
        if total == 0 {
            1.0
        } else {
            done as f64 / total as f64
        }
    };
    let smaller = fraction(plans_done, plans).min(fraction(phases_done, phases));
    Some((smaller * 100.0).round() as i64)
}

/// A phase and what is known of it.
struct Phase<'a> {
    id: &'a str,
    plans: Vec<&'a str>,
    summarised: usize,
    status: &'static str,
}

fn phases<'a>(
    input: &'a PassInput,
    summarised: &BTreeSet<&str>,
    verified: &BTreeMap<String, Verified>,
) -> Vec<Phase<'a>> {
    of_kind(input, "phase")
        .map(|phase| {
            let plans: Vec<&str> = of_kind(input, "plan")
                .filter(|p| phase_of(input, &p.id) == Some(phase.id.as_str()))
                .map(|p| p.id.as_str())
                .collect();
            let done = plans.iter().filter(|p| summarised.contains(*p)).count();
            Phase {
                id: &phase.id,
                status: ladder::status(plans.len(), done, verified.get(phase.id.as_str())),
                summarised: done,
                plans,
            }
        })
        .collect()
}

/// Whole seconds of a `ts` or a `date` (`YYYY-MM-DD`).
fn instant(text: &str) -> Option<i64> {
    seconds(text).or_else(|| seconds(&format!("{text}T00:00:00Z")))
}

fn completion_at(milestone: &PassEntity) -> Option<&str> {
    milestone.values.get("completion")?.get("at")?.as_str()
}

/// The commits of a milestone's plans, oldest first (the order `commits` gives).
fn stats(
    input: &PassInput,
    plans: &[&str],
    commits: &[&Commit],
    completion: Option<&str>,
    phases: usize,
) -> Value {
    let tasks = of_kind(input, "task")
        .filter(|t| plans.contains(&crate::digest::scope_of(&t.id)))
        .count();
    let mut out = json!({ "phases": phases, "plans": plans.len(), "tasks": tasks });
    if commits.is_empty() {
        return out;
    }
    let files: BTreeSet<&String> = commits.iter().flat_map(|c| &c.files).collect();
    out["files"] = files.len().into();
    out["loc"] = commits
        .iter()
        .map(|c| c.insertions + c.deletions)
        .sum::<i64>()
        .into();
    let first = commits.first().and_then(|c| seconds(&c.at));
    let end = completion
        .and_then(instant)
        .or_else(|| commits.last().and_then(|c| seconds(&c.at)));
    if let Some(days) = first
        .zip(end)
        .map(|(a, b)| (b - a) / 86_400)
        .filter(|d| *d >= 0)
    {
        out["days"] = days.into();
    }
    out
}

/// `` `feat(01-01)` → `feat(01-02)` ``: the first and last recorded `feat` commit, by the name of the plan each names.
fn git_range(commits: &[&Commit]) -> Option<String> {
    let label = |c: &&Commit| {
        let plan = c.plan.as_deref().map(name_of)?;
        Some(format!("`feat({plan})`"))
    };
    let mut feats = commits.iter().filter(|c| c.kind == "feat");
    let first = feats.next().and_then(label)?;
    let last = feats
        .next_back()
        .and_then(label)
        .unwrap_or_else(|| first.clone());
    Some(format!("{first} → {last}"))
}

pub fn values(
    input: &PassInput,
    commits: &[Commit],
    statuses: &Statuses,
    verified: &BTreeMap<String, Verified>,
) -> Vec<PassValue> {
    let mut out = Vec::new();
    let failing: BTreeSet<&str> = verified
        .iter()
        .filter(|(_, v)| v.status == "gaps_found")
        .map(|(phase, _)| phase.as_str())
        .collect();
    let summarised: BTreeSet<&str> = summaries(input).keys().copied().collect();
    // Requirements.
    let mut requirement_states: BTreeMap<&str, &'static str> = BTreeMap::new();
    for requirement in of_kind(input, "requirement") {
        let plans: Vec<&str> = input
            .edges
            .iter()
            .filter(|e| e.label == "requirements" && e.target == requirement.id)
            .filter(|e| statuses.contains_key(e.source.as_str()))
            .map(|e| e.source.as_str())
            .collect();
        let mut status = requirement_status(&plans, statuses);
        if status == "complete" && ladder::gaps_found(input, &requirement.id, &failing) {
            status = "gaps_found";
        }
        requirement_states.insert(&requirement.id, status);
        out.push(value(&requirement.id, "status", status));
    }
    // Phases.
    let phases = phases(input, &summarised, verified);
    for phase in &phases {
        let name = name_of(phase.id);
        let backlog = name.starts_with("999.");
        out.push(value(phase.id, "inserted", name.contains('.') && !backlog));
        out.push(value(phase.id, "backlog", backlog));
        out.push(value(
            phase.id,
            "plans_complete",
            format!("{}/{}", phase.summarised, phase.plans.len()),
        ));
        out.push(value(phase.id, "status", phase.status));
        if let Some(date) = ladder::completed(phase.status, verified.get(phase.id)) {
            out.push(value(phase.id, "completed", date));
        }
    }
    // Milestones.
    let mut milestone_state: BTreeMap<&str, &'static str> = BTreeMap::new();
    for milestone in of_kind(input, "milestone") {
        let mine: Vec<&Phase> = phases
            .iter()
            .filter(|p| target(input, p.id, "milestone") == Some(milestone.id.as_str()))
            // GSD leaves the sentinel 999 (backlog) phases out of a milestone's counts (#3167).
            .filter(|p| !name_of(p.id).starts_with("999."))
            .collect();
        let plans: Vec<&str> = mine.iter().flat_map(|p| p.plans.iter().copied()).collect();
        let recorded: Vec<&Commit> = commits
            .iter()
            .filter(|c| c.plan.as_deref().is_some_and(|p| plans.contains(&p)))
            .collect();
        let completion = completion_at(milestone);
        let state = if milestone.fields.contains_key("abandoned") {
            "abandoned"
        } else if milestone.fields.contains_key("completion") {
            "shipped"
        } else if mine.iter().any(|p| p.status != "not_started") {
            "in_progress"
        } else {
            "planned"
        };
        milestone_state.insert(&milestone.id, state);
        out.push(value(&milestone.id, "state", state));
        if let Some(at) = completion {
            out.push(value(&milestone.id, "shipped", at));
        }
        out.push(value(
            &milestone.id,
            "stats",
            stats(input, &plans, &recorded, completion, mine.len()),
        ));
        if let Some(range) = git_range(&recorded) {
            out.push(value(&milestone.id, "git_range", range));
        }
        let done = plans
            .iter()
            // A halted plan stays counted, as the phase rows and gsd-tools count it (#2830).
            .filter(|p| matches!(statuses.get(*p), Some(&"complete" | &"halted")))
            .count();
        let complete_phases = mine.iter().filter(|p| p.status == "complete").count();
        let mut progress = json!({
            "phases": mine.len(),
            "phases_complete": complete_phases,
            "plans": plans.len(),
            "plans_complete": done,
        });
        if let Some(percent) = percent(done, plans.len(), complete_phases, mine.len()) {
            progress["percent"] = percent.into();
        }
        out.push(value(&milestone.id, "progress", progress));
        // Its retrospective counts what it holds.
        for retro in of_kind(input, "retrospective")
            .filter(|r| target(input, &r.id, "milestone") == Some(milestone.id.as_str()))
        {
            out.push(value(&retro.id, "phases", mine.len()));
            out.push(value(&retro.id, "plans", plans.len()));
        }
    }
    // Workstreams.
    for workstream in of_kind(input, "workstream") {
        let states: Vec<&str> = of_kind(input, "milestone")
            .filter(|m| target(input, &m.id, "workstream") == Some(workstream.id.as_str()))
            .filter_map(|m| milestone_state.get(m.id.as_str()).copied())
            .collect();
        let done = !states.is_empty() && states.iter().all(|s| *s == "shipped");
        out.push(value(
            &workstream.id,
            "status",
            if done { "complete" } else { "active" },
        ));
    }
    // Contexts.
    for context in of_kind(input, "context") {
        let ready =
            text(context, "boundary").is_some() && !rows(context, "canonical_refs").is_empty();
        let status = if ready {
            "ready_for_planning"
        } else {
            "gathering"
        };
        out.push(value(&context.id, "status", status));
    }
    // The project.
    out.extend(ladder::windows(input));
    for project in of_kind(input, "project") {
        let ids = |keep: &dyn Fn(&PassEntity, &str) -> bool| -> Vec<String> {
            let mut found: Vec<String> = of_kind(input, "requirement")
                .filter(|r| {
                    keep(
                        r,
                        requirement_states
                            .get(r.id.as_str())
                            .copied()
                            .unwrap_or("pending"),
                    )
                })
                .map(|r| r.id.clone())
                .collect();
            found.sort();
            found
        };
        for (field, list) in [
            ("validated", ids(&|_, status| status == "complete")),
            (
                "active",
                ids(&|r, status| text(r, "tier") == Some("v1") && status != "complete"),
            ),
        ] {
            if !list.is_empty() {
                out.push(value(&project.id, field, list));
            }
        }
        let latest = commits
            .iter()
            .map(|c| c.at.as_str())
            .chain(of_kind(input, "summary").filter_map(|s| text(s, "completed")))
            .filter(|t| seconds(t).is_some())
            .max_by_key(|t| seconds(t));
        if let Some(at) = latest {
            out.push(value(&project.id, "updated", at));
        }
    }
    out
}

#[cfg(test)]
mod tests;
