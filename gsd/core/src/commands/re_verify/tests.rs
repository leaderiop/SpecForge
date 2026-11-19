use serde_json::{Value, json};

use super::*;

fn node(id: &str, kind: &str, judgements: Value) -> GraphNode {
    serde_json::from_value(json!({ "id": id, "kind": kind, "judgements": judgements }))
        .expect("a graph node")
}

fn edge(source: &str, label: &str, target: &str) -> GraphEdge {
    GraphEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

const TASKS: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000001";

/// Phase 01 (plans 01-01 and 01-02) and phase 02 (plan 02-01), each item judged in a different state.
fn graph() -> CommandGraph {
    let nodes = vec![
        node("v1.0/01", "phase", json!([])),
        node("v1.0/02", "phase", json!([])),
        node(
            "v1.0/01/01-01",
            "plan",
            json!([{ "rule": "gsd/W311", "state": "stale", "passed": false, "changed": ["plan"] }]),
        ),
        node("v1.0/01/01-02", "plan", json!([])),
        node("v1.0/02/02-01", "plan", json!([])),
        node(
            "v1.0/01/01-01/TR1",
            "truth",
            json!([{ "rule": "gsd/E501", "state": "stale", "passed": false, "outcome": "verified",
                     "at": "2026-11-18T10:00:00Z", "changed": ["cited"],
                     "files": [{ "path": "src/tasks.js", "digest": TASKS }],
                     "cited_tests": ["test/tasks.test.js#adds a task"] }]),
        ),
        node(
            "v1.0/01/01-01/AR1",
            "artifact",
            json!([{ "rule": "gsd/E501", "state": "stale", "passed": false, "outcome": "verified",
                     "changed": ["file"], "files": [{ "path": "src/tasks.js", "digest": TASKS }] }]),
        ),
        node(
            "v1.0/01/01-02/TR1",
            "truth",
            json!([{ "rule": "gsd/E501", "state": "fresh", "passed": false, "outcome": "failed" }]),
        ),
        node(
            "v1.0/01/01-02/PR1",
            "prohibition",
            json!([{ "rule": "gsd/W525", "state": "missing", "passed": false }]),
        ),
        node(
            "v1.0/02/02-01/TR1",
            "truth",
            json!([{ "rule": "gsd/E501", "state": "stale", "passed": false, "changed": ["truth"] }]),
        ),
    ];
    let edges = vec![
        edge("v1.0/01/01-01", "phase", "v1.0/01"),
        edge("v1.0/01/01-02", "phase", "v1.0/01"),
        edge("v1.0/02/02-01", "phase", "v1.0/02"),
        edge("v1.0/01/01-01/TR1", "plan", "v1.0/01/01-01"),
        edge("v1.0/01/01-01/AR1", "plan", "v1.0/01/01-01"),
        edge("v1.0/01/01-02/TR1", "plan", "v1.0/01/01-02"),
        edge("v1.0/01/01-02/PR1", "plan", "v1.0/01/01-02"),
        edge("v1.0/02/02-01/TR1", "plan", "v1.0/02/02-01"),
    ];
    CommandGraph::new(nodes, edges)
}

/// `(entity, rule, state)` of each item, in answer order.
fn listed(report: &Value) -> Vec<(String, String, String)> {
    report["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| {
            let text = |k: &str| i[k].as_str().expect("a text").to_string();
            (text("entity"), text("rule"), text("state"))
        })
        .collect()
}

fn item<'r>(report: &'r Value, entity: &str) -> &'r Value {
    report["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["entity"] == entity)
        .unwrap_or_else(|| panic!("{entity} is listed: {report:#}"))
}

#[test]
fn stale_only_lists_what_no_longer_holds_sorted_by_entity_then_rule() {
    let report = re_verify(&graph(), "v1.0/01", true).expect("an answer");
    assert_eq!(
        listed(&report),
        [
            (
                "v1.0/01/01-01/AR1".into(),
                "gsd/E501".into(),
                "stale".into()
            ),
            (
                "v1.0/01/01-01/TR1".into(),
                "gsd/E501".into(),
                "stale".into()
            ),
            (
                "v1.0/01/01-02/PR1".into(),
                "gsd/W525".into(),
                "missing".into()
            ),
        ]
    );
    assert_eq!(report["subject"], "v1.0/01");
    assert_eq!(
        (report["holding"].clone(), report["not_holding"].clone()),
        (json!(1), json!(3))
    );
}

