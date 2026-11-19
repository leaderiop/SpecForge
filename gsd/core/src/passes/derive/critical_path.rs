//! A phase's `critical_path` and `critical_weight` (ADR 0378 D4): the heaviest chain of its plans along the edges of
//! the strict waves (`depends_on` and the shared paths), named task by task, and its weight.
//!
//! The chain walks the placement the strict wave comes from (`waves::placed`), not the computed field read back, so
//! the two cannot disagree. A plan weighs its `estimate.tokens`, else its number of tasks; the tasks of a plan come in
//! id order as a person reads it (`T2` before `T10`). A phase with no plan, or whose `depends_on` edges hold a cycle,
//! has no placement and gets neither field; a chain that names no task gets its weight only (a list is absent, never
//! empty).

use std::collections::BTreeMap;

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::commands::natural;
use crate::passes::plans::of_kind;
use crate::passes::waves::{self, Placed};
use crate::scope::scope_of;

/// The two fields of every placed phase.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    let mut tasks: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for task in of_kind(input, "task") {
        tasks.entry(scope_of(&task.id)).or_default().push(&task.id);
    }
    for of_plan in tasks.values_mut() {
        of_plan.sort_by_cached_key(|id| natural(id));
    }
    waves::placed(input)
        .iter()
        .flat_map(|placed| of_phase(placed, &tasks))
        .collect()
}

fn of_phase(placed: &Placed<'_>, tasks: &BTreeMap<&str, Vec<&str>>) -> Vec<PassValue> {
    let tasks_of = |plan: &str| tasks.get(plan).map_or(&[][..], Vec::as_slice);
    let weight = |plan: &str| {
        let estimate = placed
            .plans
            .get(plan)
            .and_then(|p| p.values.get("estimate"));
        estimate
            .and_then(|e| e.get("tokens"))
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_else(|| i64::try_from(tasks_of(plan).len()).unwrap_or(i64::MAX))
    };
    let (chain, total) = waves::heaviest(placed, weight);
    let path: Vec<&str> = chain
        .iter()
        .flat_map(|plan| tasks_of(plan).iter().copied())
        .collect();
    let mut values = vec![value(placed.phase, "critical_weight", total)];
    if !path.is_empty() {
        values.push(value(placed.phase, "critical_path", path));
    }
    values
}

#[cfg(test)]
mod tests;
