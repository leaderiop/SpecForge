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

fn codes_of(found: &[PassDiagnostic]) -> Vec<(&str, Option<&str>)> {
    found
        .iter()
        .map(|d| (d.code.as_str(), d.entity.as_deref()))
        .collect()
}

#[test]
fn a_requirement_in_no_plan_is_e301_on_the_phase() {
    let input = PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/1-01", "plan", &[]),
            entity("REQ-01", "requirement", &[]),
            entity("REQ-02", "requirement", &[]),
        ],
        edges: vec![
            edge("v/1", "REQ-01", "requirements"),
            edge("v/1", "REQ-02", "requirements"),
            edge("v/1/1-01", "REQ-01", "requirements"),
        ],
        ..Default::default()
    };
    let found = check(&input);
    assert_eq!(codes_of(&found), [("E301", Some("v/1"))]);
    assert_eq!(
        found[0].message,
        "requirement 'REQ-02' of phase 'v/1' is in no plan of the phase"
    );
}

#[test]
fn an_unplanned_phase_is_not_judged() {
    let input = PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/D-01", "decision", &[("origin", "context")]),
        ],
        edges: vec![edge("v/1", "REQ-01", "requirements")],
        ..Default::default()
    };
    assert!(check(&input).is_empty());
}

/// Phase `v/1` with one plan whose objective and task action are given, and the plan's `decisions` edges (still
/// declared, read by no rule).
fn decision_input(text: &str, objective: &str, task_action: &str, listed: &[&str]) -> PassInput {
    PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/1-01", "plan", &[("objective", objective)]),
            entity("v/1/1-01/T1", "task", &[("action", task_action)]),
            entity(
                "v/1/D-03",
                "decision",
                &[("origin", "context"), ("text", text)],
            ),
            entity("v/1/D-04", "decision", &[("origin", "summary")]),
            entity("D-03", "decision", &[("origin", "context")]),
        ],
        edges: listed
            .iter()
            .map(|target| edge("v/1/1-01", target, "decisions"))
            .collect(),
        ..Default::default()
    }
}

#[test]
fn an_uncited_context_decision_is_e310_on_the_decision() {
    let found = check(&decision_input(
        "Log every retry",
        "Build it",
        "Write it",
        &[],
    ));
    assert_eq!(codes_of(&found), [("E310", Some("v/1/D-03"))]);
    assert_eq!(
        found[0].message,
        "decision 'v/1/D-03' is cited by no plan of phase 'v/1'"
    );
}

#[test]
fn a_decision_is_covered_by_its_id_in_a_plans_text() {
    let text = "Log every failed retry to the audit trail, with its cause";
    // The id, in the plan's objective or in a task, covers it.
    for (objective, action) in [("Honour D-03", "Write it"), ("Build it", "See D-03.")] {
        assert!(
            check(&decision_input(text, objective, action, &[])).is_empty(),
            "{objective} / {action}"
        );
    }
    // Its first six words, another id that ends with it, and its name in another case cover nothing.
    for (objective, action) in [
        ("Build it", "Log every failed retry to the audit trail."),
        ("Build it", "See D-030 and XD-03"),
        ("Build it", "See d-03"),
    ] {
        assert_eq!(
            codes_of(&check(&decision_input(text, objective, action, &[]))),
            [("E310", Some("v/1/D-03"))],
            "{objective} / {action}"
        );
    }
    // A `decisions` edge with no citation covers nothing: no rule reads the list.
    assert_eq!(
        codes_of(&check(&decision_input(
            text,
            "Build it",
            "Write it",
            &["v/1/D-03"]
        ))),
        [("E310", Some("v/1/D-03"))]
    );
    // The project's namesake, cited by its rooted path, covers nothing of the phase.
    assert_eq!(
        codes_of(&check(&decision_input(
            text,
            "Honour /D-03",
            "Write it",
            &[]
        ))),
        [("E310", Some("v/1/D-03"))]
    );
}

/// The roadmap's case: plan 02-01's task writes `01/D-01`. That cites phase 01's decision, so phase 02's own `D-01`
/// is still uncited, and phase 01's, cited by its own plan, is untouched.
#[test]
fn phase_two_citing_01_d_01_leaves_its_own_d_01_uncited() {
    let input = PassInput {
        entities: vec![
            entity("v1.0/01", "phase", &[]),
            entity("v1.0/01/01-01", "plan", &[("objective", "Honour D-01")]),
            entity(
                "v1.0/01/D-01",
                "decision",
                &[("origin", "context"), ("text", "ES modules")],
            ),
            entity("v1.0/02", "phase", &[]),
            entity("v1.0/02/02-01", "plan", &[("objective", "Store the tasks")]),
            entity(
                "v1.0/02/02-01/T1",
                "task",
                &[("action", "Keep phase 1's 01/D-01 (ES modules)")],
            ),
            entity(
                "v1.0/02/D-01",
                "decision",
                &[
                    ("origin", "context"),
                    ("text", "Tasks are kept in one JSON file"),
                ],
            ),
        ],
        ..Default::default()
    };
    assert_eq!(codes_of(&check(&input)), [("E310", Some("v1.0/02/D-01"))]);
    assert!(
        check(&input)[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("by its path, such as 01/D-01")),
        "{:?}",
        check(&input)
    );
}

