use serde_json::{Value, json};

use super::*;

/// A roadmap: milestones v1.0 and v2.0, phase `v1.0/01` listing R1 and phase `v2.0/01` listing R2 (R3 is in no phase),
/// and a journey `j` whose steps name `steps` (an empty one names none), whose own `requirements` are `whole` and
/// which lists `milestones`.
fn roadmap(steps: &[&str], whole: &[&str], milestones: &[&str]) -> PassInput {
    let rows: Vec<Value> = steps
        .iter()
        .map(|r| {
            if r.is_empty() {
                json!({ "action": "looks" })
            } else {
                json!({ "action": "does", "requirement": r })
            }
        })
        .collect();
    let mut edges = vec![
        json!({ "source": "v1.0/01", "label": "milestone", "target": "v1.0" }),
        json!({ "source": "v1.0/01", "label": "requirements", "target": "R1" }),
        json!({ "source": "v2.0/01", "label": "milestone", "target": "v2.0" }),
        json!({ "source": "v2.0/01", "label": "requirements", "target": "R2" }),
    ];
    for r in steps.iter().filter(|r| !r.is_empty()) {
        edges.push(json!({ "source": "j", "label": "steps.requirement", "target": r }));
    }
    for r in whole {
        edges.push(json!({ "source": "j", "label": "requirements", "target": r }));
    }
    for m in milestones {
        edges.push(json!({ "source": "j", "label": "milestones", "target": m }));
    }
    serde_json::from_value(json!({
        "entities": [
            { "id": "R1", "kind": "requirement", "fields": { "text": "one" } },
            { "id": "R2", "kind": "requirement", "fields": { "text": "two" } },
            { "id": "R3", "kind": "requirement", "fields": { "text": "three" } },
            { "id": "v1.0", "kind": "milestone", "fields": { "name": "1.0" } },
            { "id": "v2.0", "kind": "milestone", "fields": { "name": "2.0" } },
            { "id": "v1.0/01", "kind": "phase", "fields": { "milestone": "v1.0" } },
            { "id": "v2.0/01", "kind": "phase", "fields": { "milestone": "v2.0" } },
            { "id": "j", "kind": "journey", "fields": { "persona": "owner" },
              "values": { "steps": rows, "requirements": whole, "milestones": milestones } },
        ],
        "edges": edges,
    }))
    .expect("a pass input")
}

/// `(code, entity, message)` of each diagnostic.
fn found(input: &PassInput) -> Vec<(String, String, String)> {
    check(input)
        .into_iter()
        .map(|d| (d.code, d.entity.unwrap_or_default(), d.message))
        .collect()
}

fn messages(input: &PassInput) -> Vec<String> {
    found(input).into_iter().map(|(_, _, m)| m).collect()
}

#[test]
fn a_step_whose_requirement_no_phase_of_the_milestone_holds_is_w530() {
    let found = found(&roadmap(&["R1", "R2"], &[], &["v1.0"]));
    assert_eq!(
        found,
        [(
            "W530".to_string(),
            "j".to_string(),
            "journey 'j' lists milestone 'v1.0', but no phase of it lists R2, named by step 2"
                .to_string()
        )]
    );
}

#[test]
fn an_unphased_requirement_is_w530() {
    assert_eq!(
        messages(&roadmap(&["R3"], &[], &["v1.0"])),
        ["journey 'j' lists milestone 'v1.0', but no phase of it lists R3, named by step 1"]
    );
}

#[test]
fn one_w530_per_requirement_naming_its_steps() {
    assert_eq!(
        messages(&roadmap(&["R2", "R1", "R2", "", "R2"], &["R2"], &["v1.0"])),
        [
            "journey 'j' lists milestone 'v1.0', but no phase of it lists R2, named by steps 1, 3 and 5 and the \
             journey's requirements"
        ]
    );
}

#[test]
fn a_journey_level_requirement_outside_is_w530() {
    assert_eq!(
        messages(&roadmap(&["R1"], &["R3"], &["v1.0"])),
        [
            "journey 'j' lists milestone 'v1.0', but no phase of it lists R3, named by the journey's requirements"
        ]
    );
}

/// Each listed milestone is a promise of its own: R1 is phased in v1.0 only, so v2.0 cannot deliver the journey.
#[test]
fn each_listed_milestone_is_checked() {
    assert_eq!(
        messages(&roadmap(&["R1"], &[], &["v1.0", "v2.0"])),
        ["journey 'j' lists milestone 'v2.0', but no phase of it lists R1, named by step 1"]
    );
}

#[test]
fn a_step_phased_in_the_milestone_is_not_w530() {
    assert_eq!(found(&roadmap(&["R1", "R1"], &["R1"], &["v1.0"])), []);
}

#[test]
fn a_journey_with_no_milestone_is_not_w530() {
    assert_eq!(found(&roadmap(&["R3", "R2"], &["R3"], &[])), []);
}

#[test]
fn a_step_with_no_requirement_is_not_w530() {
    assert_eq!(found(&roadmap(&["", "R1", ""], &[], &["v1.0"])), []);
}
