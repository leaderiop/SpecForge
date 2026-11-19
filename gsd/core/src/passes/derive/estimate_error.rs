//! A plan's `estimate_error`: how far its `estimate` was from what the plan took, once its summary is complete.
//!
//! Each member the kind's block lists (`kinds::ESTIMATE_ERROR`) is measured against its actual, `(actual - estimate)
//! / estimate`: `tasks` against the plan's task count, the one count the summary's `actuals.tasks` reads
//! (`plans::tasks_by_plan`). A member is present only when the estimate writes it above 0, so a plan with no task
//! estimate has no value at all, never a zero. The estimate's `tokens` is not measured: gsd-core pairs it
//! with the summary's `actuals.tokens`, which this model does not hold (`actuals` is computed, and the harness ledger
//! vouches for no member of a block, so no command could record one). A plan is measured by its summary (the first by
//! id, as the plan's status reads it) once that summary's status is `complete`: a halted plan took what it took and
//! measures nothing.

use serde_json::{Map, Value};
use specforge_extension_sdk::prelude::*;

use super::execute::{status_of, summaries, value};
use crate::kinds::ESTIMATE_ERROR;
use crate::passes::plans::{of_kind, tasks_by_plan};

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

/// What the member `member` of the estimate is measured against, for a plan of `tasks` tasks; `None` for a member
/// nothing measures (a test keeps every member of the kind's block measured).
fn actual(member: &str, tasks: usize) -> Option<i64> {
    match member {
        "tasks" => i64::try_from(tasks).ok(),
        _ => None,
    }
}

/// The computed field, for each plan whose estimate measures at least one member against a complete summary.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    let summaries = summaries(input);
    let tasks = tasks_by_plan(input);
    of_kind(input, "plan")
        .filter_map(|plan| {
            let estimate = plan.values.get("estimate")?;
            let summary = summaries.get(plan.id.as_str())?;
            if status_of(summary) != "complete" {
                return None;
            }
            let count = tasks.get(plan.id.as_str()).map_or(0, Vec::len);
            let measured: Map<String, Value> = ESTIMATE_ERROR
                .iter()
                .filter_map(|member| {
                    let estimate = estimate.get(member.name)?.as_i64()?;
                    let error = error(actual(member.name, count)?, estimate)?;
                    Some((member.name.to_string(), Value::Number(error.to_json())))
                })
                .collect();
            (!measured.is_empty())
                .then(|| value(&plan.id, "estimate_error", Value::Object(measured)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
