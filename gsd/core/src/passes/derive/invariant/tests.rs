use serde_json::json;

use super::*;

const AT: &str = "2026-11-14T10:00:00Z";

/// A judgement of the invariant's rule as the host reports it; an outcome comes with its time.
fn judged(state: &str, outcome: Option<&str>) -> serde_json::Value {
    json!({ "rule": RULE, "state": state,
            "passed": state == "fresh" && outcome == Some("holds"),
            "outcome": outcome, "at": outcome.map(|_| AT) })
}

/// Phases 01 (`executed`) and 02 (`planned`), and `invariants` as `(id, holds_from, judgements)`; a `holds_from`
/// that names a phase resolves to it, as the host's edges would.
fn project(invariants: &[(&str, Option<&str>, serde_json::Value)]) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0/01", "kind": "phase" }),
        json!({ "id": "v1.0/02", "kind": "phase" }),
    ];
    let mut edges = Vec::new();
    for (id, holds_from, judgements) in invariants {
        let mut fields = json!({ "statement": "it holds" });
        if let Some(phase) = holds_from {
            fields["holds_from"] = json!(phase);
            if phase.starts_with("v1.0/") {
                edges.push(json!({ "source": id, "target": phase, "label": "holds_from" }));
            }
        }
        entities.push(json!({ "id": id, "kind": "invariant", "fields": fields,
                              "judgements": judgements }));
    }
    serde_json::from_value(json!({ "entities": entities, "edges": edges })).expect("a pass input")
}

fn phases() -> BTreeMap<String, &'static str> {
    BTreeMap::from([
        ("v1.0/01".to_string(), "executed"),
        ("v1.0/02".to_string(), "planned"),
    ])
}

fn found(values: &[PassValue], entity: &str, field: &str) -> Option<serde_json::Value> {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| v.value.clone())
}

#[test]
fn the_status_follows_the_verdict_while_in_force() {
    let fresh = |outcome| Some(("fresh", Some(outcome), Some(AT)));
    assert_eq!(status(false, fresh("holds")), "pending", "not in force");
    assert_eq!(status(false, None), "pending");
    assert_eq!(status(true, None), "unverified");
    for state in ["missing", "unrecorded", "unreadable"] {
        assert_eq!(
            status(true, Some((state, None, None))),
            "unverified",
            "{state}"
        );
    }
    assert_eq!(
        status(true, Some(("stale", Some("holds"), Some(AT)))),
        "stale"
    );
    for outcome in OUTCOMES {
        assert_eq!(status(true, fresh(outcome)), *outcome);
    }
    assert_eq!(
        status(true, fresh("other")),
        "unverified",
        "an outcome the rule does not declare is no verdict"
    );
}

#[test]
fn an_invariant_is_in_force_from_the_start_or_once_its_phase_is_complete() {
    let fresh = json!([judged("fresh", Some("holds"))]);
    let input = project(&[
        ("always", None, fresh.clone()),
        ("from_executed", Some("v1.0/01"), fresh.clone()),
        ("from_planned", Some("v1.0/02"), fresh.clone()),
        ("from_nowhere", Some("nowhere"), fresh),
    ]);
    let statuses = statuses(&input, &phases());
    assert_eq!(statuses["always"], "holds");
    assert_eq!(statuses["from_executed"], "holds");
    assert_eq!(statuses["from_planned"], "pending");
    assert_eq!(
        statuses["from_nowhere"], "pending",
        "a holds_from that names no phase"
    );
    for complete in PHASE_COMPLETE {
        let phases = BTreeMap::from([("v1.0/02".to_string(), *complete)]);
        assert_eq!(
            statuses_of(&input, &phases, "from_planned"),
            "holds",
            "{complete}"
        );
    }
}

fn statuses_of(input: &PassInput, phases: &BTreeMap<String, &'static str>, id: &str) -> String {
    statuses(input, phases)[id].to_string()
}

#[test]
fn last_verified_is_a_fresh_holds_verdicts_time() {
    let input = project(&[
        ("holds", None, json!([judged("fresh", Some("holds"))])),
        ("stale", None, json!([judged("stale", Some("holds"))])),
        ("violated", None, json!([judged("fresh", Some("violated"))])),
        (
            "pending",
            Some("v1.0/02"),
            json!([judged("fresh", Some("holds"))]),
        ),
        ("unjudged", None, json!([])),
    ]);
    let values = values(&input, &statuses(&input, &phases()));
    assert_eq!(found(&values, "holds", "last_verified"), Some(json!(AT)));
    for id in ["stale", "violated", "pending", "unjudged"] {
        assert_eq!(found(&values, id, "last_verified"), None, "{id}");
        assert!(found(&values, id, "status").is_some(), "{id}");
    }
}
