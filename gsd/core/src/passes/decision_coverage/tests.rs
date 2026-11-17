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

fn edge(source: &str, target: &str, label: &str) -> PassEdge {
    PassEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

/// Where the phase's one plan cites `D-03`, if anywhere.
#[derive(Clone, Copy, PartialEq)]
enum Cited {
    Nowhere,
    /// Listed in `decisions` and cited nowhere.
    Listed,
    Plan,
    Summary,
    Item,
    Deviation,
    Commit,
}

/// Phase `v/1` with its verification at `status`, and one plan at `plan_status` with its summary, a coverage item, a
/// deviation and a recorded commit; `cited` says which of them writes `D-03`. The summary repeats the decision's words
/// either way.
fn input(status: &str, cited: Cited, plan_status: &str) -> PassInput {
    let says = |place: Cited, text: &str| {
        if cited == place {
            format!("{text} (D-03)")
        } else {
            text.to_string()
        }
    };
    let mut edges = vec![
        edge("v/1/1-VERIFICATION", "v/1", "phase"),
        edge("abc123def456", "v/1/1-01", "plan"),
    ];
    if cited == Cited::Listed {
        edges.push(edge("v/1/1-01", "v/1/D-03", "decisions"));
    }
    PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity(
                "v/1/1-01",
                "plan",
                &[
                    ("status", plan_status),
                    ("objective", &says(Cited::Plan, "Log the retries")),
                ],
            ),
            entity(
                "v/1/1-01/1-01-SUMMARY",
                "summary",
                &[(
                    "one_liner",
                    &says(Cited::Summary, "Log every retry with its reason"),
                )],
            ),
            entity(
                "v/1/1-01/1-01-SUMMARY/D1",
                "deliverable",
                &[("description", &says(Cited::Item, "The retry log"))],
            ),
            entity(
                "v/1/1-01/1-01-SUMMARY/DV1",
                "deviation",
                &[("description", &says(Cited::Deviation, "Log moved"))],
            ),
            entity(
                "abc123def456",
                "commit",
                &[(
                    "subject",
                    &says(Cited::Commit, "feat(1-01): log every retry"),
                )],
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
    for cited in [Cited::Nowhere, Cited::Listed] {
        let found = check(&input("passed", cited, "complete"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, "W506");
        assert_eq!(found[0].entity.as_deref(), Some("v/1/D-03"));
        assert_eq!(
            found[0].message,
            "decision 'v/1/D-03' of verified phase 'v/1' is cited by no complete plan of the phase, its summaries or its commits"
        );
    }
}

#[test]
fn w506_a_complete_plan_citing_the_decision_covers_it() {
    for cited in [
        Cited::Plan,
        Cited::Summary,
        Cited::Item,
        Cited::Deviation,
        Cited::Commit,
    ] {
        assert!(check(&input("passed", cited, "complete")).is_empty());
        // A plan that is not complete ships nothing, whatever cites it.
        assert_eq!(check(&input("passed", cited, "halted")).len(), 1);
    }
}

#[test]
fn a_phase_not_verified_is_not_judged() {
    assert!(check(&input("gaps_found", Cited::Nowhere, "complete")).is_empty());
    assert!(check(&input("pending", Cited::Nowhere, "complete")).is_empty());
}

/// A verified phase's context decision a later decision says it reverses is still W506 until a complete plan cites
/// it: a decision has no supersession, so the later one, cited, covers only itself.
#[test]
fn every_context_decision_is_w506_with_no_supersession() {
    let mut reversed = input("passed", Cited::Nowhere, "complete");
    reversed.entities.push(entity(
        "v/1/D-05",
        "decision",
        &[
            ("origin", "context"),
            ("settled", "true"),
            ("text", "Supersedes D-03: a retry is not logged"),
        ],
    ));
    let plan = reversed
        .entities
        .iter_mut()
        .find(|e| e.id == "v/1/1-01")
        .expect("the plan");
    plan.fields
        .insert("objective".into(), "Implement D-05".into());
    let found = check(&reversed);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].entity.as_deref(), Some("v/1/D-03"));
}
