use serde_json::{Value, json};

use super::*;

fn input(trigger: Value, entities: Vec<Value>, edges: Vec<Value>) -> PassInput {
    serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges, "trigger": trigger }),
    ))
    .expect("a pass input")
}

fn ship(subject: Option<&str>) -> Value {
    gate_of("ship", subject)
}

fn codes(found: Vec<PassDiagnostic>) -> Vec<String> {
    found.into_iter().map(|d| d.code).collect()
}

fn window(id: &str, extra: Value) -> Value {
    json!({ "id": id, "kind": "broken_window", "fields": extra })
}

fn phase_edge() -> Vec<Value> {
    vec![json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" })]
}

#[test]
fn ship_with_an_open_window_is_refused_e512() {
    let found = input(ship(None), vec![window("BW1", json!({}))], vec![]);
    assert_eq!(codes(e512(&found)), ["E512"]);
}

#[test]
fn e512_names_the_subject_of_a_scoped_gate() {
    // A window is project-scoped, so the host would drop a diagnostic that names only it from a gate on a phase.
    let found = input(
        ship(Some("v1.0/01")),
        vec![window("BW1", json!({}))],
        vec![],
    );
    let refused = e512(&found);
    assert_eq!(codes(refused.clone()), ["E512"]);
    assert_eq!(refused[0].entity.as_deref(), Some("v1.0/01"));
    let project = input(ship(None), vec![window("BW1", json!({}))], vec![]);
    assert_eq!(e512(&project)[0].entity.as_deref(), Some("BW1"));
}

#[test]
fn a_gate_is_told_by_its_kind_not_by_a_command() {
    let no_command = json!({ "kind": "gate", "subject": null });
    let found = input(no_command, vec![window("BW1", json!({}))], vec![]);
    assert_eq!(codes(e512(&found)), ["E512"]);
    let write_with_a_command = json!({ "kind": "write", "command": "gsd-ship" });
    let found = input(write_with_a_command, vec![window("BW1", json!({}))], vec![]);
    assert!(e512(&found).is_empty());
}

#[test]
fn e512_is_silent_on_a_resolved_window_and_without_a_gate() {
    let waived = window("BW1", json!({ "waiver": "x" }));
    let fixed = window("BW2", json!({ "fixed_at": "2026-10-29T10:00:00Z" }));
    let resolved = input(ship(None), vec![waived, fixed], vec![]);
    assert_eq!(codes(e512(&resolved)), Vec::<String>::new());
    // A write has no trigger command: the open window refuses nothing.
    let write = input(
        json!({ "kind": "write" }),
        vec![window("BW1", json!({}))],
        vec![],
    );
    assert!(e512(&write).is_empty());
    let untriggered: PassInput =
        serde_json::from_value(json!({ "entities": [window("BW1", json!({}))] })).expect("input");
    assert!(e512(&untriggered).is_empty());
}

fn audited(status: &str, accepted: bool) -> PassInput {
    let mut fields = json!({ "milestone": "v1.0", "status": status });
    if accepted {
        fields["debt_accepted"] = json!("x");
    }
    input(
        ship(Some("v1.0")),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            json!({ "id": "v1.0/A1", "kind": "milestone_audit", "fields": fields }),
        ],
        vec![],
    )
}

#[test]
fn an_unaccepted_tech_debt_audit_is_e106() {
    assert_eq!(codes(e106(&audited("tech_debt", false))), ["E106"]);
    assert_eq!(codes(e106(&audited("gaps_found", true))), ["E106"]);
    assert!(e106(&audited("tech_debt", true)).is_empty());
    assert!(e106(&audited("passed", false)).is_empty());
    let none = input(
        ship(Some("v1.0")),
        vec![json!({ "id": "v1.0", "kind": "milestone" })],
        vec![],
    );
    assert_eq!(codes(e106(&none)), ["E106"]);
}

