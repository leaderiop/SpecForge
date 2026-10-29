use serde_json::{Value, json};

use super::*;

fn input(trigger: Value, entities: Vec<Value>, edges: Vec<Value>) -> PassInput {
    serde_json::from_value(json!({ "entities": entities, "edges": edges, "trigger": trigger }))
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
        vec![json!({ "id": "v1.0/A1", "kind": "milestone_audit", "fields": fields })],
        vec![],
    )
}

#[test]
fn an_unaccepted_tech_debt_audit_is_e106() {
    assert_eq!(codes(e106(&audited("tech_debt", false))), ["E106"]);
    assert_eq!(codes(e106(&audited("gaps_found", true))), ["E106"]);
    assert!(e106(&audited("tech_debt", true)).is_empty());
    assert!(e106(&audited("passed", false)).is_empty());
    let none = input(ship(Some("v1.0")), vec![], vec![]);
    assert_eq!(codes(e106(&none)), ["E106"]);
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

#[test]
fn every_guard_is_silent_on_a_write() {
    let entities = vec![
        window("BW1", json!({})),
        json!({ "id": "v1.0/01", "kind": "phase" }),
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                "judgements": [{ "rule": "gsd/E501", "state": "stale", "passed": false }] }),
    ];
    let untriggered: PassInput =
        serde_json::from_value(json!({ "entities": entities.clone() })).expect("input");
    let write = input(json!({ "kind": "write" }), entities.clone(), vec![]);
    for found in [&untriggered, &write] {
        for (name, code, check) in GUARDS {
            assert!(check(found).is_empty(), "{name} ({code}) answered a write");
        }
    }
    // The same project under a gate: E512, E516 and E511 answer.
    let gated = input(ship(None), entities, vec![]);
    assert_eq!(codes(e512(&gated)), ["E512"]);
    assert_eq!(codes(e516(&gated)), ["E516"]);
    assert_eq!(codes(e511(&gated)), ["E511"]);
}
