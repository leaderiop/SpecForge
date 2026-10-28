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

#[test]
fn soft_phrase_normalisation_is_gsds() {
    assert_eq!(
        normalise("  Tasks, ARE kept\tin one-list!  "),
        "tasks are kept in one list"
    );
    assert_eq!(
        soft_phrase("Tasks are kept in a single JSON file, `tasks.json`").as_deref(),
        Some("tasks are kept in a single")
    );
    assert_eq!(soft_phrase("five words only here"), None);
}

fn decision_input(text: &str, objective: &str, task_action: &str) -> PassInput {
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
        ..Default::default()
    }
}

#[test]
fn an_uncited_context_decision_is_e310_on_the_decision() {
    let found = check(&decision_input("Log every retry", "Build it", "Write it"));
    assert_eq!(codes_of(&found), [("E310", Some("v/1/D-03"))]);
    assert_eq!(
        found[0].message,
        "decision 'v/1/D-03' is cited by no plan of phase 'v/1'"
    );
}

#[test]
fn a_decision_cited_by_name_or_by_its_first_six_words_is_covered() {
    assert!(
        check(&decision_input(
            "Log every retry",
            "Honour D-03",
            "Write it"
        ))
        .is_empty()
    );
    assert!(check(&decision_input("Log every retry", "Build it", "See D-03.")).is_empty());
    // A name inside a longer id is not a citation.
    assert!(
        !check(&decision_input(
            "Log every retry",
            "Honour D-030",
            "Write it"
        ))
        .is_empty()
    );
    let text = "Log every failed retry to the audit trail, with its cause";
    assert!(
        check(&decision_input(
            text,
            "Build it",
            "Log every failed retry to the audit trail."
        ))
        .is_empty()
    );
    assert!(!check(&decision_input(text, "Build it", "Log every failed retry")).is_empty());
}

#[test]
fn only_the_scanned_surfaces_cite() {
    let mut input = decision_input("Log every retry", "Build it", "Write it");
    input.entities[1]
        .fields
        .insert("requirements".into(), "D-03".into());
    assert_eq!(
        check(&input).len(),
        1,
        "a plan's requirements are not scanned"
    );
    input.entities[1]
        .fields
        .insert("purpose".into(), "D-03".into());
    assert!(check(&input).is_empty());
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