#[test]
fn e106_reads_the_latest_audit_by_number_and_only_at_a_milestone() {
    let audit = |n: u32, status: &str| {
        json!({ "id": format!("v1.0/A{n}"), "kind": "milestone_audit",
                "fields": { "milestone": "v1.0", "status": status } })
    };
    let found = input(
        ship(Some("v1.0")),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            audit(9, "gaps_found"),
            audit(10, "passed"),
        ],
        vec![],
    );
    assert!(e106(&found).is_empty());
    // A phase is not audited: a gate on one asks nothing.
    let phase = input(
        ship(Some("v1.0/01")),
        vec![json!({ "id": "v1.0/01", "kind": "phase" })],
        vec![],
    );
    assert!(e106(&phase).is_empty());
}

fn phase_gate(verification: Option<Value>, truth_outcome: Option<&str>) -> PassInput {
    let mut entities = vec![
        json!({ "id": "v1.0/01", "kind": "phase" }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
    ];
    entities.extend(verification);
    entities.extend(truth_outcome.map(|outcome| {
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth", "judgements": [
            { "rule": "gsd/E501", "state": "fresh", "passed": true, "outcome": outcome } ] })
    }));
    input(ship(Some("v1.0/01")), entities, phase_edge())
}

#[test]
fn an_unverified_phase_is_e511_at_ship() {
    assert_eq!(codes(e511(&phase_gate(None, None))), ["E511"]);
    let verification = json!({ "id": "v1.0/01/01-VERIFICATION", "kind": "verification",
                               "fields": { "phase": "v1.0/01" } });
    assert_eq!(
        codes(e511(&phase_gate(Some(verification.clone()), None))),
        ["E511"]
    );
    assert!(e511(&phase_gate(Some(verification), Some("verified"))).is_empty());
}

#[test]
fn a_stale_plan_check_is_w318() {
    let plan = |state: &str| {
        input(
            ship(Some("v1.0/01")),
            vec![
                json!({ "id": "v1.0/01", "kind": "phase" }),
                json!({ "id": "v1.0/01/01-01", "kind": "plan",
                        "judgements": [{ "rule": "gsd/W313", "state": state, "passed": false }] }),
            ],
            phase_edge(),
        )
    };
    assert_eq!(codes(w318(&plan("stale"))), ["W318"]);
    assert!(w318(&plan("fresh")).is_empty());
}

#[test]
fn a_tdd_plan_without_review_is_w404() {
    let plan = |checkpoint: Option<Value>| {
        let mut entities = vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "type": "tdd" } }),
            json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" }),
        ];
        entities.extend(checkpoint);
        input(ship(Some("v1.0/01")), entities, phase_edge())
    };
    let review = |answered: bool| {
        let mut fields = json!({ "kind": "tdd_review" });
        if answered {
            fields["answer"] = json!("x");
        }
        json!({ "id": "v1.0/01/01-01/C1", "kind": "checkpoint", "fields": fields })
    };
    assert_eq!(codes(w404(&plan(None))), ["W404"]);
    assert_eq!(codes(w404(&plan(Some(review(false))))), ["W404"]);
    assert!(w404(&plan(Some(review(true)))).is_empty());
}

fn two_plans(subject: &str, state: &str, summary: bool) -> PassInput {
    let plan = |id: &str, state: &str| {
        json!({ "id": id, "kind": "plan", "fields": { "type": "tdd" },
                "judgements": [{ "rule": "gsd/W313", "state": state, "passed": false }] })
    };
    input(
        ship(Some(subject)),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            plan("v1.0/01/01-01", state),
            plan("v1.0/01/01-02", "stale"),
        ]
        .into_iter()
        .chain(summary.then(|| json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" })))
        .collect(),
        vec![
            json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }),
            json!({ "source": "v1.0/01/01-02", "target": "v1.0/01", "label": "phase" }),
        ],
    )
}

#[test]
fn a_plan_subject_scopes_its_plan() {
    // Only plan 01-01 is stale here; 01-02 would be, were the gate on the phase.
    let found = w318(&two_plans("v1.0/01/01-01", "stale", false));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity.as_deref(), Some("v1.0/01/01-01"));
    assert!(w318(&two_plans("v1.0/01/01-01", "fresh", false)).is_empty());
    assert_eq!(w318(&two_plans("v1.0/01", "fresh", false)).len(), 1);
}