#[test]
fn each_changed_input_is_named_with_its_selector() {
    let report = re_verify(&graph(), "v1.0/01", true).expect("an answer");
    let truth = item(&report, "v1.0/01/01-01/TR1");
    assert_eq!(
        truth["changed"],
        json!([{ "input": "cited", "selector": "cited" }])
    );
    assert_eq!(
        truth["files"],
        json!([{ "path": "src/tasks.js", "digest": TASKS }])
    );
    assert_eq!(
        truth["cited_tests"],
        json!(["test/tasks.test.js#adds a task"])
    );
    assert_eq!(truth["outcome"], "verified");
    assert_eq!(truth["at"], "2026-11-18T10:00:00Z");
    assert_eq!(truth["kind"], "truth");
    let artifact = item(&report, "v1.0/01/01-01/AR1");
    assert_eq!(
        artifact["changed"],
        json!([{ "input": "file", "selector": "file(self.path)" }])
    );
    assert!(artifact.get("at").is_none(), "{artifact:#}");
}

#[test]
fn a_fresh_verdict_passing_or_failing_is_listed_only_without_stale_only() {
    let all = re_verify(&graph(), "v1.0/01", false).expect("an answer");
    let failed = item(&all, "v1.0/01/01-02/TR1");
    assert_eq!(
        (failed["state"].clone(), failed["outcome"].clone()),
        (json!("fresh"), json!("failed"))
    );
    assert_eq!(all["items"].as_array().map(Vec::len), Some(4));
    let stale = re_verify(&graph(), "v1.0/01", true).expect("an answer");
    assert!(
        listed(&stale)
            .iter()
            .all(|(e, _, _)| e != "v1.0/01/01-02/TR1")
    );
}

#[test]
fn another_phases_items_and_another_judges_rules_are_not_listed() {
    let report = re_verify(&graph(), "v1.0/01", false).expect("an answer");
    let entities: Vec<String> = listed(&report).into_iter().map(|(e, _, _)| e).collect();
    assert!(
        !entities.contains(&"v1.0/02/02-01/TR1".to_string()),
        "{entities:?}"
    );
    // The plan check is gsd-plan-checker's, not the verifier's.
    assert!(
        !entities.contains(&"v1.0/01/01-01".to_string()),
        "{entities:?}"
    );
    let other = re_verify(&graph(), "v1.0/02", true).expect("an answer");
    assert_eq!(
        listed(&other),
        [(
            "v1.0/02/02-01/TR1".into(),
            "gsd/E501".into(),
            "stale".into()
        )]
    );
}

#[test]
fn a_plan_subject_keeps_its_own_plans_items() {
    let report = re_verify(&graph(), "v1.0/01/01-02", false).expect("an answer");
    assert_eq!(
        listed(&report),
        [
            (
                "v1.0/01/01-02/PR1".into(),
                "gsd/W525".into(),
                "missing".into()
            ),
            (
                "v1.0/01/01-02/TR1".into(),
                "gsd/E501".into(),
                "fresh".into()
            ),
        ]
    );
    // The end of a plan's path no other phase or plan shares names it too.
    let short = re_verify(&graph(), "01-02", false).expect("an answer");
    assert_eq!(short["subject"], "v1.0/01/01-02");
}

#[test]
fn an_unknown_subject_is_refused() {
    assert_eq!(
        re_verify(&graph(), "v9.0/07", true).unwrap_err(),
        "no phase or plan 'v9.0/07'"
    );
    // A truth is neither a phase nor a plan.
    assert!(re_verify(&graph(), "v1.0/01/01-01/TR1", true).is_err());
}

#[test]
fn the_text_names_what_changed_and_counts_what_holds() {
    let stale = re_verify(&graph(), "v1.0/01", true).expect("an answer");
    assert_eq!(
        human(&stale),
        "gsd/E501 v1.0/01/01-01/AR1 stale: file (file(self.path))\n\
         gsd/E501 v1.0/01/01-01/TR1 stale: cited (cited)\n\
         gsd/W525 v1.0/01/01-02/PR1 missing\n\
         3 of 4 verdict(s) no longer hold\n"
    );
    let fresh = json!({ "subject": "v1.0/01", "items": [], "holding": 7, "not_holding": 0 });
    assert_eq!(human(&fresh), "every verdict holds (7)\n");
    let none = json!({ "subject": "v1.0/02", "items": [], "holding": 0, "not_holding": 0 });
    assert_eq!(human(&none), "no verdict is owed in v1.0/02\n");
}
