use serde_json::json;

use super::*;

fn value_of(out: &[PassValue], entity: &str, field: &str) -> Option<Value> {
    out.iter()
        .find(|v| v.entity == entity && v.field == field)
        .map(|v| v.value.clone())
}

/// Milestone v1.0 with phase 01 tracing to R1 (and R2 when `orphan`), and an audit of it.
fn input(extra: Vec<Value>, orphan: bool) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0", "kind": "milestone" }),
        json!({ "id": "v1.0/01", "kind": "phase", "values": { "requirements": ["R1", "R2"] } }),
        json!({ "id": "R1", "kind": "requirement" }),
        json!({ "id": "R2", "kind": "requirement" }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
        json!({ "id": "v1.0/A1", "kind": "milestone_audit", "fields": { "milestone": "v1.0" } }),
    ];
    entities.extend(extra);
    let mut edges = vec![
        json!({ "source": "v1.0/01", "target": "v1.0", "label": "milestone" }),
        json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }),
        json!({ "source": "v1.0/01/01-01", "target": "R1", "label": "requirements" }),
    ];
    if !orphan {
        edges.push(json!({ "source": "v1.0/01/01-01", "target": "R2", "label": "requirements" }));
    }
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges }),
    ))
    .expect("a pass input")
}

fn audit(
    input: &PassInput,
    states: &[(&'static str, &'static str)],
    passed: bool,
) -> Vec<PassValue> {
    let states: BTreeMap<&str, &'static str> = states.iter().copied().collect();
    let verified = BTreeMap::from([(
        "v1.0/01".to_string(),
        Verified {
            status: if passed { "passed" } else { "pending" },
            at: None,
        },
    )]);
    audit_values(input, &states, &verified)
}

#[test]
fn an_orphaned_requirement_makes_gaps_found() {
    let found = input(vec![], true);
    let out = audit(&found, &[("R1", "complete"), ("R2", "pending")], true);
    assert_eq!(
        value_of(&out, "v1.0/A1", "status"),
        Some(json!("gaps_found"))
    );
    assert_eq!(
        value_of(&out, "v1.0/A1", "gaps"),
        Some(json!([{ "requirement": "R2", "status": "orphaned" }]))
    );
}

#[test]
fn an_unsatisfied_requirement_names_its_status() {
    let found = input(vec![], false);
    let out = audit(&found, &[("R1", "complete"), ("R2", "in_progress")], true);
    assert_eq!(
        value_of(&out, "v1.0/A1", "gaps"),
        Some(json!([{ "requirement": "R2", "status": "partial" }]))
    );
    // A partial gap alone, on a verified phase, is not a failure.
    assert_eq!(value_of(&out, "v1.0/A1", "status"), Some(json!("passed")));
}

#[test]
fn tech_debt_without_gaps_is_tech_debt() {
    let mut found = input(vec![], false);
    found.entities[5]
        .values
        .insert("tech_debt".into(), json!([{ "item": "slow list" }]));
    let out = audit(&found, &[("R1", "complete"), ("R2", "complete")], true);
    assert_eq!(
        value_of(&out, "v1.0/A1", "status"),
        Some(json!("tech_debt"))
    );
    let clean = audit(
        &input(vec![], false),
        &[("R1", "complete"), ("R2", "complete")],
        true,
    );
    assert_eq!(value_of(&clean, "v1.0/A1", "status"), Some(json!("passed")));
}

#[test]
fn a_phase_whose_verification_is_not_passed_makes_gaps_found() {
    let out = audit(
        &input(vec![], false),
        &[("R1", "complete"), ("R2", "complete")],
        false,
    );
    assert_eq!(
        value_of(&out, "v1.0/A1", "status"),
        Some(json!("gaps_found"))
    );
}

#[test]
fn nyquist_counts_by_phase() {
    let out = audit(
        &input(vec![], false),
        &[("R1", "complete"), ("R2", "complete")],
        true,
    );
    assert_eq!(
        value_of(&out, "v1.0/A1", "nyquist"),
        Some(json!({ "compliant": 0, "missing": 1, "not_validated": 0, "partial": 0 }))
    );
    let validation = json!({ "id": "v1.0/01/01-VALIDATION", "kind": "validation",
        "fields": { "phase": "v1.0/01", "approval": "x" } });
    let task = json!({ "id": "v1.0/01/01-01/T1", "kind": "task" });
    let found = input(vec![validation, task], false);
    let out = audit(&found, &[("R1", "complete"), ("R2", "complete")], true);
    // Validated, but the phase's task has no row: not compliant.
    assert_eq!(
        value_of(&out, "v1.0/A1", "nyquist"),
        Some(json!({ "compliant": 0, "missing": 0, "not_validated": 0, "partial": 1 }))
    );
}

#[test]
fn a_validated_validation_with_green_rows_is_compliant() {
    let validation = json!({ "id": "v1.0/01/01-VALIDATION", "kind": "validation",
        "fields": { "phase": "v1.0/01", "approval": "x" } });
    let task = json!({ "id": "v1.0/01/01-01/T1", "kind": "task" });
    let row = json!({ "id": "v1.0/01/01-VALIDATION/VR1", "kind": "validation_row",
        "fields": { "validation": "v1.0/01/01-VALIDATION", "task": "v1.0/01/01-01/T1" } });
    let mut found = input(vec![validation, task, row], false);
    found.test_results = serde_json::from_value(json!({
        "results": { "v1.0/01/01-01/T1": { "tests": [{ "name": "t", "status": "passed" }] } }
    }))
    .expect("a report");
    let out = validation_values(&found);
    assert_eq!(
        value_of(&out, "v1.0/01/01-VALIDATION", "status"),
        Some(json!("validated"))
    );
    assert_eq!(
        value_of(&out, "v1.0/01/01-VALIDATION", "nyquist_compliant"),
        Some(json!(true))
    );
    assert_eq!(
        value_of(&out, "v1.0/01/01-VALIDATION/VR1", "status"),
        Some(json!("green"))
    );
    let states = audit(&found, &[("R1", "complete"), ("R2", "complete")], true);
    assert_eq!(
        value_of(&states, "v1.0/A1", "nyquist"),
        Some(json!({ "compliant": 1, "missing": 0, "not_validated": 0, "partial": 0 }))
    );
}