#[test]
fn a_summary_subject_scopes_its_plan_and_w404_names_the_summary() {
    let summary = "v1.0/01/01-01/01-01-SUMMARY";
    let found = w404(&two_plans(summary, "fresh", true));
    assert_eq!(found.len(), 1, "only 01-01 has a summary");
    assert_eq!(found[0].entity.as_deref(), Some(summary));
    // At a phase the diagnostic names the plan.
    assert_eq!(
        w404(&two_plans("v1.0/01", "fresh", true))[0]
            .entity
            .as_deref(),
        Some("v1.0/01/01-01")
    );
}

/// A plan with `status` and, beside it, a summary that says `completed` or not, holding `judgements` as the host
/// settled them.
fn owned(status: &str, completed: bool, judgements: Value) -> PassInput {
    let mut summary = json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" });
    if completed {
        summary["fields"] = json!({ "completed": "2026-10-29T10:00:00Z" });
    }
    input(
        ship(None),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "status": status },
                    "judgements": judgements }),
            summary,
        ],
        phase_edge(),
    )
}

#[test]
fn a_complete_plans_settled_away_plan_check_is_not_reported() {
    // The plan check of a complete plan is not owed, so the host settles it away: nothing to count, whatever a summary
    // says.
    for completed in [true, false] {
        let complete = owned("complete", completed, json!([]));
        assert!(w318(&complete).is_empty());
        assert!(e516(&complete).is_empty());
    }
}

#[test]
fn a_halted_plans_stale_plan_check_still_counts() {
    // A summary that says `completed` does not make an executed plan: only the plan's `status` does, and a halted plan
    // is open, so its plan check is owed and its stale one counts.
    let stale = json!([{ "rule": "gsd/W313", "state": "stale", "passed": false }]);
    for status in ["halted", "executing", "checked"] {
        let open = owned(status, true, stale.clone());
        assert_eq!(codes(w318(&open)), ["W318"], "{status}");
        assert_eq!(codes(e516(&open)), ["E516"], "{status}");
    }
}

#[test]
fn another_rules_stale_judgement_counts_for_e516_on_any_entity() {
    let stale = |rule: &str| json!([{ "rule": rule, "state": "stale", "passed": false }]);
    // A complete plan carrying another rule's stale judgement (the host keeps what is owed).
    assert_eq!(
        codes(e516(&owned("complete", true, stale("gsd/E518")))),
        ["E516"]
    );
    let truth = input(
        ship(None),
        vec![json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                     "judgements": stale("gsd/E501") })],
        vec![],
    );
    assert_eq!(codes(e516(&truth)), ["E516"]);
    // W318 is the plan checks' alone: a truth is no plan.
    assert!(w318(&truth).is_empty());
}

#[test]
fn e516_names_the_subject_of_a_scoped_gate() {
    let entities = || {
        vec![
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
            json!({ "id": "v1.0/01/01-02", "kind": "plan",
                    "judgements": [{ "rule": "gsd/W311", "state": "stale", "passed": false }] }),
        ]
    };
    let scoped = e516(&input(ship(Some("v1.0/01/01-01")), entities(), vec![]));
    assert_eq!(codes(scoped.clone()), ["E516"]);
    assert_eq!(scoped[0].entity.as_deref(), Some("v1.0/01/01-01"));
    assert!(
        scoped[0]
            .message
            .contains("the gsd/W311 verdict on 'v1.0/01/01-02' is stale")
    );
    let project = e516(&input(ship(None), entities(), vec![]));
    assert_eq!(project[0].entity.as_deref(), Some("v1.0/01/01-02"));
}

#[test]
fn a_stale_verdict_is_e516_under_plus() {
    let stale = |state: &str| {
        input(
            ship(None),
            vec![json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                         "judgements": [{ "rule": "gsd/E501", "state": state, "passed": false }] })],
            vec![],
        )
    };
    assert_eq!(codes(e516(&stale("stale"))), ["E516"]);
    assert!(e516(&stale("fresh")).is_empty());
}

