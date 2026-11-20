use serde_json::{Value, json};

use super::*;
use crate::passes::gates::tests::{gate_of, input, judged, phase_edge, refused, ship};
use crate::prompts::workflows;

/// The items naming each prompt that runs `judge` on `at`, as the suggestion writes them, derived from the registry.
fn items(judge: &str, at: &str) -> String {
    prompts_running(judge)
        .into_iter()
        .map(|s| {
            assert!(s.slash.is_some(), "{} has a slash command", s.id);
            format!(
                "/gsd-{} {at} (`specforge prompt gsd.{} {at}`)",
                s.command, s.id
            )
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// The tail naming the other holders `at`.
fn tail(at: &str) -> String {
    format!(
        "; the same judge's verdicts are also owed on {at}: each of those needs its own run, and a run is refused \
         while another holder's verdict is owed (it sits outside its scope), so record them with `specforge gsd \
         record-verdict`, which no gate refuses"
    )
}

/// The suggestion of a stale or unrecorded verifier verdict held in phase `at`, on the truths of plan `plan`.
fn verifier(at: &str, plan: &str) -> String {
    format!(
        "record the verdict again over the current inputs: run {}; `specforge gsd re-verify {plan} --stale-only` lists \
         the verifier's verdicts that are not fresh",
        items(VERIFIER, at)
    )
}

/// Milestone `v1.0` and, for each `nn` of `phases`, phase `v1.0/<nn>`, its plan `<nn>-01` and the plan's truths TR1 and
/// TR2 and prohibition PR1 (each naming the plan), at the gate `trigger`; `owed` puts judgements on entities by id.
fn project(trigger: Value, phases: &[&str], owed: &[(&str, Value)]) -> PassInput {
    let on = |id: &str| -> Vec<Value> {
        owed.iter()
            .filter(|(at, _)| *at == id)
            .map(|(_, j)| j.clone())
            .collect()
    };
    let mut entities = vec![json!({ "id": "v1.0", "kind": "milestone" })];
    let mut edges = Vec::new();
    for nn in phases {
        let phase = format!("v1.0/{nn}");
        let plan = format!("{phase}/{nn}-01");
        entities.push(json!({ "id": phase, "kind": "phase", "judgements": on(&phase) }));
        entities.push(json!({ "id": plan, "kind": "plan", "judgements": on(&plan) }));
        for (leaf, kind) in [("TR1", "truth"), ("TR2", "truth"), ("PR1", "prohibition")] {
            let id = format!("{plan}/{leaf}");
            entities.push(json!({ "id": id, "kind": kind, "fields": { "plan": plan },
                                  "judgements": on(&id) }));
        }
        edges.push(json!({ "source": plan, "target": phase, "label": "phase" }));
    }
    for (id, j) in owed {
        if !entities.iter().any(|e| e["id"] == *id) {
            entities.push(json!({ "id": id, "kind": "truth", "judgements": [j] }));
        }
    }
    input(trigger, entities, edges)
}

/// The suggestion of the E516 on `id`.
fn how(found: &[PassDiagnostic], id: &str) -> String {
    found
        .iter()
        .find(|d| d.message.contains(&format!("'{id}'")))
        .and_then(|d| d.suggestion.clone())
        .unwrap_or_else(|| panic!("no E516 on {id}: {found:?}"))
}

/// The suggestion of a stale or unrecorded verdict names its remedy by the rule's judge: the prompts that run it on the
/// verdict's phase, and `re_verify` for the verifier's verdicts alone, with the entity's plan; another extension's rule
/// is named by its own step, which the inputs query names.
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
    let exact = "record the verdict again over the current inputs: run /gsd-execute-phase v1.0/01 (`specforge \
        prompt gsd.execute_phase v1.0/01`); `specforge gsd re-verify v1.0/01/01-01 --stale-only` lists the verifier's \
        verdicts that are not fresh";
    assert_eq!(how(&found, "v1.0/01/01-01/TR1"), exact);
    assert_eq!(how(&found, "v1.0/01/01-01/PR1"), exact);
    let checker = how(&found, "v1.0/01/01-01");
    assert!(
        checker.starts_with(
            "run /gsd-plan-phase v1.0/01 (`specforge prompt gsd.plan_phase v1.0/01`) or /gsd-plan-review-convergence \
             v1.0/01 (`specforge prompt gsd.plan_review_convergence v1.0/01`) or "
        ),
        "{checker}"
    );
    assert_eq!(
        checker,
        format!(
            "run {} to judge it again (its judge: gsd-plan-checker): it records the verdict over the current inputs \
             (`specforge query v1.0/01/01-01 --inputs gsd/W311`)",
            items("gsd-plan-checker", "v1.0/01")
        )
    );
    assert_eq!(
        how(&found, "v1.0/01/01-01/T1"),
        "run the step of its extension that judges it again: it records the verdict over the current inputs \
         (`specforge query v1.0/01/01-01/T1 --inputs gsd.security/W302` names its judge)"
    );
    for id in ["v1.0/01/01-01", "v1.0/01/01-01/T1"] {
        assert!(!how(&found, id).contains("re-verify"), "{id}");
    }
}

#[test]
fn a_missing_plan_check_names_every_prompt_that_runs_the_checker() {
    let found = e516(&project(
        gate_of("execute_phase", Some("v1.0/02")),
        &["02"],
        &[("v1.0/02/02-01", judged("gsd/W311", "missing", &[]))],
    ));
    let suggestion = how(&found, "v1.0/02/02-01");
    assert!(
        suggestion.starts_with(
            "run /gsd-plan-phase v1.0/02 (`specforge prompt gsd.plan_phase v1.0/02`) or \
             /gsd-plan-review-convergence v1.0/02 (`specforge prompt gsd.plan_review_convergence v1.0/02`) or "
        ),
        "{suggestion}"
    );
    assert_eq!(
        suggestion,
        format!(
            "run {} to judge it (its judge: gsd-plan-checker)",
            items("gsd-plan-checker", "v1.0/02")
        )
    );
    for spec in prompts_running("gsd-plan-checker") {
        let cli = format!("`specforge prompt gsd.{} v1.0/02`", spec.id);
        assert!(suggestion.contains(&cli), "{cli}");
    }
}

/// The remedy runs on the phase that holds the verdict: a prompt run on the gated phase leaves a verdict held in
/// another phase in place (a verdict outside its scope is not excused).
#[test]
fn a_stale_verdict_in_another_phase_names_that_phase() {
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR1", judged("gsd/E501", "stale", &["cited"])),
            ("v1.0/01/01-01/PR1", judged("gsd/W525", "unrecorded", &[])),
            ("v1.0/01/01-01", judged("gsd/W311", "stale", &["body"])),
        ],
    ));
    assert!(
        refused(found.clone())
            .iter()
            .all(|(entity, _)| entity == "v1.0/02"),
        "{found:?}"
    );
    let exact = "record the verdict again over the current inputs: run /gsd-execute-phase v1.0/01 (`specforge \
        prompt gsd.execute_phase v1.0/01`); `specforge gsd re-verify v1.0/01/01-01 --stale-only` lists the verifier's \
        verdicts that are not fresh";
    assert_eq!(how(&found, "v1.0/01/01-01/TR1"), exact);
    assert_eq!(how(&found, "v1.0/01/01-01/PR1"), exact);
    assert_eq!(
        how(&found, "v1.0/01/01-01"),
        format!(
            "run {} to judge it again (its judge: gsd-plan-checker): it records the verdict over the current inputs \
             (`specforge query v1.0/01/01-01 --inputs gsd/W311`)",
            items("gsd-plan-checker", "v1.0/01")
        )
    );
    for d in &found {
        let suggestion = d.suggestion.as_deref().unwrap_or_default();
        assert!(!suggestion.contains("v1.0/02"), "{suggestion}");
    }
}

