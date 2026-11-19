//! The wave pass: `plan.wave` and `plan.autonomous`, computed as `gsd-tools phase-plan-index` does, and
//! `plan.strict_wave`, where a plan runs once the plans of its phase that share a path are kept apart (ADR 0378 D1).
//!
//! A wave is Kahn's longest-path level over the phase's in-phase `depends_on` DAG, plus 1; the offset is 0 when any
//! plan of the phase writes `wave0 true` (`src/phase.cts:1112-1148`). An edge to a plan of another phase is dropped,
//! as GSD drops a token that does not resolve in the phase. A cycle errors the whole phase upstream, so none of its
//! plans gets a wave here (gsd/E102 reports the cycle). A quick plan is in no phase and has no wave. GSD reads
//! `autonomous:` from the frontmatter; here it is computed, false exactly when the plan has a checkpoint, which is what
//! GSD's structure check demands of every conforming plan.
//!
//! The strict wave is the same level function over the `depends_on` edges plus an implied edge from the earlier plan
//! (by wave, then id) to the later one of every pair of the phase that shares a path; `wave` never moves (ADR 0370 D1).
//!
//! Also ports gsd-core bin/lib modules: file-overlap-partitioner.cjs.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::plans::{of_kind, strings, touched};
use crate::scope::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("waves", |p| {
        p.compute()
            .computes(&["plan.wave", "plan.autonomous", "plan.strict_wave"])
            .run(compute);
    });
}

/// One phase's plans placed twice: by their `depends_on` edges (`wave`) and once the plans that share a path are kept
/// apart (`strict`). Every map is keyed by plan id.
#[derive(Debug)]
pub struct Placed<'a> {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the values computed on a phase from its placement read it"
        )
    )]
    pub phase: &'a str,
    pub plans: BTreeMap<&'a str, &'a PassEntity>,
    /// gsd-core's wave.
    pub wave: BTreeMap<&'a str, i64>,
    /// The in-phase `depends_on` edges: the plans each plan names.
    pub depends: BTreeMap<&'a str, BTreeSet<&'a str>>,
    /// The paths each pair shares, keyed `(later, earlier)` by (wave, id); a pair that shares nothing is absent.
    pub shared: BTreeMap<(&'a str, &'a str), BTreeSet<String>>,
    /// The plans each plan runs after: its `depends` plus the earlier plan of every pair it shares a path in.
    pub before: BTreeMap<&'a str, BTreeSet<&'a str>>,
    /// The strict wave: the levels over `before`, with the offset of `wave`.
    pub strict: BTreeMap<&'a str, i64>,
}

/// The placement of every phase whose plans have waves, in phase id order. A quick plan is in no phase; a phase whose
/// `depends_on` edges hold a cycle has no wave, so it is left out.
pub fn placed(input: &PassInput) -> Vec<Placed<'_>> {
    let mut phases: BTreeMap<&str, BTreeMap<&str, &PassEntity>> = BTreeMap::new();
    for plan in of_kind(input, "plan").filter(|p| !scope_of(&p.id).is_empty()) {
        phases
            .entry(scope_of(&plan.id))
            .or_default()
            .insert(&plan.id, plan);
    }
    // A task's file outside its plan's lists is only a warning (gsd/W306), so the plan's lists alone would miss it.
    let mut paths: BTreeMap<&str, BTreeSet<String>> = phases
        .values()
        .flat_map(BTreeMap::values)
        .map(|plan| (plan.id.as_str(), touched(plan)))
        .collect();
    for task in of_kind(input, "task") {
        if let Some(own) = paths.get_mut(scope_of(&task.id)) {
            own.extend(strings(task, "files"));
        }
    }
    phases
        .into_iter()
        .filter_map(|(phase, plans)| place(input, phase, plans, &paths))
        .collect()
}

fn place<'a>(
    input: &'a PassInput,
    phase: &'a str,
    plans: BTreeMap<&'a str, &'a PassEntity>,
    paths: &BTreeMap<&str, BTreeSet<String>>,
) -> Option<Placed<'a>> {
    let ids: BTreeSet<&str> = plans.keys().copied().collect();
    let mut depends: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in input.edges.iter().filter(|e| e.label == "depends_on") {
        if ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()) {
            depends
                .entry(&edge.source)
                .or_default()
                .insert(&edge.target);
        }
    }
    let offset = if plans
        .values()
        .any(|p| p.fields.get("wave0").map(String::as_str) == Some("true"))
    {
        0
    } else {
        1
    };
    let shift = |level: BTreeMap<&'a str, i64>| -> BTreeMap<&'a str, i64> {
        level.into_iter().map(|(id, l)| (id, l + offset)).collect()
    };
    let wave = shift(levels(&ids, &depends)?);
    // The earlier plan of a pair is the one of smaller (wave, id). A `depends_on` edge already goes from a smaller wave
    // to a larger one, so every edge of `before` does: the implied edges close no cycle, and `levels` places them all.
    let mut order: Vec<&str> = ids.iter().copied().collect();
    order.sort_by_key(|id| (wave[id], *id));
    let none = BTreeSet::new();
    let paths_of = |id: &str| paths.get(id).unwrap_or(&none);
    let mut shared = BTreeMap::new();
    let mut before = depends.clone();
    for (at, later) in order.iter().enumerate() {
        for earlier in &order[..at] {
            let common: BTreeSet<String> = paths_of(later)
                .intersection(paths_of(earlier))
                .cloned()
                .collect();
            if !common.is_empty() {
                before.entry(*later).or_default().insert(*earlier);
                shared.insert((*later, *earlier), common);
            }
        }
    }
    let strict = shift(levels(&ids, &before)?);
    Some(Placed {
        phase,
        plans,
        wave,
        depends,
        shared,
        before,
        strict,
    })
}

/// Kahn's longest-path level of each id over `before` (the ids each one runs after), from 0; `None` when some id is
/// never placed, which is a cycle. The one level function of both `wave` and `strict_wave`.
fn levels<'a>(
    ids: &BTreeSet<&'a str>,
    before: &BTreeMap<&'a str, BTreeSet<&'a str>>,
) -> Option<BTreeMap<&'a str, i64>> {
    let mut level: BTreeMap<&str, i64> = BTreeMap::new();
    while level.len() < ids.len() {
        let ready: Vec<(&str, i64)> = ids
            .iter()
            .filter(|id| !level.contains_key(**id))
            .filter_map(|id| {
                let deps = before.get(id).into_iter().flatten();
                let below: Option<Vec<i64>> = deps.map(|d| level.get(d).copied()).collect();
                Some((*id, below?.into_iter().max().map_or(0, |m| m + 1)))
            })
            .collect();
        if ready.is_empty() {
            return None;
        }
        level.extend(ready);
    }
    Some(level)
}

fn compute(input: &PassInput) -> PassOutput {
    let with_checkpoint: BTreeSet<&str> = of_kind(input, "checkpoint")
        .map(|c| scope_of(&c.id))
        .collect();
    let placed = placed(input);
    let mut values = Vec::new();
    for plan in of_kind(input, "plan") {
        let id = plan.id.as_str();
        if let Some(p) = placed.iter().find(|p| p.plans.contains_key(id)) {
            for (field, wave) in [("wave", p.wave[id]), ("strict_wave", p.strict[id])] {
                values.push(PassValue {
                    entity: plan.id.clone(),
                    field: field.into(),
                    value: wave.into(),
                });
            }
        }
        values.push(PassValue {
            entity: plan.id.clone(),
            field: "autonomous".into(),
            value: (!with_checkpoint.contains(id)).into(),
        });
    }
    PassOutput {
        values,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