/// A project every guard has something to say about at a gate on milestone v1.0: no audit (E106), an unverified
/// phase (E511), an open window (E512), a stale plan-check (W318), a tdd plan with a summary and no review (W404), a
/// complete requirement no task traces (E515) and a stale verdict (E516).
fn everything_wrong() -> (Vec<Value>, Vec<Value>) {
    let entities = vec![
        json!({ "id": "v1.0", "kind": "milestone" }),
        json!({ "id": "v1.0/01", "kind": "phase", "values": { "requirements": ["R1"] } }),
        json!({ "id": "R1", "kind": "requirement", "fields": { "status": "complete" } }),
        window("BW1", json!({})),
        json!({ "id": "v1.0/01/01-01", "kind": "plan", "fields": { "type": "tdd" },
                "values": { "requirements": ["R1"] } }),
        json!({ "id": "v1.0/01/01-01/01-01-SUMMARY", "kind": "summary" }),
        json!({ "id": "v1.0/01/01-02", "kind": "plan",
                "judgements": [{ "rule": "gsd/W313", "state": "stale", "passed": false }] }),
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth",
                "judgements": [{ "rule": "gsd/E501", "state": "stale", "passed": false }] }),
    ];
    let mut edges = phase_edge();
    edges.push(json!({ "source": "v1.0/01/01-02", "target": "v1.0/01", "label": "phase" }));
    edges.push(json!({ "source": "v1.0/01", "target": "v1.0", "label": "milestone" }));
    (entities, edges)
}

#[test]
fn every_guard_answers_a_gate_and_is_silent_on_a_write() {
    let (entities, edges) = everything_wrong();
    let gate = input(ship(Some("v1.0")), entities.clone(), edges.clone());
    // The same project and subject, but the host is applying an edit plan: not a gate.
    let write = input(
        json!({ "kind": "write", "subject": "v1.0" }),
        entities.clone(),
        edges.clone(),
    );
    let untriggered: PassInput = serde_json::from_value(crate::passes::plans::with_field_edges(
        json!({ "entities": entities, "edges": edges }),
    ))
    .expect("input");
    for (name, code, check) in GUARDS {
        let answered = codes(check(&gate));
        assert!(
            !answered.is_empty() && answered.iter().all(|c| c == code),
            "{name} ({code}) is silent at a gate over a project it should refuse: {answered:?}"
        );
        for found in [&write, &untriggered] {
            assert!(check(found).is_empty(), "{name} ({code}) answered a write");
        }
    }
}

/// A phase that lists requirement R1, its computed `status` and `proof` as the gate's graph carries them (an absent
/// `proof` when `None`).
fn traced(subject: Option<&str>, status: &str, proof: Option<&str>) -> PassInput {
    let mut fields = json!({ "status": status });
    if let Some(proof) = proof {
        fields["proof"] = json!(proof);
    }
    input(
        ship(subject),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase", "values": { "requirements": ["R1"] } }),
            json!({ "id": "R1", "kind": "requirement", "fields": fields }),
        ],
        vec![],
    )
}

#[test]
fn e515_is_silent_outside_a_gate() {
    let mut found = traced(Some("v1.0/01"), "complete", Some("no_task"));
    found.trigger = None;
    assert!(e515(&found).is_empty());
}

#[test]
fn e515_names_the_first_missing_link_of_the_proof() {
    let suggestion = "tag a passing test with the task or the requirement ([gsd:<id>]), or name it in a check of the \
        summary's coverage item for the requirement, and record the commit";
    for (proof, missing) in [
        (Some("no_task"), "no task of its plans"),
        // The derive pass writes a proof for every requirement: none reads as a thread with no task.
        (None, "no task of its plans"),
        (Some("no_commit"), "no commit of its tasks"),
        (Some("not_run"), "no passing test"),
        (Some("no_test"), "no passing test"),
        (Some("failed"), "no passing test"),
    ] {
        let refused = e515(&traced(Some("v1.0/01"), "complete", proof));
        assert_eq!(codes(refused.clone()), ["E515"], "{proof:?}");
        assert_eq!(
            refused[0].message,
            format!(
                "complete requirement 'R1' does not trace to a task, a commit and a passing test: {missing}"
            )
        );
        assert_eq!(refused[0].suggestion.as_deref(), Some(suggestion));
    }
}