#[test]
fn a_milestone_audit_verdict_names_the_milestone() {
    let found = e516(&input(
        ship(None),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            json!({ "id": "v1.0/A1", "kind": "milestone_audit",
                    "judgements": [judged("gsd/E518", "stale", &["phases"])] }),
        ],
        vec![],
    ));
    assert_eq!(
        how(&found, "v1.0/A1"),
        "run /gsd-audit-milestone v1.0 (`specforge prompt gsd.audit_milestone v1.0`) to judge it again (its judge: \
         gsd-integration-checker): it records the verdict over the current inputs (`specforge query v1.0/A1 --inputs \
         gsd/E518`)"
    );
}

/// A rule a prompt judges itself is remedied by that prompt; `ship` runs no judge, so it does not excuse it.
#[test]
fn a_prompt_judged_rule_names_its_own_prompt() {
    let found = e516(&input(
        ship(Some("v1.0/02")),
        vec![
            json!({ "id": "v1.0/02", "kind": "phase" }),
            json!({ "id": "v1.0/02/02-SPEC", "kind": "phase_spec",
                    "judgements": [judged("gsd/E202", "missing", &[])] }),
        ],
        vec![],
    ));
    assert_eq!(
        how(&found, "v1.0/02/02-SPEC"),
        "run /gsd-spec-phase v1.0/02 (`specforge prompt gsd.spec_phase v1.0/02`) to judge it (its judge: \
         prompt:spec_phase)"
    );
}

