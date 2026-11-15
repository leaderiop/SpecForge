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

/// A context decision another decision supersedes no longer holds: no plan must list it, under every profile (the pass
/// reads no setting). The one that supersedes it is tracked.
#[test]
fn a_superseded_context_decision_needs_no_plan() {
    let input = |listed: &[&str]| {
        let mut edges = vec![edge("v/1/D-05", "v/1/D-03", "supersedes")];
        edges.extend(listed.iter().map(|d| edge("v/1/1-01", d, "decisions")));
        PassInput {
            entities: vec![
                entity("v/1", "phase", &[]),
                entity("v/1/1-01", "plan", &[]),
                entity("v/1/D-03", "decision", &[("origin", "context")]),
                entity("v/1/D-05", "decision", &[("origin", "context")]),
            ],
            edges,
            ..Default::default()
        }
    };
    assert!(check(&input(&["v/1/D-05"])).is_empty());
    assert_eq!(codes_of(&check(&input(&[]))), [("E310", Some("v/1/D-05"))]);
}

/// Phase `v/1` with one plan that lists no decision, its context decisions `decisions` (`(name, fields)`) and the
/// `supersedes` edges `(by, earlier)` between them: the decisions E310 names, by name.
fn uncited(decisions: &[(&str, &[(&str, &str)])], supersedes: &[(&str, &str)]) -> Vec<String> {
    let id = |name: &str| format!("v/1/{name}");
    let mut entities = vec![entity("v/1", "phase", &[]), entity("v/1/1-01", "plan", &[])];
    entities.extend(decisions.iter().map(|(name, fields)| {
        let mut fields = fields.to_vec();
        if !fields.iter().any(|(field, _)| *field == "origin") {
            fields.push(("origin", "context"));
        }
        entity(&id(name), "decision", &fields)
    }));
    let input = PassInput {
        entities,
        edges: supersedes
            .iter()
            .map(|(by, earlier)| edge(&id(by), &id(earlier), "supersedes"))
            .collect(),
        ..Default::default()
    };
    let found = check(&input);
    assert!(found.iter().all(|d| d.code == "E310"), "{found:?}");
    found
        .iter()
        .filter_map(|d| d.entity.as_deref()?.strip_prefix("v/1/"))
        .map(str::to_string)
        .collect()
}

/// Context decisions named `names`, with no field of their own.
fn plain(names: &[&'static str]) -> Vec<(&'static str, &'static [(&'static str, &'static str)])> {
    names.iter().map(|name| (*name, &[][..])).collect()
}

/// A decision that supersedes itself, directly or round a cycle, is reversed by nothing: every decision of the cycle
/// stays E310. An edge from a cycle to a decision outside it still retires that decision, as a chain's edges do.
#[test]
fn a_supersedes_cycle_retires_none_of_its_decisions() {
    assert_eq!(uncited(&plain(&["D-01"]), &[("D-01", "D-01")]), ["D-01"]);
    assert_eq!(
        uncited(
            &plain(&["D-01", "D-02"]),
            &[("D-01", "D-02"), ("D-02", "D-01")]
        ),
        ["D-01", "D-02"]
    );
    // D-01 -> D-02 -> D-03 -> D-01 is a cycle; D-03 -> D-04 leaves it.
    let three_and_out = [
        ("D-01", "D-02"),
        ("D-02", "D-03"),
        ("D-03", "D-01"),
        ("D-03", "D-04"),
    ];
    assert_eq!(
        uncited(&plain(&["D-01", "D-02", "D-03", "D-04"]), &three_and_out),
        ["D-01", "D-02", "D-03"]
    );
    // A chain retires every decision but its head.
    assert_eq!(
        uncited(
            &plain(&["D-01", "D-02", "D-03"]),
            &[("D-03", "D-02"), ("D-02", "D-01")]
        ),
        ["D-03"]
    );
    // Two cycles joined by one edge: the edge retires its target only, and that cycle's other member stays.
    let joined = [
        ("D-01", "D-02"),
        ("D-02", "D-01"),
        ("D-03", "D-04"),
        ("D-04", "D-03"),
        ("D-02", "D-03"),
    ];
    assert_eq!(
        uncited(&plain(&["D-01", "D-02", "D-03", "D-04"]), &joined),
        ["D-01", "D-02", "D-04"]
    );
}

/// A settled decision, locked with the user, leaves force only through a decision taken with the user: an agent's
/// `summary` or `checkpoint` decision that supersedes it leaves it E310, a `context` or `project` one retires it. An
/// agent's decision still retires an unsettled one.
#[test]
fn only_a_decision_taken_with_the_user_retires_a_settled_one() {
    let settled: &[(&str, &str)] = &[("settled", "true")];
    for (origin, retired) in [
        ("summary", false),
        ("checkpoint", false),
        ("context", true),
        ("project", true),
    ] {
        let later: &[(&str, &str)] = &[("origin", origin)];
        let found = uncited(&[("D-01", settled), ("D-02", later)], &[("D-02", "D-01")]);
        let mut expected = Vec::new();
        if !retired {
            expected.push("D-01");
        }
        if origin == "context" {
            expected.push("D-02");
        }
        assert_eq!(found, expected, "{origin}");
    }
    let agent: &[(&str, &str)] = &[("origin", "summary")];
    assert_eq!(
        uncited(&[("D-01", &[]), ("D-02", agent)], &[("D-02", "D-01")]),
        Vec::<String>::new()
    );
}
