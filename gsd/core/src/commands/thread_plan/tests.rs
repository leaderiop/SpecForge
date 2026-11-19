use serde_json::{Value, json};

use super::*;

fn node(id: &str, kind: &str, fields: Value) -> GraphNode {
    serde_json::from_value(json!({ "id": id, "kind": kind, "fields": fields }))
        .expect("a graph node")
}

fn edge(source: &str, label: &str, target: &str) -> GraphEdge {
    GraphEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

/// Phase `v1.0/01` (plans A and B of strict wave 1, C of strict wave 2; A's tasks T2, T10, T1), phase `v1.0/02` with
/// one plan, and a quick plan with a task.
fn graph() -> CommandGraph {
    let wave = |w: i64| json!({ "wave": w, "strict_wave": w });
    let nodes = vec![
        node("v1.0/01", "phase", json!({})),
        node("v1.0/02", "phase", json!({})),
        node("v1.0/01/C", "plan", wave(2)),
        node("v1.0/01/B", "plan", wave(1)),
        node("v1.0/01/A", "plan", wave(1)),
        node("v1.0/01/A/T2", "task", json!({})),
        node("v1.0/01/A/T10", "task", json!({})),
        node("v1.0/01/A/T1", "task", json!({})),
        node("v1.0/02/D", "plan", wave(1)),
        node("v1.0/02/D/T1", "task", json!({})),
        node("261028-a1b", "plan", json!({ "quick": true })),
        node("261028-a1b/T1", "task", json!({})),
    ];
    let edges = vec![
        edge("v1.0/01/C", "phase", "v1.0/01"),
        edge("v1.0/01/C", "depends_on", "v1.0/01/A"),
        edge("v1.0/01/B", "phase", "v1.0/01"),
        edge("v1.0/01/A", "phase", "v1.0/01"),
        edge("v1.0/01/A/T2", "plan", "v1.0/01/A"),
        edge("v1.0/01/A/T10", "plan", "v1.0/01/A"),
        edge("v1.0/01/A/T1", "plan", "v1.0/01/A"),
        edge("v1.0/02/D", "phase", "v1.0/02"),
        edge("v1.0/02/D/T1", "plan", "v1.0/02/D"),
        edge("261028-a1b/T1", "plan", "261028-a1b"),
    ];
    CommandGraph::new(nodes, edges)
}

/// `(entity_id, action)` of each entry, in answer order.
fn listed(report: &Value) -> Vec<(String, String)> {
    report["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| {
            let text = |k: &str| e[k].as_str().expect("a text").to_string();
            (text("entity_id"), text("action"))
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(id, action)| (id.to_string(), action.to_string()))
        .collect()
}

#[test]
fn plans_run_by_strict_wave_then_id_each_before_its_tasks_in_natural_order() {
    let report = thread(&graph(), "v1.0/01").expect("a thread");
    assert_eq!(report["plan_id"], "v1.0/01");
    assert_eq!(
        listed(&report),
        pairs(&[
            ("v1.0/01/A", "execute"),
            ("v1.0/01/A/T1", "implement"),
            ("v1.0/01/A/T2", "implement"),
            ("v1.0/01/A/T10", "implement"),
            ("v1.0/01/B", "execute"),
            ("v1.0/01/C", "execute"),
        ])
    );
}

#[test]
fn another_phase_and_a_quick_plan_are_not_listed() {
    let report = thread(&graph(), "02").expect("a thread");
    assert_eq!(report["plan_id"], "v1.0/02");
    assert_eq!(
        listed(&report),
        pairs(&[("v1.0/02/D", "execute"), ("v1.0/02/D/T1", "implement")])
    );
}

#[test]
fn a_phase_with_no_plan_answers_no_entry() {
    let graph = CommandGraph::new(vec![node("v1.0/03", "phase", json!({}))], vec![]);
    let report = thread(&graph, "v1.0/03").expect("a thread");
    assert_eq!(report, json!({ "plan_id": "v1.0/03", "entries": [] }));
    assert_eq!(human(&report, &graph), "no plan in v1.0/03\n");
}

#[test]
fn a_plan_without_a_strict_wave_refuses_naming_the_cycle() {
    let nodes = vec![
        node("v1.0/01", "phase", json!({})),
        node("v1.0/01/A", "plan", json!({})),
        node("v1.0/01/B", "plan", json!({})),
    ];
    let edges = vec![
        edge("v1.0/01/A", "phase", "v1.0/01"),
        edge("v1.0/01/B", "phase", "v1.0/01"),
        edge("v1.0/01/A", "depends_on", "v1.0/01/B"),
        edge("v1.0/01/B", "depends_on", "v1.0/01/A"),
    ];
    let refused = thread(&CommandGraph::new(nodes, edges), "v1.0/01").expect_err("a cycle");
    assert_eq!(
        refused,
        "the plans of 'v1.0/01' have no strict wave: their depends_on has a cycle (gsd/E102)"
    );
}

#[test]
fn an_unknown_and_an_ambiguous_phase_are_refused() {
    assert_eq!(
        thread(&graph(), "v1.0/09").expect_err("unknown"),
        "no phase 'v1.0/09'"
    );
    let nodes = vec![
        node("v1.0/01", "phase", json!({})),
        node("v2.0/01", "phase", json!({})),
    ];
    assert_eq!(
        thread(&CommandGraph::new(nodes, vec![]), "01").expect_err("ambiguous"),
        "phase '01' is ambiguous: v1.0/01, v2.0/01"
    );
}

#[test]
fn the_answer_holds_exactly_the_agent_plan_keys() {
    let report = thread(&graph(), "v1.0/01").expect("a thread");
    let keys = |v: &Value| {
        let mut keys: Vec<String> = v.as_object().expect("an object").keys().cloned().collect();
        keys.sort();
        keys
    };
    assert_eq!(keys(&report), ["entries", "plan_id"]);
    for entry in report["entries"].as_array().expect("entries") {
        assert_eq!(keys(entry), ["action", "entity_id"]);
    }
}

#[test]
fn the_text_names_each_plan_its_wave_and_its_tasks() {
    let graph = graph();
    let report = thread(&graph, "v1.0/01").expect("a thread");
    assert_eq!(
        human(&report, &graph),
        "wave 1  v1.0/01/A  (3 tasks)\n\
         wave 1  v1.0/01/B  (0 tasks)\n\
         wave 2  v1.0/01/C  (0 tasks)\n\
         check the order: write `specforge gsd thread-plan v1.0/01 --format json` to a file and run \
         `specforge trace --plan <file>`\n"
    );
}

#[test]
fn unpadded_plan_ids_thread_as_a_person_reads_them() {
    let wave = json!({ "wave": 1, "strict_wave": 1 });
    let mut nodes = vec![node("v1.0/01", "phase", json!({}))];
    let mut edges = Vec::new();
    for plan in ["v1.0/01/01-10", "v1.0/01/01-2", "v1.0/01/01-1"] {
        nodes.push(node(plan, "plan", wave.clone()));
        edges.push(edge(plan, "phase", "v1.0/01"));
    }
    let report = thread(&CommandGraph::new(nodes, edges), "v1.0/01").expect("a thread");
    assert_eq!(
        listed(&report),
        pairs(&[
            ("v1.0/01/01-1", "execute"),
            ("v1.0/01/01-2", "execute"),
            ("v1.0/01/01-10", "execute"),
        ])
    );
}
