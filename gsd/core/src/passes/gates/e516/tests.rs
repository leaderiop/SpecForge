use serde_json::{Value, json};

use super::*;
use crate::passes::gates::tests::{gate_of, input, judged, phase_edge, refused, ship};
use crate::prompts::{rejudging, workflows};

/// The items naming each prompt whose run records `judge`'s verdict again on `at`, with its flags, as the suggestion
/// writes them, derived from the registry.
fn items(judge: &str, at: &str) -> String {
    rejudging(judge)
        .into_iter()
        .map(|(s, r)| {
            let run = std::iter::once(at.to_string())
                .chain(r.flags.iter().map(|f| format!("--{f}")))
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "/gsd-{} {run} (`specforge prompt gsd.{} {run}`)",
                s.command, s.id
            )
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// The tail naming the verdicts that refuse every named run, `owed` as [`on`] writes its groups.
fn tail(owed: &str) -> String {
    format!(
        "; every run named here is also refused by the verdicts owed {owed}, outside its scope or of a judge it does \
         not run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no gate refuses"
    )
}

/// The groups of a tail held by a phase or milestone, each written by [`held`]: `on <group>, <group>`.
fn on(groups: &[String]) -> String {
    format!("on {}", groups.join(", "))
}

/// One group of a tail: its `holder`, its judge `by` and the `entities` owing it, quoted, in the order given.
fn held(holder: &str, by: &str, entities: &[&str]) -> String {
    let quoted: Vec<String> = entities.iter().map(|e| format!("'{e}'")).collect();
    format!("{holder} ({by}: {})", quoted.join(", "))
}

/// One group of a tail no phase or milestone holds: its `entities`, quoted, each owing its judge `by` itself.
fn itself(by: &str, entities: &[&str]) -> String {
    let quoted: Vec<String> = entities.iter().map(|e| format!("'{e}'")).collect();
    let each = if entities.len() == 1 { "" } else { ", each" };
    format!("by {}{each} itself ({by})", quoted.join(", "))
}

/// The suggestion of a stale or unrecorded verdict of another core judge than the verifier on `entity`, held on `at`.
fn again(rule: &str, judge: &str, entity: &str, at: &str) -> String {
    format!(
        "run {} to judge it again (its judge: {judge}): it records the verdict over the current inputs (`specforge \
         query {entity} --inputs {rule}`)",
        items(judge, at)
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

/// The suggestion of a stale or unrecorded verdict names its remedy by the rule's judge: the prompts that record it
/// again on the verdict's phase, and `re_verify` for the verifier's verdicts alone, with the entity's plan; another
/// extension's rule is named by its own step, which the inputs query names. Each also names the verdicts of the other
/// judges and extensions in the phase, which refuse its runs.
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
    let exact = format!(
        "record the verdict again over the current inputs: run /gsd-execute-phase v1.0/01 (`specforge prompt \
         gsd.execute_phase v1.0/01`); `specforge gsd re-verify v1.0/01/01-01 --stale-only` lists the verifier's \
         verdicts that are not fresh{}",
        tail(&on(&[
            held("v1.0/01", "gsd-plan-checker", &["v1.0/01/01-01"]),
            held("v1.0/01", "gsd.security/W302", &["v1.0/01/01-01/T1"])
        ]))
    );
    assert_eq!(how(&found, "v1.0/01/01-01/TR1"), exact);
    assert_eq!(how(&found, "v1.0/01/01-01/PR1"), exact);
    let checker = how(&found, "v1.0/01/01-01");
    assert!(
        checker.starts_with(
            "run /gsd-plan-phase v1.0/01 (`specforge prompt gsd.plan_phase v1.0/01`) or /gsd-plan-review-convergence \
             v1.0/01 (`specforge prompt gsd.plan_review_convergence v1.0/01`) to judge it again"
        ),
        "{checker}"
    );
    assert_eq!(
        checker,
        again("gsd/W311", "gsd-plan-checker", "v1.0/01/01-01", "v1.0/01")
            + &tail(&on(&[
                held(
                    "v1.0/01",
                    "gsd-verifier",
                    &["v1.0/01/01-01/PR1", "v1.0/01/01-01/TR1"]
                ),
                held("v1.0/01", "gsd.security/W302", &["v1.0/01/01-01/T1"])
            ]))
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

/// A missing plan check names the prompts that check the phase's plans again; `verify_work` and `ultraplan_phase` run
/// the plan checker only over the plans they make, so neither is named.
#[test]
fn a_missing_plan_check_names_every_prompt_that_checks_it_again() {
    let found = e516(&project(
        gate_of("execute_phase", Some("v1.0/02")),
        &["02"],
        &[("v1.0/02/02-01", judged("gsd/W311", "missing", &[]))],
    ));
    let suggestion = how(&found, "v1.0/02/02-01");
    assert_eq!(
        suggestion,
        "run /gsd-plan-phase v1.0/02 (`specforge prompt gsd.plan_phase v1.0/02`) or /gsd-plan-review-convergence \
         v1.0/02 (`specforge prompt gsd.plan_review_convergence v1.0/02`) to judge it (its judge: gsd-plan-checker)"
    );
    assert_eq!(
        suggestion,
        format!(
            "run {} to judge it (its judge: gsd-plan-checker)",
            items("gsd-plan-checker", "v1.0/02")
        )
    );
    for prompt in ["verify-work", "verify_work", "ultraplan"] {
        assert!(!suggestion.contains(prompt), "{prompt}: {suggestion}");
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
    let exact = verifier("v1.0/01", "v1.0/01/01-01")
        + &tail(&on(&[held(
            "v1.0/01",
            "gsd-plan-checker",
            &["v1.0/01/01-01"],
        )]));
    assert_eq!(how(&found, "v1.0/01/01-01/TR1"), exact);
    assert_eq!(how(&found, "v1.0/01/01-01/PR1"), exact);
    assert_eq!(
        how(&found, "v1.0/01/01-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/01/01-01", "v1.0/01")
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &["v1.0/01/01-01/PR1", "v1.0/01/01-01/TR1"]
            )]))
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

/// A quick plan has no phase or milestone above it: no prompt's run covers it, so its verdict names the gate-free
/// `record-verdict`, never a placeholder, and it refuses every run another verdict names (it sits outside each scope),
/// which names it by its own id, never as a holder.
#[test]
fn an_entity_no_phase_scopes_names_the_gate_free_route_and_refuses_the_named_runs() {
    let found = e516(&input(
        ship(None),
        vec![
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
            json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth", "fields": { "plan": "v1.0/01/01-01" },
                    "judgements": [judged("gsd/E501", "stale", &["cited"])] }),
            json!({ "id": "260321-a1b", "kind": "plan",
                    "judgements": [judged("gsd/W311", "stale", &["body"])] }),
            json!({ "id": "260321-a1b/TR1", "kind": "truth",
                    "judgements": [judged("gsd/E501", "missing", &[])] }),
        ],
        phase_edge(),
    ));
    assert_eq!(
        how(&found, "260321-a1b"),
        "no prompt judges it again where no phase or milestone scopes it: record its verdict with `specforge gsd \
         record-verdict 260321-a1b W311 <outcome> <model> '<basis>'` over its inputs (`specforge query 260321-a1b \
         --inputs gsd/W311`), which no gate refuses (its judge: gsd-plan-checker)"
    );
    assert_eq!(
        how(&found, "260321-a1b/TR1"),
        "no prompt judges it where no phase or milestone scopes it: record its verdict with `specforge gsd \
         record-verdict 260321-a1b/TR1 E501 <outcome> <model> '<basis>'` over its inputs (`specforge query \
         260321-a1b/TR1 --inputs gsd/E501`), which no gate refuses (its judge: gsd-verifier)"
    );
    // The missing verdict refuses only a gate that covers it: no run on a phase does.
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        verifier("v1.0/01", "v1.0/01/01-01") + &tail(&itself("gsd-plan-checker", &["260321-a1b"]))
    );
    for d in &found {
        assert!(
            !d.suggestion
                .as_deref()
                .unwrap_or_default()
                .contains("<its "),
            "{d:?}"
        );
    }
}

/// Every core judged rule, missing or stale, names a prompt in both forms, and never a workflow without a subject nor
/// one that does not record the verdict again.
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
            for spec in workflows().filter(|s| s.rejudges.iter().all(|r| r.judge != rule.judge)) {
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
        verifier("v1.0/01", "v1.0/01/01-01")
            + &tail(&on(&[held(
                "v1.0/03",
                "gsd-verifier",
                &["v1.0/03/03-01/TR1"]
            )]))
    );
    assert_eq!(
        how(&found, "v1.0/03/03-01/TR1"),
        verifier("v1.0/03", "v1.0/03/03-01")
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &["v1.0/01/01-01/TR1"]
            )]))
    );
    // Three holders: the tail lists the other two, in natural order.
    let found = e516(&project(
        ship(Some("v1.0/02")),
        &["01", "02", "9", "10"],
        &[
            ("v1.0/10/10-01/TR1", stale()),
            ("v1.0/9/9-01/TR1", stale()),
            ("v1.0/01/01-01/TR1", stale()),
        ],
    ));
    assert!(
        how(&found, "v1.0/01/01-01/TR1").ends_with(&tail(&on(&[
            held("v1.0/9", "gsd-verifier", &["v1.0/9/9-01/TR1"]),
            held("v1.0/10", "gsd-verifier", &["v1.0/10/10-01/TR1"])
        ]))),
        "{found:?}"
    );
    assert!(
        how(&found, "v1.0/10/10-01/TR1").ends_with(&tail(&on(&[
            held("v1.0/01", "gsd-verifier", &["v1.0/01/01-01/TR1"]),
            held("v1.0/9", "gsd-verifier", &["v1.0/9/9-01/TR1"])
        ]))),
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
        verifier("v1.0/03", "v1.0/03/03-01")
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &["v1.0/01/01-01/TR1"]
            )]))
    );
}

