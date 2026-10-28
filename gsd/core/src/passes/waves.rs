//! The wave pass: `plan.wave` and `plan.autonomous`, computed as `gsd-tools phase-plan-index` does.
//!
//! A wave is Kahn's longest-path level over the phase's in-phase `depends_on` DAG, plus 1; the offset is 0 when any
//! plan of the phase writes `wave0 true` (`src/phase.cts:1112-1148`). An edge to a plan of another phase is dropped,
//! as GSD drops a token that does not resolve in the phase. A cycle errors the whole phase upstream, so none of its
//! plans gets a wave here (gsd/E102 reports the cycle). A quick plan is in no phase and has no wave. GSD reads
//! `autonomous:` from the frontmatter; here it is computed, false exactly when the plan has a checkpoint, which is what
//! GSD's structure check demands of every conforming plan.

use std::collections::{BTreeMap, BTreeSet};

use specforge_extension_sdk::prelude::*;

use super::plans::of_kind;
use crate::digest::scope_of;

pub fn declare(c: &mut ContributionsBuilder) {
    c.pass("waves", |p| {
        p.compute()
            .computes(&["plan.wave", "plan.autonomous"])
            .run(compute);
    });
}

/// The wave of each plan that has one, by plan id.
pub fn waves(input: &PassInput) -> BTreeMap<&str, i64> {
    let mut phases: BTreeMap<&str, Vec<&PassEntity>> = BTreeMap::new();
    for plan in of_kind(input, "plan").filter(|p| !scope_of(&p.id).is_empty()) {
        phases.entry(scope_of(&plan.id)).or_default().push(plan);
    }
    let mut found = BTreeMap::new();
    for plans in phases.values() {
        let ids: BTreeSet<&str> = plans.iter().map(|p| p.id.as_str()).collect();
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
            .iter()
            .any(|p| p.fields.get("wave0").map(String::as_str) == Some("true"))
        {
            0
        } else {
            1
        };
        let mut level: BTreeMap<&str, i64> = BTreeMap::new();
        while level.len() < ids.len() {
            let ready: Vec<(&str, i64)> = ids
                .iter()
                .filter(|id| !level.contains_key(**id))
                .filter_map(|id| {
                    let deps = depends.get(id).into_iter().flatten();
                    let below: Option<Vec<i64>> = deps.map(|d| level.get(d).copied()).collect();
                    Some((*id, below?.into_iter().max().map_or(0, |m| m + 1)))
                })
                .collect();
            if ready.is_empty() {
                break;
            }
            level.extend(ready);
        }
        if level.len() == ids.len() {
            found.extend(level.into_iter().map(|(id, l)| (id, l + offset)));
        }
    }
    found
}

fn compute(input: &PassInput) -> PassOutput {
    let with_checkpoint: BTreeSet<&str> = of_kind(input, "checkpoint")
        .map(|c| scope_of(&c.id))
        .collect();
    let waves = waves(input);
    let mut values = Vec::new();
    for plan in of_kind(input, "plan") {
        if let Some(wave) = waves.get(plan.id.as_str()) {
            values.push(PassValue {
                entity: plan.id.clone(),
                field: "wave".into(),
                value: (*wave).into(),
            });
        }
        values.push(PassValue {
            entity: plan.id.clone(),
            field: "autonomous".into(),
            value: (!with_checkpoint.contains(plan.id.as_str())).into(),
        });
    }
    PassOutput {
        values,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
