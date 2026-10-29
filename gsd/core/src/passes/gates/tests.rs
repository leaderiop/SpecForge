use serde_json::{Value, json};

use super::*;

fn input(trigger: Value, entities: Vec<Value>, edges: Vec<Value>) -> PassInput {
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges, "trigger": trigger }),
    ))
    .expect("a pass input")
}

fn ship(subject: Option<&str>) -> Value {
    json!({ "kind": "gate", "command": "gsd-ship", "subject": subject })
}

fn codes(found: Vec<PassDiagnostic>) -> Vec<String> {
    found.into_iter().map(|d| d.code).collect()
}

fn window(id: &str, extra: Value) -> Value {
    json!({ "id": id, "kind": "broken_window", "fields": extra })
}

fn phase_edge() -> Vec<Value> {
    vec![json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" })]
}

#[test]
fn ship_with_an_open_window_is_refused_e512() {
    let found = input(ship(None), vec![window("BW1", json!({}))], vec![]);
    assert_eq!(codes(e512(&found)), ["E512"]);
}

#[test]
fn e512_names_the_subject_of_a_scoped_gate() {
    // A window is project-scoped, so the host would drop a diagnostic that names only it from a gate on a phase.
    let found = input(
        ship(Some("v1.0/01")),
        vec![window("BW1", json!({}))],
        vec![],
    );
    let refused = e512(&found);
    assert_eq!(codes(refused.clone()), ["E512"]);
    assert_eq!(refused[0].entity.as_deref(), Some("v1.0/01"));
    let project = input(ship(None), vec![window("BW1", json!({}))], vec![]);
    assert_eq!(e512(&project)[0].entity.as_deref(), Some("BW1"));
}

#[test]
fn a_gate_is_told_by_its_kind_not_by_a_command() {
    let no_command = json!({ "kind": "gate", "subject": null });
    let found = input(no_command, vec![window("BW1", json!({}))], vec![]);
    assert_eq!(codes(e512(&found)), ["E512"]);
    let write_with_a_command = json!({ "kind": "write", "command": "gsd-ship" });
    let found = input(write_with_a_command, vec![window("BW1", json!({}))], vec![]);
    assert!(e512(&found).is_empty());
}

#[test]
fn e512_is_silent_on_a_resolved_window_and_without_a_gate() {
    let waived = window("BW1", json!({ "waiver": "x" }));
    let fixed = window("BW2", json!({ "fixed_at": "2026-10-29T10:00:00Z" }));
    let resolved = input(ship(None), vec![waived, fixed], vec![]);
    assert_eq!(codes(e512(&resolved)), Vec::<String>::new());
    // A write has no trigger command: the open window refuses nothing.
    let write = input(
        json!({ "kind": "write" }),
        vec![window("BW1", json!({}))],
        vec![],
    );
    assert!(e512(&write).is_empty());
    let untriggered: PassInput =
        serde_json::from_value(json!({ "entities": [window("BW1", json!({}))] })).expect("input");
    assert!(e512(&untriggered).is_empty());
}

fn audited(status: &str, accepted: bool) -> PassInput {
    let mut fields = json!({ "milestone": "v1.0", "status": status });
    if accepted {
        fields["debt_accepted"] = json!("x");
    }
    input(
        ship(Some("v1.0")),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            json!({ "id": "v1.0/A1", "kind": "milestone_audit", "fields": fields }),
        ],
        vec![],
    )
}

#[test]
fn an_unaccepted_tech_debt_audit_is_e106() {
    assert_eq!(codes(e106(&audited("tech_debt", false))), ["E106"]);
    assert_eq!(codes(e106(&audited("gaps_found", true))), ["E106"]);
    assert!(e106(&audited("tech_debt", true)).is_empty());
    assert!(e106(&audited("passed", false)).is_empty());
    let none = input(
        ship(Some("v1.0")),
        vec![json!({ "id": "v1.0", "kind": "milestone" })],
        vec![],
    );
    assert_eq!(codes(e106(&none)), ["E106"]);
}

#[test]
fn e106_reads_the_latest_audit_by_number_and_only_at_a_milestone() {
    let audit = |n: u32, status: &str| {
        json!({ "id": format!("v1.0/A{n}"), "kind": "milestone_audit",
                "fields": { "milestone": "v1.0", "status": status } })
    };
    let found = input(
        ship(Some("v1.0")),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            audit(9, "gaps_found"),
            audit(10, "passed"),
        ],
        vec![],
    );
    assert!(e106(&found).is_empty());
    // A phase is not audited: a gate on one asks nothing.
    let phase = input(
        ship(Some("v1.0/01")),
        vec![json!({ "id": "v1.0/01", "kind": "phase" })],
        vec![],
    );
    assert!(e106(&phase).is_empty());
}

fn phase_gate(verification: Option<Value>, truth_outcome: Option<&str>) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0/01", "kind": "phase" }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
    ];
    entities.extend(verification);
    entities.extend(truth_outcome.map(|outcome| {
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth", "judgements": [
            { "rule": "gsd/E501", "state": "fresh", "passed": true, "outcome": outcome } ] })
    }));
    input(ship(Some("v1.0/01")), entities, phase_edge())
}

#[test]
fn an_unverified_phase_is_e511_at_ship() {
    assert_eq!(codes(e511(&phase_gate(None, None))), ["E511"]);
    let verification = json!({ "id": "v1.0/01/01-VERIFICATION", "kind": "verification",
                               "fields": { "phase": "v1.0/01" } });
    assert_eq!(
        codes(e511(&phase_gate(Some(verification.clone()), None))),
        ["E511"]
    );
    assert!(e511(&phase_gate(Some(verification), Some("verified"))).is_empty());
}