/// A missing verdict within the gate's scope names a stale verdict of the same judge held elsewhere; a missing verdict
/// outside a run's scope does not refuse it, so it is named by no tail.
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
            tail(&on(&[held(
                "v1.0/03",
                "gsd-plan-checker",
                &["v1.0/03/03-01"]
            )]))
        )
    );
    assert_eq!(
        how(&found, "v1.0/03/03-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/03/03-01", "v1.0/03")
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

/// An unreadable verdict keeps its own text (what to fix), and it refuses every run another verdict names, like any
/// verdict on record outside a run's scope; another extension's rule names no prompt.
#[test]
fn an_unreadable_verdict_keeps_its_text_and_refuses_the_named_runs() {
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
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &["v1.0/01/01-01/TR1"]
            )]))
    );
}

/// Phase `v1.0/01` (complete when `closed`) holding a stale verifier verdict on its plan's truth and a stale package
/// legitimacy verdict on its research, and phase `v1.0/02` holding a stale plan check on its open plan, at a project
/// gate that runs no judge.
fn cross_judges(closed: bool, researched: bool) -> PassInput {
    let status = if closed { "complete" } else { "executed" };
    let mut entities = vec![
        json!({ "id": "v1.0", "kind": "milestone" }),
        json!({ "id": "v1.0/01", "kind": "phase", "fields": { "status": status } }),
        json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
        json!({ "id": "v1.0/01/01-01/TR1", "kind": "truth", "fields": { "plan": "v1.0/01/01-01" },
                "judgements": [judged("gsd/E501", "stale", &["cited"])] }),
        json!({ "id": "v1.0/02", "kind": "phase", "fields": { "status": "planned" } }),
        json!({ "id": "v1.0/02/02-01", "kind": "plan",
                "judgements": [judged("gsd/W311", "stale", &["body"])] }),
    ];
    if researched {
        entities.push(json!({ "id": "v1.0/01/01-RESEARCH", "kind": "research" }));
        entities.push(
            json!({ "id": "v1.0/01/01-RESEARCH/PA1", "kind": "package_audit",
                              "judgements": [judged("gsd/W205", "stale", &["audit"])] }),
        );
    }
    input(
        ship(None),
        entities,
        vec![
            json!({ "source": "v1.0/01/01-01", "target": "v1.0/01", "label": "phase" }),
            json!({ "source": "v1.0/02/02-01", "target": "v1.0/02", "label": "phase" }),
        ],
    )
}

