use serde_json::{Value, json};

use super::*;

fn plan(id: &str, estimate: Option<Value>) -> PassEntity {
    let mut plan = PassEntity {
        id: id.into(),
        kind: "plan".into(),
        ..Default::default()
    };
    if let Some(estimate) = estimate {
        plan.values.insert("estimate".into(), estimate);
    }
    plan
}

fn task(id: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "task".into(),
        ..Default::default()
    }
}

/// A summary of plan `plan` with `status`, and `actuals.tokens` when given.
fn summary(plan: &str, status: &str, tokens: Option<i64>) -> PassEntity {
    let mut summary = PassEntity {
        id: format!("{plan}/SUMMARY"),
        kind: "summary".into(),
        ..Default::default()
    };
    summary.fields.insert("status".into(), status.into());
    if let Some(tokens) = tokens {
        summary
            .values
            .insert("actuals".into(), json!({ "tokens": tokens }));
    }
    summary
}

/// Plan `v1/01/01` with `estimate`, two tasks and, when given, a summary of `status` with `actuals.tokens`.
fn measured(estimate: Option<Value>, summary_of: Option<(&str, Option<i64>)>) -> PassInput {
    let mut entities = vec![
        plan("v1/01/01", estimate),
        task("v1/01/01/T1"),
        task("v1/01/01/T2"),
    ];
    if let Some((status, tokens)) = summary_of {
        entities.push(summary("v1/01/01", status, tokens));
    }
    PassInput {
        entities,
        ..Default::default()
    }
}

/// The one `estimate_error` the input answers, `None` when it answers none.
fn error_of(input: &PassInput) -> Option<Value> {
    let values = values(input);
    assert!(values.len() <= 1, "{values:?}");
    values.into_iter().next().map(|v| {
        assert_eq!(
            (v.entity.as_str(), v.field.as_str()),
            ("v1/01/01", "estimate_error")
        );
        v.value
    })
}

#[test]
fn both_members_are_measured_against_their_actuals() {
    let input = measured(
        Some(json!({ "tokens": 1000, "tasks": 3 })),
        Some(("complete", Some(1500))),
    );
    assert_eq!(
        error_of(&input),
        Some(json!({ "tokens": 0.5, "tasks": -0.3333 }))
    );
}

#[test]
fn a_missing_actual_leaves_its_member_out() {
    let input = measured(
        Some(json!({ "tokens": 1000, "tasks": 3 })),
        Some(("complete", None)),
    );
    assert_eq!(error_of(&input), Some(json!({ "tasks": -0.3333 })));
}

#[test]
fn an_estimate_that_held_is_a_measured_zero() {
    let input = measured(Some(json!({ "tasks": 2 })), Some(("complete", None)));
    let error = error_of(&input).expect("a value");
    assert_eq!(error, json!({ "tasks": 0 }));
    assert!(error["tasks"].is_i64(), "zero is held as the host holds it");
}

#[test]
fn an_estimate_at_or_below_zero_measures_nothing() {
    let input = measured(
        Some(json!({ "tokens": 0, "tasks": 3 })),
        Some(("complete", Some(1500))),
    );
    assert_eq!(error_of(&input), Some(json!({ "tasks": -0.3333 })));
    let input = measured(
        Some(json!({ "tokens": 1000, "tasks": 0 })),
        Some(("complete", Some(1500))),
    );
    assert_eq!(error_of(&input), Some(json!({ "tokens": 0.5 })));
    let input = measured(
        Some(json!({ "tokens": -5, "tasks": -1 })),
        Some(("complete", Some(1500))),
    );
    assert_eq!(error_of(&input), None, "neither member: no value at all");
}

#[test]
fn without_an_estimate_or_a_complete_summary_nothing_is_measured() {
    assert_eq!(
        error_of(&measured(None, Some(("complete", Some(1500))))),
        None
    );
    let estimate = || Some(json!({ "tokens": 1000, "tasks": 3 }));
    assert_eq!(error_of(&measured(estimate(), None)), None, "no summary");
    assert_eq!(
        error_of(&measured(estimate(), Some(("halted", Some(1500))))),
        None,
        "a halted summary"
    );
    assert_eq!(
        error_of(&measured(estimate(), Some(("Complete", None)))),
        Some(json!({ "tasks": -0.3333 })),
        "the status is case-folded"
    );
}

#[test]
fn a_ratio_a_number_cannot_hold_is_left_out() {
    // 9007199254740992 / 1 keeps 16 significant digits after rounding: a number holds 15.
    let input = measured(
        Some(json!({ "tokens": 1, "tasks": 2 })),
        Some(("complete", Some(9_007_199_254_740_993))),
    );
    assert_eq!(error_of(&input), Some(json!({ "tasks": 0 })));
}

#[test]
fn the_error_rounds_half_away_from_zero_to_four_places() {
    let cases = [
        ((1, 3), "-0.6667"),
        ((2, 3), "-0.3333"),
        ((4, 3), "0.3333"),
        ((5, 3), "0.6667"),
        ((1, 8), "-0.875"),
        ((3, 20_000), "-0.9999"),
        ((1, 20_000), "-1"),
        ((20_001, 20_000), "0.0001"),
        ((40_001, 80_000), "-0.5"),
        ((0, 7), "-1"),
        ((7, 7), "0"),
        ((i64::MAX, 1_000_000_000_000), "9223371.0369"),
    ];
    for ((actual, estimate), want) in cases {
        let got = error(actual, estimate).map(|d| d.to_string());
        assert_eq!(got.as_deref(), Some(want), "{actual} against {estimate}");
    }
    assert_eq!(error(1, 0), None);
    assert_eq!(error(1, -3), None);
    assert_eq!(error(i64::MAX, 1), None, "too many digits");
}