#[test]
fn a_proven_complete_requirement_passes_e515() {
    assert!(e515(&traced(Some("v1.0/01"), "complete", Some("proven"))).is_empty());
    assert!(e515(&traced(None, "complete", Some("proven"))).is_empty());
}

#[test]
fn an_incomplete_requirement_is_not_e515() {
    for status in ["pending", "in_progress", "blocked", "gaps_found"] {
        assert!(e515(&traced(Some("v1.0/01"), status, Some("no_task"))).is_empty());
    }
}

#[test]
fn e515_names_the_subject() {
    let scoped = e515(&traced(Some("v1.0/01"), "complete", Some("not_run")));
    assert_eq!(scoped[0].entity.as_deref(), Some("v1.0/01"));
    let project = e515(&traced(None, "complete", Some("not_run")));
    assert_eq!(project[0].entity.as_deref(), Some("R1"));
}

/// A gate of the workflow (or command) `command`, scoped to `subject`.
fn gate_of(command: &str, subject: Option<&str>) -> Value {
    json!({ "kind": "gate", "command": command, "subject": subject })
}

fn judged(rule: &str, state: &str, changed: &[&str]) -> Value {
    json!({ "rule": rule, "state": state, "passed": false, "changed": changed })
}

/// Phase 01 (plan 01-01, whose truth TR1 holds `here`) and phase 02 (plan 02-01, whose truth TR1 holds `there`), at
/// the gate `trigger`.
fn two_phases(trigger: Value, here: Vec<Value>, there: Vec<Value>) -> PassInput {
    input(
        trigger,
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/02", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
            json!({ "id": "v1.0/02/02-01", "kind": "plan" }),
            json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth", "judgements": here }),
            json!({ "id": "v1.0/02/02-01/TR1", "kind": "truth", "judgements": there }),
        ],
        vec![
            json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }),
            json!({ "source": "v1.0/02/02-01", "target": "v1.0/02", "label": "phase" }),
        ],
    )
}

/// `(entity, message)` of each E516.
fn refused(found: Vec<PassDiagnostic>) -> Vec<(String, String)> {
    found
        .into_iter()
        .map(|d| (d.entity.unwrap_or_default(), d.message))
        .collect()
}

#[test]
fn e516_is_silent_outside_a_gate() {
    let write = json!({ "kind": "write", "command": "ship", "subject": "v1.0/01" });
    let stale = vec![judged("gsd/E501", "stale", &["cited"])];
    assert!(e516(&two_phases(write, stale.clone(), stale)).is_empty());
}

#[test]
fn e516_a_stale_verdict_anywhere_refuses_a_scoped_gate_naming_what_changed() {
    let found = e516(&two_phases(
        gate_of("ship", Some("v1.0/01")),
        vec![],
        vec![judged("gsd/E501", "stale", &["cited", "tests"])],
    ));
    assert_eq!(
        refused(found.clone()),
        [(
            "v1.0/01".to_string(),
            "the gsd/E501 verdict on 'v1.0/02/02-01/TR1' is stale: cited, tests changed"
                .to_string()
        )]
    );
    assert!(
        found[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("specforge gsd re-verify")),
        "{found:?}"
    );
}