/// A stale verifier verdict in a closed phase and a stale plan check of an open plan in another: `execute_phase` on
/// the first is refused by the plan check and `plan_phase` on the second by the verifier verdict, so each diagnostic
/// names the other's verdict and the gate-free route.
#[test]
fn verdicts_of_two_judges_in_two_phases_name_each_other_and_the_gate_free_route() {
    let found = e516(&cross_judges(true, false));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        verifier("v1.0/01", "v1.0/01/01-01")
            + &tail(&on(&[held(
                "v1.0/02",
                "gsd-plan-checker",
                &["v1.0/02/02-01"]
            )]))
    );
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &["v1.0/01/01-01/TR1"]
            )]))
    );
}

/// A stale researcher verdict beside a stale verifier verdict in one open phase: the prompt of each does not run the
/// other's judge, so each names the other, with `--research` for the research that `plan_phase` would reuse.
#[test]
fn verdicts_of_two_judges_in_one_phase_name_each_other() {
    let found = e516(&cross_judges(false, true));
    assert_eq!(
        how(&found, "v1.0/01/01-RESEARCH/PA1"),
        "run /gsd-plan-phase v1.0/01 --research (`specforge prompt gsd.plan_phase v1.0/01 --research`) to judge it \
         again (its judge: gsd-phase-researcher): it records the verdict over the current inputs (`specforge query \
         v1.0/01/01-RESEARCH/PA1 --inputs gsd/W205`)"
            .to_string()
            + &tail(&on(&[held("v1.0/01", "gsd-verifier", &["v1.0/01/01-01/TR1"]), held("v1.0/02", "gsd-plan-checker", &["v1.0/02/02-01"])]))
    );
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        verifier("v1.0/01", "v1.0/01/01-01")
            + &tail(&on(&[
                held(
                    "v1.0/01",
                    "gsd-phase-researcher",
                    &["v1.0/01/01-RESEARCH/PA1"]
                ),
                held("v1.0/02", "gsd-plan-checker", &["v1.0/02/02-01"])
            ]))
    );
}

