//! E206: a phase has at most one context, phase spec, discussion log and research, a quick plan at most one context
//! and research, and the project at most one research. GSD keeps one such file in a phase (or quick task) directory;
//! the kernel's scopes would allow two entities of a kind in a phase, and the drift pass needs "the phase's context".
//!
//! A context, a decision and a research name their owner: a context exactly one of `phase` and `plan`, a decision and a
//! research at most one, and a `plan` must be a quick plan (one with no phase). A quick plan's research is not named
//! `RESEARCH`, the name of the project's research, which the research pass exempts. A quick plan's context and
//! research are in the project scope, as their quick plan is, so they are counted by the plan they name, not by their
//! scope.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use crate::digest::{owner, quick_plan, scope_of};

/// The kinds a phase (or, for `context` and `research`, a quick plan; for `research`, the project) holds one of.
const ONE_PER_SCOPE: &[&str] = &["context", "phase_spec", "discussion", "research"];

/// The kinds that may name a quick plan in `plan` instead of a phase in `phase`.
const OWNED: &[&str] = &["context", "decision", "research"];

/// The one name a research without `phase` (the project's research) may have.
const PROJECT_RESEARCH: &str = "RESEARCH";

fn e206(id: &str, message: String, suggestion: String) -> PassDiagnostic {
    PassDiagnostic::new("E206", PassSeverity::Error, message)
        .with_suggestion(suggestion)
        .with_entity(id)
}

/// The owner rules: both or (for a context) neither of `phase` and `plan`, a `plan` that is a phase's, and a quick
/// plan's research named `RESEARCH`.
fn owners(input: &PassInput) -> Vec<PassDiagnostic> {
    let has_phase = |id: &str| {
        input
            .edges
            .iter()
            .any(|e| e.source == id && e.label == "phase")
    };
    let mut found = Vec::new();
    for entity in input
        .entities
        .iter()
        .filter(|e| OWNED.contains(&e.kind.as_str()))
    {
        let (id, kind) = (entity.id.as_str(), entity.kind.as_str());
        let plan = quick_plan(input, id);
        let phase = !scope_of(id).is_empty();
        if phase && plan.is_some() {
            found.push(e206(
                id,
                format!("{kind} '{id}' names both a phase and a plan; it belongs to one of them"),
                "keep `phase` for a phase's, or `plan` for a quick plan's".to_string(),
            ));
        } else if kind == "context" && !phase && plan.is_none() {
            found.push(e206(
                id,
                format!("context '{id}' names neither a phase nor a quick plan"),
                "add its `phase`, or its quick plan in `plan` (/gsd-quick --discuss)".to_string(),
            ));
        }
        if kind == "research" && id == PROJECT_RESEARCH {
            if let Some(plan) = plan {
                found.push(e206(
                    id,
                    format!("research '{id}' names quick plan '{plan}', but '{PROJECT_RESEARCH}' is the name of the project's research"),
                    format!("rename it '{plan}-RESEARCH'"),
                ));
            }
        }
        if let Some(plan) = plan.filter(|p| has_phase(p)) {
            found.push(e206(
                id,
                format!(
                    "{kind} '{id}' names plan '{plan}', which is not a quick plan: it has a phase"
                ),
                "name the phase in `phase` instead, or a quick plan (one with no phase) in `plan`"
                    .to_string(),
            ));
        }
    }
    found
}

pub fn check(input: &PassInput) -> Vec<PassDiagnostic> {
    let mut found = owners(input);
    let mut by_owner: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
    for entity in &input.entities {
        if let Some(kind) = ONE_PER_SCOPE.iter().find(|k| **k == entity.kind) {
            by_owner
                .entry((kind, owner(input, entity)))
                .or_default()
                .push(&entity.id);
        }
    }
    for ((kind, owner), mut ids) in by_owner {
        // A research without `phase` or `plan` is the project's only when it is named `RESEARCH`; any other is a
        // forgotten owner, reported whether or not the project has its own. A context with neither is reported above.
        if owner.is_empty() {
            if kind == "research" {
                for id in ids.iter().filter(|id| **id != PROJECT_RESEARCH) {
                    found.push(e206(
                        id,
                        format!("research '{id}' has no phase, so it is the project's research, which must be named '{PROJECT_RESEARCH}'"),
                        format!("add its phase (or its quick plan in `plan`), or rename it '{PROJECT_RESEARCH}' if it is the project's research"),
                    ));
                }
            }
            continue;
        }
        ids.sort_unstable();
        let Some((first, rest)) = ids.split_first() else {
            continue;
        };
        let place = if scope_of(first) == owner {
            format!("phase '{owner}'")
        } else {
            format!("quick plan '{owner}'")
        };
        let kind_name = kind.replace('_', " ");
        for id in rest {
            found.push(e206(
                id,
                format!("{kind_name} '{id}' is a second {kind_name} of {place}; its {kind_name} is '{first}'"),
                format!(
                    "keep one {kind_name} for {place}: merge '{id}' into '{first}' or remove it"
                ),
            ));
        }
    }
    found
}
