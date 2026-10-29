use serde_json::json;

use super::*;
use crate::passes::derive::derive;

fn verified(status: &'static str, at: Option<&str>) -> Verified {
    Verified {
        status,
        at: at.map(String::from),
    }
}

fn value_of(out: &PassOutput, entity: &str, field: &str) -> Option<serde_json::Value> {
    out.values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| v.value.clone())
}

/// A milestone with one phase of one plan with a summary, the phase tracing to requirement R1, and a verification of
/// the phase whose single truth has this judgement.
fn input(outcome: &str, passed: bool) -> PassInput {
    serde_json::from_value(crate::passes::plans::with_field_edges(json!({
        "entities": [
            { "id": "v1.0", "kind": "milestone" },
            { "id": "v1.0/01", "kind": "phase" },
            { "id": "R1", "kind": "requirement" },
            { "id": "v1.0/01/01-01", "kind": "plan" },
            { "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary",
              "fields": { "status": "complete" } },
            { "id": "v1.0/01/01-01/TR1", "kind": "truth", "judgements": [
                { "rule": "gsd/E501", "state": "fresh", "passed": passed, "outcome": outcome,
                  "at": "2026-10-29T10:00:00Z" } ] },
            { "id": "v1.0/01/01-VERIFICATION", "kind": "verification", "fields": { "phase": "v1.0/01" } },
        ],
        "edges": [
            { "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" },
            { "source": "v1.0/01", "target": "v1.0", "label": "milestone" },
            { "source": "v1.0/01", "target": "R1", "label": "requirements" },
            { "source": "v1.0/01/01-01", "target": "R1", "label": "requirements" },
        ],
    })))
    .expect("a pass input")
}

#[test]
fn the_ladder_up_to_executed() {
    assert_eq!(status(0, 0, None), "not_started");
    assert_eq!(status(2, 0, None), "planned");
    assert_eq!(status(2, 1, None), "in_progress");
    assert_eq!(status(2, 2, None), "executed");
    assert_eq!(status(2, 2, Some(&verified("pending", None))), "executed");
}

#[test]
fn a_passed_verification_makes_the_phase_complete_and_counts_in_progress() {
    let out = derive(&input("verified", true));
    assert_eq!(value_of(&out, "v1.0/01", "status"), Some(json!("complete")));
    assert_eq!(
        value_of(&out, "v1.0", "progress").map(|p| p["phases_complete"].clone()),
        Some(json!(1))
    );
}

#[test]
fn human_needed_is_needs_review() {
    assert_eq!(
        status(1, 1, Some(&verified("human_needed", None))),
        "needs_review"
    );
    let out = derive(&input("present_behavior_unverified", false));
    assert_eq!(
        value_of(&out, "v1.0/01", "status"),
        Some(json!("needs_review"))
    );
}

#[test]
fn gaps_found_stays_executed() {
    let out = derive(&input("failed", false));
    assert_eq!(value_of(&out, "v1.0/01", "status"), Some(json!("executed")));
    assert_eq!(
        value_of(&out, "v1.0", "progress").map(|p| p["phases_complete"].clone()),
        Some(json!(0))
    );
}

#[test]
fn a_complete_requirement_of_a_gaps_found_phase_is_gaps_found() {
    let out = derive(&input("failed", false));
    assert_eq!(value_of(&out, "R1", "status"), Some(json!("gaps_found")));
    let ok = derive(&input("verified", true));
    assert_eq!(value_of(&ok, "R1", "status"), Some(json!("complete")));
}

#[test]
fn completed_is_the_verified_date() {
    assert_eq!(
        completed(
            "complete",
            Some(&verified("passed", Some("2026-10-29T10:00:00Z")))
        ),
        Some("2026-10-29".into())
    );
    assert_eq!(
        completed(
            "executed",
            Some(&verified("pending", Some("2026-10-29T10:00:00Z")))
        ),
        None
    );
    assert_eq!(completed("complete", Some(&verified("passed", None))), None);
    let out = derive(&input("verified", true));
    assert_eq!(
        value_of(&out, "v1.0/01", "completed"),
        Some(json!("2026-10-29"))
    );
}