/// Plan `v/1/1-01` of `wave_a` naming its peer `<peer_scope>/1-02` of `wave_b` as the writer spells it from its own
/// scope (`1-02` in its phase, `2/1-02` in phase 2), and the edge the graph draws.
fn coupled(wave_a: &str, wave_b: &str, peer_scope: &str) -> Vec<PassDiagnostic> {
    let mut a = entity("v/1/1-01", "plan", &[("wave", wave_a)]);
    let written = if peer_scope == "v/1" {
        "1-02"
    } else {
        "2/1-02"
    };
    a.values.insert(
        "coupling_justified".into(),
        serde_json::json!([{ "peer": written, "reason": "both read the store" }]),
    );
    let peer = format!("{peer_scope}/1-02");
    let b = entity(&peer, "plan", &[("wave", wave_b)]);
    couplings(&PassInput {
        entities: vec![a, b],
        edges: vec![edge("v/1/1-01", &peer, "coupling_justified.peer")],
        ..Default::default()
    })
}

/// Plan `v/1/1-01` of wave 1 naming `rows` in `coupling_justified`, beside `v/1/1-02` (wave 1) and `v/2/2-01`
/// (wave 1), with the `coupling_justified.peer` edges the graph draws to `resolved`.
fn coupling_rows(rows: serde_json::Value, resolved: &[&str]) -> Vec<PassDiagnostic> {
    let mut a = entity("v/1/1-01", "plan", &[("wave", "1")]);
    a.values.insert("coupling_justified".into(), rows);
    let peers = ["v/1/1-02", "v/2/2-01"].map(|id| entity(id, "plan", &[("wave", "1")]));
    let mut entities = vec![a];
    entities.extend(peers);
    couplings(&PassInput {
        entities,
        edges: resolved
            .iter()
            .map(|peer| edge("v/1/1-01", peer, "coupling_justified.peer"))
            .collect(),
        ..Default::default()
    })
}

/// A row's peer is the plan the kernel resolves it to from the plan: a rooted path is the project's, and a name
/// outside the plan's scope chain (E003) is nothing, though another row's peer ends with it.
#[test]
fn a_coupling_peer_is_the_plan_the_kernel_resolves_it_to() {
    let rooted = coupling_rows(
        serde_json::json!([{ "peer": "/v/1/1-02", "reason": "both read the store" }]),
        &["v/1/1-02"],
    );
    assert_eq!(
        codes_of(&rooted),
        [("I321", Some("v/1/1-01"))],
        "the rooted peer"
    );
    assert_eq!(
        rooted[0].message,
        "coupling between 'v/1/1-01' and 'v/1/1-02' is justified: both read the store"
    );
    let short = coupling_rows(
        serde_json::json!([
            { "peer": "2/2-01", "reason": "a" },
            { "peer": "2-01", "reason": "b" }
        ]),
        &["v/2/2-01"],
    );
    assert_eq!(short.len(), 1, "2-01 resolves to nothing: {short:?}");
}

#[test]
fn a_justified_coupling_is_i321() {
    let found = coupled("1", "1", "v/1");
    assert_eq!(codes_of(&found), [("I321", Some("v/1/1-01"))]);
    assert_eq!(found[0].severity, PassSeverity::Info);
    assert_eq!(
        found[0].message,
        "coupling between 'v/1/1-01' and 'v/1/1-02' is justified: both read the store"
    );
}

#[test]
fn a_coupling_peer_in_another_wave_or_phase_covers_nothing() {
    for found in [coupled("1", "2", "v/1"), coupled("1", "1", "v/2")] {
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0]
                .message
                .contains("is not in its wave: the exemption covers nothing"),
            "{found:?}"
        );
    }
}

/// Every context decision is counted, as GSD counts it: a decision has no supersession, so one whose text a later
/// decision names as reversed, or that another decision of the context restates, is still E310 until a plan cites it,
/// whatever the later decision's origin and whether either is settled.
#[test]
fn every_context_decision_is_counted_with_no_supersession() {
    for origin in ["context", "project", "summary", "checkpoint"] {
        for settled in ["true", "false"] {
            let input = PassInput {
                entities: vec![
                    entity("v/1", "phase", &[]),
                    entity(
                        "v/1/1-01",
                        "plan",
                        &[("objective", "Implement D-04 and D-05")],
                    ),
                    entity(
                        "v/1/D-03",
                        "decision",
                        &[
                            ("origin", "context"),
                            ("text", "Tasks are kept in one JSON file"),
                        ],
                    ),
                    entity(
                        "v/1/D-04",
                        "decision",
                        &[
                            ("origin", "context"),
                            ("text", "Tasks are kept in one JSON file"),
                        ],
                    ),
                    entity(
                        "v/1/D-05",
                        "decision",
                        &[
                            ("origin", origin),
                            ("settled", settled),
                            ("text", "Supersedes D-03: tasks are kept in SQLite"),
                        ],
                    ),
                ],
                ..Default::default()
            };
            assert_eq!(
                codes_of(&check(&input)),
                [("E310", Some("v/1/D-03"))],
                "{origin}, settled {settled}"
            );
        }
    }
}