#[test]
fn e516_a_missing_verdict_counts_only_within_the_scope() {
    let missing = || vec![judged("gsd/E501", "missing", &[])];
    let inside = e516(&two_phases(
        gate_of("ship", Some("v1.0/01")),
        missing(),
        vec![],
    ));
    assert_eq!(
        refused(inside.clone()),
        [(
            "v1.0/01".to_string(),
            "the gsd/E501 verdict on 'v1.0/01/01-01/TR1' is missing".to_string()
        )]
    );
    assert!(
        inside[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("its judge: gsd-verifier")),
        "{inside:?}"
    );
    let outside = e516(&two_phases(
        gate_of("ship", Some("v1.0/01")),
        vec![],
        missing(),
    ));
    assert!(outside.is_empty(), "{outside:?}");
    // A milestone's scope holds its phases' entities; a project gate holds every entity.
    assert_eq!(
        codes(e516(&two_phases(
            gate_of("ship", Some("v1.0")),
            vec![],
            missing()
        ))),
        ["E516"]
    );
    let project = e516(&two_phases(gate_of("ship", None), vec![], missing()));
    assert_eq!(project[0].entity.as_deref(), Some("v1.0/02/02-01/TR1"));
}

#[test]
fn e516_an_unrecorded_or_unreadable_verdict_refuses_the_gate() {
    let found = e516(&two_phases(
        gate_of("ship", Some("v1.0/01")),
        vec![judged("gsd/E501", "unrecorded", &[])],
        vec![judged("gsd/E501", "unreadable", &[])],
    ));
    let messages: Vec<String> = refused(found).into_iter().map(|(_, m)| m).collect();
    assert_eq!(
        messages,
        [
            "the gsd/E501 verdict on 'v1.0/01/01-01/TR1' is not recorded in specforge-harness",
            "the gsd/E501 verdict on 'v1.0/02/02-01/TR1' cannot be read",
        ]
    );
}

#[test]
fn e516_reports_each_rule_on_each_entity() {
    // Two rules not fresh on one entity are two errors; one rule on two entities is two.
    let two_rules = vec![
        judged("gsd/E501", "stale", &["truth"]),
        judged("gsd.security/W302", "stale", &["threat"]),
    ];
    assert_eq!(
        codes(e516(&two_phases(gate_of("ship", None), two_rules, vec![]))),
        ["E516", "E516"]
    );
    let stale = || vec![judged("gsd/E501", "stale", &["truth"])];
    assert_eq!(
        codes(e516(&two_phases(gate_of("ship", None), stale(), stale()))),
        ["E516", "E516"]
    );
}

#[test]
fn e516_a_fresh_verdict_passing_or_failing_is_no_e516() {
    let failing =
        json!({ "rule": "gsd/E501", "state": "fresh", "passed": false, "outcome": "failed" });
    let passing =
        json!({ "rule": "gsd/E501", "state": "fresh", "passed": true, "outcome": "verified" });
    assert!(
        e516(&two_phases(
            gate_of("ship", None),
            vec![failing],
            vec![passing]
        ))
        .is_empty()
    );
}

#[test]
fn e516_excuses_within_its_scope_the_verdicts_the_gated_workflow_judges() {
    // execute_phase runs gsd-verifier: its phase's missing and stale E501 verdicts are its own work.
    let own = || {
        vec![
            json!({ "id": "v1.0/01/01-01/AR1", "kind": "artifact",
                    "judgements": [judged("gsd/E501", "missing", &[])] }),
            json!({ "id": "v1.0/01/01-01/KL1", "kind": "key_link",
                    "judgements": [judged("gsd/E501", "stale", &["to"])] }),
        ]
    };
    let at = |command: &str| {
        let mut entities = vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
        ];
        entities.extend(own());
        input(gate_of(command, Some("v1.0/01")), entities, phase_edge())
    };
    assert!(e516(&at("execute_phase")).is_empty());
    // ship judges nothing: both refuse it.
    assert_eq!(codes(e516(&at("ship"))), ["E516", "E516"]);
    // Outside its scope the exception does not reach: another phase's stale verdict refuses execute_phase.
    let elsewhere = e516(&two_phases(
        gate_of("execute_phase", Some("v1.0/01")),
        vec![judged("gsd/E501", "missing", &[])],
        vec![judged("gsd/E501", "stale", &["cited"])],
    ));
    assert_eq!(
        refused(elsewhere),
        [(
            "v1.0/01".to_string(),
            "the gsd/E501 verdict on 'v1.0/02/02-01/TR1' is stale: cited changed".to_string()
        )]
    );
}

