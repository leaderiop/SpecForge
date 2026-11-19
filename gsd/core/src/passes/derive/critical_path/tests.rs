use serde_json::{Value, json};

use super::*;

fn plan(id: &str, tokens: Option<i64>) -> PassEntity {
    let mut plan = PassEntity {
        id: id.into(),
        kind: "plan".into(),
        ..Default::default()
    };
    if let Some(tokens) = tokens {
        plan.values
            .insert("estimate".into(), json!({ "tokens": tokens }));
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

/// The entities, with `(plan, depends on)` pairs as `depends_on` edges.
fn graph(entities: Vec<PassEntity>, depends: &[(&str, &str)]) -> PassInput {
    PassInput {
        entities,
        edges: depends
            .iter()
            .map(|(source, target)| PassEdge {
                source: (*source).into(),
                target: (*target).into(),
                label: "depends_on".into(),
            })
            .collect(),
        ..Default::default()
    }
}

/// `(entity, field, value)` of every value, in the order they come.
fn found(input: &PassInput) -> Vec<(String, String, Value)> {
    values(input)
        .into_iter()
        .map(|v| (v.entity, v.field, v.value))
        .collect()
}

fn fields(phase: &str, path: &[&str], weight: i64) -> Vec<(String, String, Value)> {
    vec![
        (phase.into(), "critical_weight".into(), json!(weight)),
        (phase.into(), "critical_path".into(), json!(path)),
    ]
}

/// Phase `v/1`: plan `a` with tasks T1 and T2, plan `b` after it with T1, `b` estimating `tokens` when given.
fn two_plans(tokens: Option<i64>) -> PassInput {
    graph(
        vec![
            plan("v/1/a", None),
            plan("v/1/b", tokens),
            task("v/1/a/T1"),
            task("v/1/a/T2"),
            task("v/1/b/T1"),
        ],
        &[("v/1/b", "v/1/a")],
    )
}

#[test]
fn a_plan_without_an_estimate_weighs_its_task_count() {
    assert_eq!(
        found(&two_plans(None)),
        fields("v/1", &["v/1/a/T1", "v/1/a/T2", "v/1/b/T1"], 3)
    );
}

#[test]
fn an_estimate_weighs_a_plan_by_its_tokens() {
    assert_eq!(
        found(&two_plans(Some(9))),
        fields("v/1", &["v/1/a/T1", "v/1/a/T2", "v/1/b/T1"], 11)
    );
}

#[test]
fn the_tasks_of_a_plan_come_in_natural_order_with_their_full_ids() {
    let input = graph(
        vec![
            plan("v/1/a", None),
            task("v/1/a/T10"),
            task("v/1/a/T2"),
            task("v/1/a/T1"),
        ],
        &[],
    );
    assert_eq!(
        found(&input),
        fields("v/1", &["v/1/a/T1", "v/1/a/T2", "v/1/a/T10"], 3)
    );
}

#[test]
fn each_phase_has_its_own_chain() {
    let input = graph(
        vec![
            plan("v/1/a", None),
            plan("v/2/a", None),
            task("v/1/a/T1"),
            task("v/2/a/T1"),
            task("v/2/a/T2"),
        ],
        &[],
    );
    let mut both = fields("v/1", &["v/1/a/T1"], 1);
    both.extend(fields("v/2", &["v/2/a/T1", "v/2/a/T2"], 2));
    assert_eq!(found(&input), both);
}

#[test]
fn no_plan_a_cycle_and_a_quick_plan_answer_nothing() {
    assert_eq!(found(&graph(vec![task("v/1/a/T1")], &[])), []);
    let cycle = graph(
        vec![plan("v/1/a", None), plan("v/1/b", None), task("v/1/a/T1")],
        &[("v/1/a", "v/1/b"), ("v/1/b", "v/1/a")],
    );
    assert_eq!(found(&cycle), []);
    let quick = graph(vec![plan("261028-a1b", None), task("261028-a1b/T1")], &[]);
    assert_eq!(found(&quick), []);
}

#[test]
fn a_chain_that_names_no_task_has_a_weight_and_no_path() {
    let none = graph(vec![plan("v/1/a", None)], &[]);
    assert_eq!(
        found(&none),
        [("v/1".into(), "critical_weight".into(), json!(0))]
    );
    let estimated = graph(vec![plan("v/1/a", Some(9))], &[]);
    assert_eq!(
        found(&estimated),
        [("v/1".into(), "critical_weight".into(), json!(9))]
    );
}

#[test]
fn a_negative_estimate_weighs_0() {
    assert_eq!(
        found(&two_plans(Some(-4))),
        fields("v/1", &["v/1/a/T1", "v/1/a/T2", "v/1/b/T1"], 2)
    );
}
