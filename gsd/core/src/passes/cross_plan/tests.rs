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

/// Phase `v/1` with one plan whose objective and task action are given, and the plan's `decisions` edges.
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
fn a_decision_is_covered_only_by_a_decisions_edge() {
    let text = "Log every failed retry to the audit trail, with its cause";
    // Prose alone, by name or by its first six words, covers nothing.
    for (objective, action) in [
        ("Honour D-03", "Write it"),
        ("Build it", "See D-03."),
        ("Build it", "Log every failed retry to the audit trail."),
    ] {
        assert_eq!(
            codes_of(&check(&decision_input(text, objective, action, &[]))),
            [("E310", Some("v/1/D-03"))],
            "{objective} / {action}"
        );
    }
    assert!(check(&decision_input(text, "Build it", "Write it", &["v/1/D-03"])).is_empty());
    // An edge to another decision, or to the project's namesake, covers nothing.
    for other in ["v/1/D-030", "D-03"] {
        assert_eq!(
            codes_of(&check(&decision_input(
                text,
                "Build it",
                "Write it",
                &[other]
            ))),
            [("E310", Some("v/1/D-03"))],
            "{other}"
        );
    }
}

/// A plan of phase 2 that lists phase 1's `D-01` covers nothing of phase 2: its own `D-01` is still uncited.
#[test]
fn a_citation_of_another_phases_decision_covers_nothing_here() {
    let input = PassInput {
        entities: vec![
            entity("v/1", "phase", &[]),
            entity("v/1/1-01", "plan", &[("objective", "Honour D-01")]),
            entity(
                "v/1/D-01",
                "decision",
                &[("origin", "context"), ("text", "ES modules")],
            ),
            entity("v/2", "phase", &[]),
            entity(
                "v/2/2-01",
                "plan",
                &[("objective", "Keep phase 1's D-01 (ES modules)")],
            ),
            entity(
                "v/2/D-01",
                "decision",
                &[
                    ("origin", "context"),
                    ("text", "Tasks are kept in one JSON file"),
                ],
            ),
        ],
        edges: vec![
            edge("v/1/1-01", "v/1/D-01", "decisions"),
            edge("v/2/2-01", "v/1/D-01", "decisions"),
        ],
        ..Default::default()
    };
    assert_eq!(codes_of(&check(&input)), [("E310", Some("v/2/D-01"))]);
}

fn coupled(wave_a: &str, wave_b: &str, peer_scope: &str) -> Vec<PassDiagnostic> {
    let mut a = entity("v/1/1-01", "plan", &[("wave", wave_a)]);
    a.values.insert(
        "coupling_justified".into(),
        serde_json::json!([{ "peer": "1-02", "reason": "both read the store" }]),
    );
    let peer = format!("{peer_scope}/1-02");
    let b = entity(&peer, "plan", &[("wave", wave_b)]);
    couplings(&PassInput {
        entities: vec![a, b],
        edges: vec![edge("v/1/1-01", &peer, "coupling_justified.peer")],
        ..Default::default()
    })
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