#[test]
fn e516_excuses_a_prompt_judged_rule_at_its_own_prompt() {
    let spec = |command: &str| {
        input(
            gate_of(command, Some("v1.0/02")),
            vec![
                json!({ "id": "v1.0/02", "kind": "phase" }),
                json!({ "id": "v1.0/02/02-SPEC", "kind": "phase_spec",
                        "judgements": [judged("gsd/E202", "missing", &[])] }),
            ],
            vec![],
        )
    };
    assert!(e516(&spec("spec_phase")).is_empty());
    assert_eq!(codes(e516(&spec("ship"))), ["E516"]);
}

#[test]
fn e516_never_excuses_another_extensions_rule() {
    let found = e516(&input(
        gate_of("execute_phase", Some("v1.0/01")),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
            json!({ "id": "v1.0/01/01-01/T1", "kind": "threat",
                    "judgements": [judged("gsd.security/W302", "missing", &[])] }),
        ],
        phase_edge(),
    ));
    assert_eq!(
        refused(found.clone()),
        [(
            "v1.0/01".to_string(),
            "the gsd.security/W302 verdict on 'v1.0/01/01-01/T1' is missing".to_string()
        )]
    );
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some(
            "run the step of its extension that judges it (`specforge query v1.0/01/01-01/T1 --inputs \
             gsd.security/W302` names its judge)"
        )
    );
}

/// The suggestion of a stale or unrecorded verdict names its remedy by the rule's judge: `re_verify` lists only the
/// verifier's verdicts, so it is named for those alone, with the entity's plan; a plan check is recorded again by the
/// step that runs its checker, and another extension's rule by its own step, which the inputs query names.
#[test]
fn e516_names_the_remedy_of_a_verdict_by_its_judge() {
    let entity = |id: &str, kind: &str, rule: &str, state: &str| {
        json!({ "id": id, "kind": kind, "fields": { "plan": "v1.0/01/01-01" },
                "judgements": [judged(rule, state, &["plan"])] })
    };
    let found = e516(&input(
        ship(Some("v1.0/01")),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan",
                    "judgements": [judged("gsd/W311", "stale", &["body"])] }),
            entity("v1.0/01/01-01/TR1", "truth", "gsd/E501", "stale"),
            entity("v1.0/01/01-01/PR1", "prohibition", "gsd/W525", "unrecorded"),
            entity("v1.0/01/01-01/T1", "threat", "gsd.security/W302", "stale"),
        ],
        phase_edge(),
    ));
    let how = |id: &str| {
        found
            .iter()
            .find(|d| d.message.contains(&format!("'{id}'")))
            .and_then(|d| d.suggestion.clone())
            .unwrap_or_else(|| panic!("no E516 on {id}: {found:?}"))
    };
    let verifier = "record the verdict again over the current inputs: `specforge gsd re-verify v1.0/01/01-01 \
        --stale-only` lists the verifier's verdicts that are not fresh";
    assert_eq!(how("v1.0/01/01-01/TR1"), verifier);
    assert_eq!(how("v1.0/01/01-01/PR1"), verifier);
    assert_eq!(
        how("v1.0/01/01-01"),
        "run the step that judges it again (its judge: gsd-plan-checker): it records the verdict over the current \
         inputs (`specforge query v1.0/01/01-01 --inputs gsd/W311`)"
    );
    assert_eq!(
        how("v1.0/01/01-01/T1"),
        "run the step of its extension that judges it again: it records the verdict over the current inputs \
         (`specforge query v1.0/01/01-01/T1 --inputs gsd.security/W302` names its judge)"
    );
    for id in ["v1.0/01/01-01", "v1.0/01/01-01/T1"] {
        assert!(!how(id).contains("re-verify"), "{id}");
    }
}
