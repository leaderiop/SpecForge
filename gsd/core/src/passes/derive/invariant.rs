//! The computed values of an invariant (`spec/governance.md`): its `status`, from its verdict while it is in force,
//! and its `last_verified`, the time of a fresh verdict that it holds.
//!
//! An invariant is in force from the start, or once the phase its `holds_from` names has been complete in the phase's
//! own sense (`roadmap::PHASE_COMPLETE`, the values of the phase's `complete_when`), and for good. Until then it is
//! `pending`, whatever its verdict says, and the kind's own `complete_when` keeps its `proof` and `items` from being
//! checked.

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use super::verify::Verified;
use crate::kinds::PHASE_COMPLETE;
use crate::passes::plans::{link, of_kind, text};

/// The judged rule an invariant's verdict is recorded under.
pub const RULE: &str = "gsd/I536";

/// The outcomes a verdict on an invariant may record, the passing one first.
pub const OUTCOMES: &[&str] = &["holds", "violated", "uncertain"];

/// A judgement as the host reports it: its state, its outcome and when it was recorded.
pub type Verdict<'a> = (&'a str, Option<&'a str>, Option<&'a str>);

/// An invariant's status: `pending` while it is not in force; else `unverified` with no usable verdict (none,
/// missing, unrecorded or unreadable), `stale` when an input moved since, or the fresh verdict's outcome.
pub fn status(in_force: bool, verdict: Option<Verdict<'_>>) -> &'static str {
    if !in_force {
        return "pending";
    }
    match verdict {
        Some(("fresh", Some(outcome), _)) => OUTCOMES
            .iter()
            .find(|o| **o == outcome)
            .copied()
            .unwrap_or("unverified"),
        Some(("stale", ..)) => "stale",
        _ => "unverified",
    }
}

/// The judgement of [`RULE`] on `invariant`, when the host reports one.
fn verdict(invariant: &PassEntity) -> Option<Verdict<'_>> {
    invariant
        .judgements
        .iter()
        .find(|j| j.rule == RULE)
        .map(|j| (j.state.as_str(), j.outcome.as_deref(), j.at.as_deref()))
}

/// Whether `invariant` is in force: it names no `holds_from`, or the phase it names has been complete: it is complete
/// now, or it has a recorded verification. A pass keeps no history, so the latch is read from what outlives a slide
/// back: a verification is recorded once its phase ran, and GSD's gap closure keeps it while the plan it adds moves the
/// phase back to `in_progress`. A `holds_from` that resolves to no phase (the kernel reports it) keeps it pending.
fn in_force(
    input: &PassInput,
    invariant: &PassEntity,
    phases: &BTreeMap<String, &'static str>,
    verified: &BTreeMap<String, Verified>,
) -> bool {
    if text(invariant, "holds_from").is_none() {
        return true;
    }
    link(input, invariant, "holds_from").is_some_and(|phase| {
        phases
            .get(phase)
            .is_some_and(|status| PHASE_COMPLETE.contains(status))
            || verified.contains_key(phase)
    })
}

/// Each invariant's status, by id, given each phase's status by id (`progress::phase_statuses`) and its verification
/// (`verify::by_phase`).
pub fn statuses(
    input: &PassInput,
    phases: &BTreeMap<String, &'static str>,
    verified: &BTreeMap<String, Verified>,
) -> BTreeMap<String, &'static str> {
    of_kind(input, "invariant")
        .map(|invariant| {
            let status = status(
                in_force(input, invariant, phases, verified),
                verdict(invariant),
            );
            (invariant.id.clone(), status)
        })
        .collect()
}

/// `status` of each invariant, and `last_verified` of each that `holds`: its verdict's `at`.
pub fn values(input: &PassInput, statuses: &BTreeMap<String, &'static str>) -> Vec<PassValue> {
    let mut out = Vec::new();
    for invariant in of_kind(input, "invariant") {
        let Some(status) = statuses.get(&invariant.id).copied() else {
            continue;
        };
        out.push(value(&invariant.id, "status", status));
        if status == OUTCOMES[0]
            && let Some((.., Some(at))) = verdict(invariant)
        {
            out.push(value(&invariant.id, "last_verified", at));
        }
    }
    out
}

#[cfg(test)]
mod tests;
