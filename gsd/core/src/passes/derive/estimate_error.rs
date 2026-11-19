//! A plan's `estimate_error`: how far its `estimate` was from what the plan took, once its summary is complete.
//!
//! The estimate's `tasks` is measured against the plan's task count (counted here, as the summary's `actuals.tasks`
//! is), `(actual - estimate) / estimate`. The member is present only when the estimate writes it above 0, so a plan
//! with no task estimate has no value at all, never a zero. The estimate's `tokens` is not measured: gsd-core pairs it
//! with the summary's `actuals.tokens`, which this model does not hold (`actuals` is computed, and the harness ledger
//! vouches for no member of a block, so no command could record one). A plan is measured by its summary (the first by
//! id, as the plan's status reads it) once that summary's status is `complete`: a halted plan took what it took and
//! measures nothing.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::*;

use super::execute::{status_of, summaries, value};
use crate::passes::plans::of_kind;
use crate::scope::scope_of;

/// The decimal places a member keeps: a ratio such as -1/3 has more digits than a number holds (ADR 0361).
const PLACES: u32 = 4;

/// `(actual - estimate) / estimate`, rounded half away from zero to [`PLACES`] decimal places, exactly as the host
/// holds the number; `None` for an estimate at or below 0 (no division) and for a ratio that still keeps more than
/// 15 significant digits once rounded.
fn error(actual: i64, estimate: i64) -> Option<Decimal> {
    if estimate <= 0 {
        return None;
    }
    let estimate = i128::from(estimate);
    let scaled = (i128::from(actual) - estimate) * 10i128.pow(PLACES);
    let (quotient, rest) = (scaled / estimate, scaled % estimate);
    let rounded = quotient
        + if 2 * rest.abs() >= estimate {
            scaled.signum()
        } else {
            0
        };
    Decimal::parse(&format!("{rounded}e-{PLACES}")).ok()
}

/// The computed field, for each plan whose task estimate is measured against a complete summary.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    let summaries = summaries(input);
    let mut tasks: BTreeMap<&str, i64> = BTreeMap::new();
    for task in of_kind(input, "task") {
        *tasks.entry(scope_of(&task.id)).or_default() += 1;
    }
    of_kind(input, "plan")
        .filter_map(|plan| {
            let estimate = plan.values.get("estimate")?.get("tasks")?.as_i64()?;
            let summary = summaries.get(plan.id.as_str())?;
            if status_of(summary) != "complete" {
                return None;
            }
            let actual = tasks.get(plan.id.as_str()).copied().unwrap_or(0);
            let error = error(actual, estimate)?;
            Some(value(
                &plan.id,
                "estimate_error",
                json!({ "tasks": Value::Number(error.to_json()) }),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests;