#[test]
fn an_entity_with_no_phase_above_it_names_a_placeholder() {
    let found = e516(&input(
        ship(None),
        vec![json!({ "id": "X1", "kind": "truth", "judgements": [
            judged("gsd/W311", "stale", &[]),
            judged("gsd/E501", "missing", &[]),
        ] })],
        vec![],
    ));
    let suggestions: Vec<&str> = found
        .iter()
        .filter_map(|d| d.suggestion.as_deref())
        .collect();
    assert_eq!(
        suggestions,
        [
            format!(
                "run {} to judge it again (its judge: gsd-plan-checker): it records the verdict over the current \
                 inputs (`specforge query X1 --inputs gsd/W311`)",
                items("gsd-plan-checker", "<its phase>")
            ),
            format!(
                "run {} to judge it (its judge: gsd-verifier)",
                items(VERIFIER, "<its phase>")
            ),
        ]
    );
}

/// Every core judged rule, missing or stale, names a prompt in both forms, and never a workflow without a subject.
#[test]
fn every_judged_rule_names_a_prompt_when_missing_or_stale() {
    let subjectless: Vec<_> = workflows().filter(|s| s.subject.is_none()).collect();
    assert!(!subjectless.is_empty());
    for rule in crate::judged::JUDGED {
        for state in ["missing", "stale"] {
            let found = e516(&project(
                ship(Some("v1.0/01")),
                &["01"],
                &[(
                    "v1.0/01/X1",
                    judged(&format!("gsd/{}", rule.code), state, &[]),
                )],
            ));
            let suggestion = how(&found, "v1.0/01/X1");
            assert!(suggestion.contains("/gsd-"), "{}: {suggestion}", rule.code);
            assert!(
                suggestion.contains("specforge prompt gsd."),
                "{}: {suggestion}",
                rule.code
            );
            for spec in &subjectless {
                assert!(
                    !suggestion.contains(&format!("/gsd-{} ", spec.command))
                        && !suggestion.contains(&format!("gsd.{} ", spec.id)),
                    "{}: names {}: {suggestion}",
                    rule.code,
                    spec.id
                );
            }
        }
    }
}

/// A judge no prompt runs keeps the step text it is named by, and its verdicts name no holder.
#[test]
fn a_judge_no_prompt_runs_keeps_the_step_text() {
    let p = project(ship(None), &["01"], &[]);
    let kinds = kinds(&p);
    assert_eq!(remedies(&kinds, "v1.0/01/01-01", "no-such-judge"), None);
    assert_eq!(
        elsewhere(&owed(&p, &kinds), &kinds, "v1.0/01/01-01", "no-such-judge"),
        ""
    );
    let none = || {
        Some((
            "no-such-judge",
            Remedy {
                run: None,
                tail: String::new(),
            },
        ))
    };
    assert_eq!(
        first("v1.0/01/01-01", "gsd/W311", none()),
        "run the step that judges it (its judge: no-such-judge)"
    );
    assert_eq!(
        again(&p, "v1.0/01/01-01", "gsd/W311", none()),
        "run the step that judges it again (its judge: no-such-judge): it records the verdict over the current \
         inputs (`specforge query v1.0/01/01-01 --inputs gsd/W311`)"
    );
}