#[test]
fn a_stale_plan_check_is_w318() {
    let plan = |state: &str| {
        input(
            ship(Some("v1.0/01")),
            vec![
                json!({ "id": "v1.0/01", "kind": "phase" }),
                json!({ "id": "v1.0/01/01-01", "kind": "plan",
                        "judgements": [{ "rule": "gsd/W313", "state": state, "passed": false }] }),
            ],
            phase_edge(),
        )
    };
    assert_eq!(codes(w318(&plan("stale"))), ["W318"]);
    assert!(w318(&plan("fresh")).is_empty());
}

#[test]
fn a_tdd_plan_without_review_is_w404() {
    let plan = |checkpoint: Option<Value>| {
        let mut entities = vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "type": "tdd" } }),
            json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" }),
        ];
        entities.extend(checkpoint);
        input(ship(Some("v1.0/01")), entities, phase_edge())
    };
    let review = |answered: bool| {
        let mut fields = json!({ "kind": "tdd_review" });
        if answered {
            fields["answer"] = json!("x");
        }
        json!({ "id": "v1.0/01/01-01/C1", "kind": "checkpoint", "fields": fields })
    };
    assert_eq!(codes(w404(&plan(None))), ["W404"]);
    assert_eq!(codes(w404(&plan(Some(review(false))))), ["W404"]);
    assert!(w404(&plan(Some(review(true)))).is_empty());
}

fn two_plans(subject: &str, state: &str) -> PassInput {
    let plan = |id: &str, state: &str| {
        json!({ "id": id, "kind": "plan", "fields": { "type": "tdd" },
                "judgements": [{ "rule": "gsd/W313", "state": state, "passed": false }] })
    };
    input(
        ship(Some(subject)),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            plan("v1.0/01/01-01", state),
            plan("v1.0/01/01-02", "stale"),
            json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" }),
        ],
        vec![
            json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }),
            json!({ "source": "v1.0/01/01-02", "target": "v1.0/01", "label": "phase" }),
        ],
    )
}

#[test]
fn a_plan_subject_scopes_its_plan() {
    // Only plan 01-01 is stale here; 01-02 would be, were the gate on the phase.
    let found = w318(&two_plans("v1.0/01/01-01", "stale"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity.as_deref(), Some("v1.0/01/01-01"));
    assert!(w318(&two_plans("v1.0/01/01-01", "fresh")).is_empty());
    assert_eq!(w318(&two_plans("v1.0/01", "fresh")).len(), 1);
}

#[test]
fn a_summary_subject_scopes_its_plan_and_w404_names_the_summary() {
    let summary = "v1.0/01/01-01/01-01-SUMMARY";
    let found = w404(&two_plans(summary, "fresh"));
    assert_eq!(found.len(), 1, "only 01-01 has a summary");
    assert_eq!(found[0].entity.as_deref(), Some(summary));
    // At a phase the diagnostic names the plan.
    assert_eq!(
        w404(&two_plans("v1.0/01", "fresh"))[0].entity.as_deref(),
        Some("v1.0/01/01-01")
    );
}

#[test]
fn a_stale_verdict_is_e516_under_plus() {
    let stale = |state: &str| {
        input(
            ship(None),
            vec![json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                         "judgements": [{ "rule": "gsd/E501", "state": state, "passed": false }] })],
            vec![],
        )
    };
    assert_eq!(codes(e516(&stale("stale"))), ["E516"]);
    assert!(e516(&stale("fresh")).is_empty());
}

/// A project every guard has something to say about at a gate on milestone v1.0: no audit (E106), an unverified
/// phase (E511), an open window (E512), a stale plan-check (W318), a tdd plan with a summary and no review (W404) and
/// a stale verdict (E516).
fn everything_wrong() -> (Vec<Value>, Vec<Value>) {
    let entities = vec![
        json!({ "id": "v1.0", "kind": "milestone" }),
        json!({ "id": "v1.0/01", "kind": "phase" }),
        window("BW1", json!({})),
        json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "type": "tdd" },
                "judgements": [{ "rule": "gsd/W313", "state": "stale", "passed": false }] }),
        json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" }),
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                "judgements": [{ "rule": "gsd/E501", "state": "stale", "passed": false }] }),
    ];
    let mut edges = phase_edge();
    edges.push(json!({ "source": "v1.0/01", "target": "v1.0", "label": "milestone" }));
    (entities, edges)
}

#[test]
fn every_guard_answers_a_gate_and_is_silent_on_a_write() {
    let (entities, edges) = everything_wrong();
    let gate = input(ship(Some("v1.0")), entities.clone(), edges.clone());
    // The same project and subject, but the host is applying an edit plan: not a gate.
    let write = input(
        json!({ "kind": "write", "subject": "v1.0" }),
        entities.clone(),
        edges.clone(),
    );
    let untriggered: PassInput = serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges }),
    ))
    .expect("input");
    for (name, code, check) in GUARDS {
        let answered = codes(check(&gate));
        assert!(
            !answered.is_empty() && answered.iter().all(|c| c == code),
            "{name} ({code}) is silent at a gate over a project it should refuse: {answered:?}"
        );
        for found in [&write, &untriggered] {
            assert!(check(found).is_empty(), "{name} ({code}) answered a write");
        }
    }
}
