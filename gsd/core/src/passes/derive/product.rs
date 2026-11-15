//! The computed values of the product kinds (`spec/product.md`, ADR 0357): a term's `used_in`, the entities whose
//! prose holds its name or one of its aliases (`passes::vocabulary`); a journey's `last_uat`, the latest session of a
//! UAT test that names it, and `covered`, read from the requirements it names ([`Names`]) and the requirement thread.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::{link, of_kind, rows, strings, targets, text, written_target};
use crate::passes::thread::{Proof, requirement_proven};
use crate::passes::vocabulary::{prose, uses, writers};

/// The product kinds' computed values. `requirements` and `proofs` are derive's requirement states and thread proofs,
/// computed in this pass: its input does not hold them.
pub(crate) fn values(
    input: &PassInput,
    requirements: &BTreeMap<&str, &'static str>,
    proofs: &BTreeMap<&str, Proof>,
) -> Vec<PassValue> {
    let mut out = used_in(input);
    out.extend(last_uat(input));
    let covered = covered(input, requirements, proofs);
    out.extend(
        covered
            .iter()
            .map(|(journey, covered)| value(journey, "covered", *covered)),
    );
    out
}

/// A requirement a journey names, as it reads in the graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Named<'a> {
    /// A step that names no requirement.
    Nothing,
    /// A written reference the graph did not resolve (the host reports it).
    Unresolved,
    /// The requirement the reference resolves to.
    Requirement(&'a str),
}

/// The requirements a journey names (ADR 0357 D19): each step's `requirement` and the journey's own `requirements`.
pub(crate) struct Names<'a> {
    /// One per step, in order.
    pub steps: Vec<Named<'a>>,
    /// One per item of `requirements`, in order (never [`Named::Nothing`]).
    pub whole: Vec<Named<'a>>,
}

impl<'a> Names<'a> {
    /// Each written reference matched to its resolved edge (`plans::written_target`).
    pub(crate) fn of(input: &'a PassInput, journey: &PassEntity) -> Self {
        let resolve = |label: &str, written: &str| {
            written_target(&targets(input, &journey.id, label), &journey.id, written)
                .map_or(Named::Unresolved, Named::Requirement)
        };
        let steps = rows(journey, "steps")
            .iter()
            .map(|row| match row.get("requirement").map(|r| r.trim()) {
                Some(written) if !written.is_empty() => resolve("steps.requirement", written),
                _ => Named::Nothing,
            })
            .collect();
        let whole = strings(journey, "requirements")
            .iter()
            .map(|written| resolve("requirements", written))
            .collect();
        Self { steps, whole }
    }

    /// The resolved requirements, each once, in id order.
    pub(crate) fn requirements(&self) -> BTreeSet<&'a str> {
        self.steps
            .iter()
            .chain(&self.whole)
            .filter_map(|named| match named {
                Named::Requirement(id) => Some(*id),
                _ => None,
            })
            .collect()
    }
}

/// Each term's `used_in`: the ids of the entities whose prose holds its `name` or an alias as a whole word or phrase,
/// sorted, and `[]` when none, so "unused" is never "unknown". An `avoid` word is no use.
fn used_in(input: &PassInput) -> Vec<PassValue> {
    let written: Vec<(&str, Vec<(String, String)>)> =
        writers(input).map(|e| (e.id.as_str(), prose(e))).collect();
    of_kind(input, "term")
        .map(|term| {
            let words: Vec<String> = text(term, "name")
                .map(str::to_string)
                .into_iter()
                .chain(strings(term, "aliases"))
                .collect();
            let mut ids: Vec<&str> = written
                .iter()
                .filter(|(_, places)| words.iter().any(|w| uses(places, w).is_some()))
                .map(|(id, _)| *id)
                .collect();
            ids.sort_unstable();
            ids.dedup();
            value(&term.id, "used_in", ids)
        })
        .collect()
}

/// Each journey a UAT test names (`uat_test.journey`): its `last_uat`, the session of such a test that started last. A
/// session with no `started` is older than every dated one, and of two that started at once the greater id is the
/// later. Any result counts: the session's own `status` says how it went.
fn last_uat(input: &PassInput) -> Vec<PassValue> {
    let mut latest: BTreeMap<&str, (Option<i64>, &str)> = BTreeMap::new();
    for test in of_kind(input, "uat_test") {
        let (Some(journey), Some(session)) =
            (link(input, test, "journey"), link(input, test, "session"))
        else {
            continue;
        };
        let started = input
            .entities
            .iter()
            .find(|e| e.id == session)
            .and_then(|s| text(s, "started"))
            .and_then(crate::time::seconds);
        let candidate = (started, session);
        let best = latest.entry(journey).or_insert(candidate);
        *best = (*best).max(candidate);
    }
    latest
        .into_iter()
        .map(|(journey, (_, session))| value(journey, "last_uat", session))
        .collect()
}

/// Each journey's `covered` (ADR 0357 D20): it names a requirement, every reference it writes resolves, each step
/// names a requirement or the journey lists `requirements`, and each requirement it names is proven by the
/// requirement thread (`thread::requirement_proven` on derive's states and proofs, the predicate gsd/E515 reads).
fn covered<'a>(
    input: &'a PassInput,
    requirements: &BTreeMap<&str, &'static str>,
    proofs: &BTreeMap<&str, Proof>,
) -> BTreeMap<&'a str, bool> {
    let proven = |requirement: &str| {
        proofs
            .get(requirement)
            .is_some_and(|proof| requirement_proven(requirements.get(requirement).copied(), *proof))
    };
    of_kind(input, "journey")
        .map(|journey| {
            let names = Names::of(input, journey);
            let named = names.requirements();
            let resolved = !names
                .steps
                .iter()
                .chain(&names.whole)
                .any(|n| *n == Named::Unresolved);
            let carried = !names.whole.is_empty() || !names.steps.contains(&Named::Nothing);
            let covered =
                !named.is_empty() && resolved && carried && named.iter().all(|r| proven(r));
            (journey.id.as_str(), covered)
        })
        .collect()
}

#[cfg(test)]
mod tests;