/// A prompt run on either of two holders is refused by the other's verdict (outside its scope): each diagnostic names
/// its own holder first, then every other holder and the gate-free route.
#[test]
fn a_verdict_owed_in_two_phases_names_both_and_the_gate_free_route() {
    let stale = || judged("gsd/E501", "stale", &["cited"]);
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02", "03"],
        &[
            ("v1.0/01/01-01/TR1", stale()),
            ("v1.0/03/03-01/TR1", stale()),
        ],
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        format!(
            "{}{}",
            verifier("v1.0/01", "v1.0/01/01-01"),
            tail("v1.0/03")
        )
    );
    assert_eq!(
        how(&found, "v1.0/03/03-01/TR1"),
        format!(
            "{}{}",
            verifier("v1.0/03", "v1.0/03/03-01"),
            tail("v1.0/01")
        )
    );
    // Three holders: the tail lists the other two, sorted.
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02", "03", "04"],
        &[
            ("v1.0/04/04-01/TR1", stale()),
            ("v1.0/03/03-01/TR1", stale()),
            ("v1.0/01/01-01/TR1", stale()),
        ],
    ));
    assert!(
        how(&found, "v1.0/01/01-01/TR1").ends_with(&tail("v1.0/03, v1.0/04")),
        "{found:?}"
    );
    assert!(
        how(&found, "v1.0/04/04-01/TR1").ends_with(&tail("v1.0/01, v1.0/03")),
        "{found:?}"
    );
}

/// A holder the gate excuses (inside its scope, judged by the gated workflow) still refuses a run of the prompt on
/// another holder, so that one's diagnostic names it.
#[test]
fn a_holder_the_gate_excuses_is_still_named_for_another_holder() {
    let stale = || judged("gsd/E501", "stale", &["cited"]);
    let found = e516(&project(
        gate_of("execute_phase", Some("v1.0/01")),
        &["01", "03"],
        &[
            ("v1.0/01/01-01/TR1", stale()),
            ("v1.0/03/03-01/TR1", stale()),
        ],
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/03/03-01/TR1"),
        format!(
            "{}{}",
            verifier("v1.0/03", "v1.0/03/03-01"),
            tail("v1.0/01")
        )
    );
}

/// A missing verdict within the gate's scope names a stale verdict of the same judge held elsewhere; a missing verdict
/// outside the scope is no diagnostic and no holder.
#[test]
fn a_missing_verdict_names_a_stale_holder_elsewhere() {
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["02", "03", "04"],
        &[
            ("v1.0/02/02-01", judged("gsd/W311", "missing", &[])),
            ("v1.0/03/03-01", judged("gsd/W311", "stale", &["body"])),
            ("v1.0/04/04-01", judged("gsd/W311", "missing", &[])),
        ],
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        format!(
            "run {} to judge it (its judge: gsd-plan-checker){}",
            items("gsd-plan-checker", "v1.0/02"),
            tail("v1.0/03")
        )
    );
    assert_eq!(
        how(&found, "v1.0/03/03-01"),
        format!(
            "run {} to judge it again (its judge: gsd-plan-checker): it records the verdict over the current inputs \
             (`specforge query v1.0/03/03-01 --inputs gsd/W311`)",
            items("gsd-plan-checker", "v1.0/03")
        )
    );
    for d in &found {
        assert!(
            !d.suggestion
                .as_deref()
                .unwrap_or_default()
                .contains("v1.0/04"),
            "{d:?}"
        );
    }
}

#[test]
fn one_phase_holding_several_verdicts_adds_no_tail() {
    let stale = || judged("gsd/E501", "stale", &["cited"]);
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR1", stale()),
            ("v1.0/01/01-01/TR2", stale()),
        ],
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    for id in ["v1.0/01/01-01/TR1", "v1.0/01/01-01/TR2"] {
        assert_eq!(
            how(&found, id),
            verifier("v1.0/01", "v1.0/01/01-01"),
            "{id}"
        );
    }
}

/// Neither an unreadable verdict's text nor another extension's rule names a prompt, and an unreadable verdict is no
/// holder: its diagnostic carries its own fix.
#[test]
fn an_unreadable_verdict_keeps_its_text_and_is_no_holder() {
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR1", judged("gsd/E501", "unreadable", &[])),
            ("v1.0/02/02-01/TR1", judged("gsd/E501", "stale", &["cited"])),
        ],
    ));
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        "fix what the inputs cannot read, then record the verdict again"
    );
    assert_eq!(
        how(&found, "v1.0/02/02-01/TR1"),
        verifier("v1.0/02", "v1.0/02/02-01")
    );
}
