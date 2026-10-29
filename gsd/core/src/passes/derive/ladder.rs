//! The phase ladder, the requirements it can fail and the project's window counts (`spec/project.md`).
//!
//! The ladder is `phase-status.cts:174-197`, first match: `complete` when the phase's verification is `passed`;
//! `not_started` with no plan; `planned` with no summary; `in_progress` with fewer summaries than plans;
//! `needs_review` when the verification is `human_needed`; else `executed`. UAT does not feed it. `deferred` is
//! never derived. A requirement a plan completed is `gaps_found` while a phase it traces to has a `gaps_found`
//! verification (`milestone.cts:453-514`).

use std::collections::BTreeSet;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use super::verify::Verified;
use crate::passes::plans::{link, of_kind};

pub fn status(plans: usize, summarised: usize, verification: Option<&Verified>) -> &'static str {
    let verified = verification.map(|v| v.status);
    match (plans, summarised) {
        _ if verified == Some("passed") => "complete",
        (0, _) => "not_started",
        (_, 0) => "planned",
        (p, s) if s < p => "in_progress",
        _ if verified == Some("human_needed") => "needs_review",
        _ => "executed",
    }
}

/// `phase.completed`: the date of the latest verdict, once the phase is complete.
pub fn completed(phase_status: &str, verification: Option<&Verified>) -> Option<String> {
    let at = verification?.at.as_deref()?;
    (phase_status == "complete").then(|| at.chars().take(10).collect())
}

/// The phases a requirement traces to whose verification found gaps.
pub fn gaps_found(input: &PassInput, requirement: &str, failing: &BTreeSet<&str>) -> bool {
    input.edges.iter().any(|e| {
        e.label == "requirements" && e.target == requirement && failing.contains(e.source.as_str())
    })
}

/// `open`, `waived` or `fixed` (D-r39-8): a waiver wins over a fix, and a fix is `fixed_at` or a disposed proposal.
pub fn window_status(input: &PassInput, window: &PassEntity) -> &'static str {
    let set = |field: &str| {
        window
            .fields
            .get(field)
            .is_some_and(|v| !v.trim().is_empty())
    };
    let disposed = link(input, window, "refactor").is_some_and(|proposal| {
        of_kind(input, "refactor_proposal")
            .any(|p| p.id == proposal && p.fields.contains_key("disposition"))
    });
    if set("waiver") {
        "waived"
    } else if set("fixed_at") || disposed {
        "fixed"
    } else {
        "open"
    }
}

/// `project.windows_*`: the project's windows by status.
pub fn windows(input: &PassInput) -> Vec<PassValue> {
    let statuses: Vec<&str> = of_kind(input, "broken_window")
        .map(|w| window_status(input, w))
        .collect();
    let count = |status: &str| statuses.iter().filter(|s| **s == status).count();
    of_kind(input, "project")
        .flat_map(|project| {
            [
                ("windows_open", count("open")),
                ("windows_waived", count("waived")),
                ("windows_fixed", count("fixed")),
                ("windows_total", statuses.len()),
            ]
            .map(|(field, n)| value(&project.id, field, n))
        })
        .collect()
}

#[cfg(test)]
mod tests;