/// `plan_phase` stops on a complete phase, so a stale researcher verdict there names the gate-free route; it still
/// refuses the runs other verdicts name.
#[test]
fn a_researcher_verdict_in_a_complete_phase_names_the_gate_free_route() {
    let found = e516(&cross_judges(true, true));
    assert_eq!(
        how(&found, "v1.0/01/01-RESEARCH/PA1"),
        "no prompt judges it again on the complete phase v1.0/01, where /gsd-plan-phase stops: record its verdict \
         with `specforge gsd record-verdict v1.0/01/01-RESEARCH/PA1 W205 <outcome> <model> '<basis>'` over its \
         inputs (`specforge query v1.0/01/01-RESEARCH/PA1 --inputs gsd/W205`), which no gate refuses (its judge: \
         gsd-phase-researcher)"
    );
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
            + &tail(&on(&[
                held(
                    "v1.0/01",
                    "gsd-phase-researcher",
                    &["v1.0/01/01-RESEARCH/PA1"]
                ),
                held("v1.0/01", "gsd-verifier", &["v1.0/01/01-01/TR1"])
            ]))
    );
}

/// A stale plan check beside a stale researcher verdict in one open phase: `plan_phase` runs both judges, so the plan
/// check's refusal lists no verdict, though `plan_review_convergence`, also named, does not run the researcher. A
/// verdict enters the list only when it refuses every run named.
#[test]
fn a_verdict_that_refuses_only_some_named_runs_is_not_listed() {
    let found = e516(&input(
        ship(None),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            json!({ "id": "v1.0/02", "kind": "phase", "fields": { "status": "planned" } }),
            json!({ "id": "v1.0/02/02-01", "kind": "plan",
                    "judgements": [judged("gsd/W311", "stale", &["body"])] }),
            json!({ "id": "v1.0/02/02-RESEARCH", "kind": "research" }),
            json!({ "id": "v1.0/02/02-RESEARCH/PA1", "kind": "package_audit",
                    "judgements": [judged("gsd/W205", "stale", &["audit"])] }),
        ],
        vec![json!({ "source": "v1.0/02/02-01", "target": "v1.0/02", "label": "phase" })],
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
    );
    assert!(
        !how(&found, "v1.0/02/02-RESEARCH/PA1").contains("also refused"),
        "{found:?}"
    );
}

mod tail;
