//! `specforge gsd thread-plan`: a phase's plans and tasks as the agent plan `specforge trace --plan` checks, on the
//! `plan` project (phase v1.0/01: 01-01 with T1 and T2, then 01-02, which depends on it, with T1).

use serde_json::{Value, json};

use super::harness::{Project, Run, project};
use super::plan_rules::{third_plan, with};

/// The JSON answer of `gsd thread-plan <phase>`.
pub(super) fn thread(p: &Project, phase: &str) -> Value {
    let run = p.run(&["gsd", "thread-plan", phase, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("thread-plan prints JSON ({e}): {}", run.all()))
}

/// `specforge trace --plan` over `plan`, written to a file under the project: its run and its JSON outcome.
pub(super) fn trace(p: &Project, plan: &Value) -> (Run, Value) {
    p.write("agent-plan.json", &plan.to_string());
    let file = p.root().join("agent-plan.json");
    let run = p.run(&[
        "trace",
        "--plan",
        file.to_str().expect("a UTF-8 path"),
        "--format",
        "json",
    ]);
    let outcome = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("trace --plan prints JSON ({e}): {}", run.all()));
    (run, outcome)
}

/// The gaps of `outcome` of the kind `kind`.
fn gaps<'a>(outcome: &'a Value, kind: &str) -> Vec<&'a Value> {
    outcome["gaps"]
        .as_array()
        .expect("gaps")
        .iter()
        .filter(|g| g["missing_link_type"] == kind)
        .collect()
}

fn ids(plan: &Value) -> Vec<&str> {
    plan["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["entity_id"].as_str().expect("an id"))
        .collect()
}

#[test]
fn the_thread_of_a_phase_passes_trace() {
    let p = project("plan");
    let plan = thread(&p, "v1.0/01");
    assert_eq!(
        plan,
        json!({
            "plan_id": "v1.0/01",
            "entries": [
                { "entity_id": "v1.0/01/01-01", "action": "execute" },
                { "entity_id": "v1.0/01/01-01/T1", "action": "implement" },
                { "entity_id": "v1.0/01/01-01/T2", "action": "implement" },
                { "entity_id": "v1.0/01/01-02", "action": "execute" },
                { "entity_id": "v1.0/01/01-02/T1", "action": "implement" },
            ]
        })
    );
    let (run, outcome) = trace(&p, &plan);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(outcome["ok"], true, "{outcome:#}");
    assert_eq!(gaps(&outcome, "ordering"), Vec::<&Value>::new());
    assert_eq!(gaps(&outcome, "unresolved_entity"), Vec::<&Value>::new());

    let text = p.run(&["gsd", "thread-plan", "01"]);
    assert_eq!(text.code, 0, "{}", text.all());
    assert!(
        text.stdout
            .starts_with("wave 1  v1.0/01/01-01  (2 tasks)\nwave 2  v1.0/01/01-02  (1 task)\n"),
        "{}",
        text.all()
    );
}

#[test]
fn a_plan_listed_before_its_dependency_is_flagged() {
    let p = project("plan");
    let mut plan = thread(&p, "v1.0/01");
    let entries = plan["entries"].as_array_mut().expect("entries");
    entries.rotate_left(3);
    assert_eq!(
        ids(&plan),
        [
            "v1.0/01/01-02",
            "v1.0/01/01-02/T1",
            "v1.0/01/01-01",
            "v1.0/01/01-01/T1",
            "v1.0/01/01-01/T2"
        ]
    );
    let (run, outcome) = trace(&p, &plan);
    assert_eq!(run.code, 1, "{}", run.all());
    assert_eq!(outcome["ok"], false);
    let ordering = gaps(&outcome, "ordering");
    assert!(
        ordering
            .iter()
            .any(|g| g["source_entity"] == "v1.0/01/01-02"
                && g["target_entity"] == "v1.0/01/01-01"
                && g["gap_context"]
                    .as_str()
                    .is_some_and(|c| c.contains("via depends_on"))),
        "{outcome:#}"
    );
}

#[test]
fn a_coupling_peer_that_runs_later_is_an_ordering_gap() {
    // 01-03 has no depends_on, so it runs in wave 1, before 01-02; its exemption names 01-02 as its peer.
    let third = third_plan("", "\"src/third.js\"", "").replace(
        "  objective      \"A third plan\"\n",
        "  objective      \"A third plan\"\n  coupling_justified [{ peer 01-02 reason \"both read the store\" }]\n",
    );
    let p = with(&third);
    let plan = thread(&p, "v1.0/01");
    assert_eq!(
        ids(&plan),
        [
            "v1.0/01/01-01",
            "v1.0/01/01-01/T1",
            "v1.0/01/01-01/T2",
            "v1.0/01/01-03",
            "v1.0/01/01-02",
            "v1.0/01/01-02/T1"
        ]
    );
    let (run, outcome) = trace(&p, &plan);
    assert_eq!(run.code, 1, "{}", run.all());
    let ordering = gaps(&outcome, "ordering");
    assert!(
        ordering
            .iter()
            .any(|g| g["source_entity"] == "v1.0/01/01-03"
                && g["target_entity"] == "v1.0/01/01-02"
                && g["gap_context"]
                    .as_str()
                    .is_some_and(|c| c.contains("via coupling_justified.peer"))),
        "{outcome:#}"
    );
}

#[test]
fn an_entry_naming_no_entity_is_e003() {
    let p = project("plan");
    let mut plan = thread(&p, "v1.0/01");
    plan["entries"]
        .as_array_mut()
        .expect("entries")
        .push(json!({ "entity_id": "v1.0/01/01-09", "action": "execute" }));
    let (run, outcome) = trace(&p, &plan);
    assert_eq!(run.code, 1, "{}", run.all());
    let unresolved = gaps(&outcome, "unresolved_entity");
    assert_eq!(unresolved.len(), 1, "{outcome:#}");
    // The plan itself is at fault, so the gap's source is `plan` and its target the entry.
    assert_eq!(unresolved[0]["source_entity"], "plan");
    assert_eq!(unresolved[0]["target_entity"], "v1.0/01/01-09");
    assert!(
        unresolved[0]["gap_context"]
            .as_str()
            .is_some_and(|c| c.starts_with("E003")),
        "{outcome:#}"
    );
}

#[test]
fn an_unknown_phase_is_refused() {
    let p = project("plan");
    let run = p.run(&["gsd", "thread-plan", "v1.0/09"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("no phase 'v1.0/09'"), "{}", run.all());
}

#[test]
fn a_cycle_is_refused_naming_e102() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  requirements   [TASK-01]\n",
        "  depends_on     [01-02]\n  requirements   [TASK-01]\n",
    );
    assert!(
        p.codes().contains(&"gsd/E102".to_string()),
        "{:?}",
        p.codes()
    );
    let run = p.run(&["gsd", "thread-plan", "v1.0/01"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains(
            "the plans of 'v1.0/01' have no strict wave: their depends_on has a cycle (gsd/E102)"
        ),
        "{}",
        run.all()
    );
}
