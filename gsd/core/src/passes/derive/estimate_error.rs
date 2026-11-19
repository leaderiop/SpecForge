//! A plan's `estimate_error`: how far its `estimate` was from what the plan took, once its summary is complete.
//!
//! Each member of the estimate is measured against its actual, `(actual - estimate) / estimate`: `tasks` against the
//! plan's task count (counted here, as the summary's `actuals.tasks` is), `tokens` against the summary's
//! `actuals.tokens`. A member is present only when both sides exist and the estimate is above 0, so a missing actual
//! is an absent member, never a zero; a plan with no member measured has no value at all. A plan is measured by its
//! summary (the first by id, as the plan's status reads it) once that summary's status is `complete`: a halted plan
//! took what it took and measures nothing.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use specforge_extension_sdk::prelude::*;

use super::execute::{status_of, summaries, value};
use crate::passes::plans::of_kind;
use crate::scope::scope_of;

/// The estimate's members, in the block's order.
const MEMBERS: [&str; 2] = ["tokens", "tasks"];

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

/// The computed field, for each plan whose estimate measures at least one member against a complete summary.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    let summaries = summaries(input);
    let mut tasks: BTreeMap<&str, i64> = BTreeMap::new();
    for task in of_kind(input, "task") {
        *tasks.entry(scope_of(&task.id)).or_default() += 1;
    }
    of_kind(input, "plan")
        .filter_map(|plan| {
            let estimate = plan.values.get("estimate")?;
            let summary = summaries.get(plan.id.as_str())?;
            if status_of(summary) != "complete" {
                return None;
            }
            // Nothing writes `actuals.tokens` until a harness records the tokens a plan took.
            let tokens = summary
                .values
                .get("actuals")
                .and_then(|actuals| actuals.get("tokens"))
                .and_then(Value::as_i64);
            let tasks = tasks.get(plan.id.as_str()).copied().unwrap_or(0);
            let measured: Map<String, Value> = MEMBERS
                .into_iter()
                .zip([tokens, Some(tasks)])
                .filter_map(|(member, actual)| {
                    let estimate = estimate.get(member)?.as_i64()?;
                    let error = error(actual?, estimate)?;
                    Some((member.to_string(), Value::Number(error.to_json())))
                })
                .collect();
            (!measured.is_empty())
                .then(|| value(&plan.id, "estimate_error", Value::Object(measured)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
