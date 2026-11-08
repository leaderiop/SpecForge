//! A plan's `estimate_slices`: how many plans its `estimate.tokens` makes under `workflow.smart_zone_tokens`, so the
//! plan checker reads a figure instead of dividing. Absent without an estimate or a budget.

use specforge_extension_sdk::prelude::*;

use super::execute::value;
use crate::passes::plans::of_kind;

/// The estimate over the budget, rounded up; at least one plan.
fn slices(tokens: i64, budget: u64) -> u64 {
    u64::try_from(tokens).unwrap_or(0).div_ceil(budget).max(1)
}

fn sliced(input: &PassInput, budget: u64) -> Vec<PassValue> {
    of_kind(input, "plan")
        .filter_map(|plan| {
            let tokens = plan.values.get("estimate")?.get("tokens")?.as_i64()?;
            Some(value(&plan.id, "estimate_slices", slices(tokens, budget)))
        })
        .collect()
}

/// The computed field, for each plan carrying an estimate.
pub fn values(input: &PassInput) -> Vec<PassValue> {
    Settings::current()
        .integer("workflow.smart_zone_tokens")
        .and_then(|budget| u64::try_from(budget).ok())
        .filter(|budget| *budget > 0)
        .map(|budget| sliced(input, budget))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(id: &str, tokens: Option<i64>) -> PassEntity {
        let mut plan = PassEntity {
            id: id.into(),
            kind: "plan".into(),
            ..Default::default()
        };
        if let Some(tokens) = tokens {
            plan.values
                .insert("estimate".into(), serde_json::json!({ "tokens": tokens }));
        }
        plan
    }

    #[test]
    fn a_plan_with_an_estimate_answers_its_slices_rounded_up() {
        let input = PassInput {
            entities: vec![
                plan("v1/01/01", Some(80_000)),
                plan("v1/01/02", Some(100_000)),
                plan("v1/01/03", Some(250_001)),
                plan("v1/01/04", Some(0)),
                plan("v1/01/05", None),
            ],
            ..Default::default()
        };
        let got: Vec<(String, u64)> = sliced(&input, 100_000)
            .into_iter()
            .inspect(|v| assert_eq!(v.field, "estimate_slices"))
            .map(|v| (v.entity, v.value.as_u64().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                ("v1/01/01".into(), 1),
                ("v1/01/02".into(), 1),
                ("v1/01/03".into(), 3),
                ("v1/01/04".into(), 1),
            ],
            "no value for the plan without an estimate"
        );
    }

    #[test]
    fn no_budget_answers_nothing() {
        let input = PassInput {
            entities: vec![plan("v1/01/01", Some(80_000))],
            ..Default::default()
        };
        assert!(
            values(&input).is_empty(),
            "outside a call no setting is set"
        );
    }
}
