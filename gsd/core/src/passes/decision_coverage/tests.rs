use super::*;

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

/// Phase `v/1` with its verification at `status`, and one plan at `plan_status` that lists `v/1/D-03` when `listed`;
/// its summary names D-03 and its commit's subject says the decision's words.
fn input(status: &str, listed: bool, plan_status: &str) -> PassInput {
    let mut edges = vec![PassEdge {
        source: "v/1/1-VERIFICATION".into(),
        target: "v/1".into(),
        label: "phase".into(),
    }];
    if listed {
        edges.push(PassEdge {
            source: "v/1/1-01".into(),
            target: "v/1/D-03".into(),
            label: "decisions".into(),
        });
    }
    PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/1-01", "plan", &[("status", plan_status)]),
            entity(
                "v/1/1-01/1-01-SUMMARY",
                "summary",
                &[("one_liner", "Implements D-03")],
            ),
            entity(
                "v/1/1-01/abc",
                "commit",
                &[("subject", "feat(1-01): log every retry with its reason")],
            ),
            entity("v/1/1-VERIFICATION", "verification", &[("status", status)]),
            entity(
                "v/1/D-03",
                "decision",
                &[
                    ("origin", "context"),
                    ("text", "Log every retry with its reason"),
                ],
            ),
            entity("v/1/D-04", "decision", &[("origin", "summary")]),
        ],
        edges,
        ..Default::default()
    }
}

#[test]
fn a_decision_nothing_shipped_is_w506_on_the_decision() {
    let found = check(&input("passed", false, "complete"));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].code, "W506");
    assert_eq!(found[0].entity.as_deref(), Some("v/1/D-03"));
    assert_eq!(
        found[0].message,
        "decision 'v/1/D-03' of verified phase 'v/1' is listed by no complete plan of the phase"
    );
}

#[test]
fn w506_a_complete_plan_listing_the_decision_covers_it() {
    assert!(check(&input("passed", true, "complete")).is_empty());
    assert_eq!(check(&input("passed", true, "halted")).len(), 1);
}

#[test]
fn a_phase_not_verified_is_not_judged() {
    assert!(check(&input("gaps_found", false, "complete")).is_empty());
    assert!(check(&input("pending", false, "complete")).is_empty());
}
