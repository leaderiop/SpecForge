//! The tail of a gsd/E516 help: the entities it names per holder and judge, owing the verdicts that refuse every run
//! the help names.

use super::*;

/// A stale verdict of the gsd verifier, as the host sends one after a cited file moved.
fn stale_truth() -> Value {
    judged("gsd/E501", "stale", &["cited"])
}

/// A stale plan check, as the host sends one after the plan's body moved.
fn stale_check(rule: &str) -> Value {
    judged(rule, "stale", &["body"])
}

/// Two stale truths of phase `v1.0/01` and a stale plan check of phase `v1.0/02`: each help names the entities that
/// owe the other phase's verdicts, by their full ids, beside the holder and the judge.
#[test]
fn the_tail_names_each_entity_owing_a_verdict_that_blocks_the_runs() {
    let found = e516(&project(
        ship(None),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR1", stale_truth()),
            ("v1.0/01/01-01/TR2", stale_truth()),
            ("v1.0/02/02-01", stale_check("gsd/W311")),
        ],
    ));
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
            + "; every run named here is also refused by the verdicts owed on v1.0/01 (gsd-verifier: \
               'v1.0/01/01-01/TR1', 'v1.0/01/01-01/TR2'), outside its scope or of a judge it does not run: record \
               those first, a gsd rule's with `specforge gsd record-verdict`, which no gate refuses"
    );
    for id in ["v1.0/01/01-01/TR1", "v1.0/01/01-01/TR2"] {
        let help = how(&found, id);
        assert!(
            help.ends_with(
                "owed on v1.0/02 (gsd-plan-checker: 'v1.0/02/02-01'), outside its scope or of a judge it does not \
                 run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no gate refuses"
            ),
            "{id}: {help}"
        );
    }
}

/// A plan owing four plan checks is one entity of one group: named once, while the gate still reports one E516 per
/// entity and rule.
#[test]
fn an_entity_owing_several_rules_of_one_judge_is_named_once() {
    let found = e516(&project(
        ship(None),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR1", stale_truth()),
            ("v1.0/02/02-01", stale_check("gsd/W311")),
            ("v1.0/02/02-01", stale_check("gsd/W313")),
            ("v1.0/02/02-01", stale_check("gsd/W315")),
            ("v1.0/02/02-01", stale_check("gsd/W316")),
        ],
    ));
    assert_eq!(found.len(), 5, "{found:?}");
    let help = how(&found, "v1.0/01/01-01/TR1");
    assert_eq!(
        help,
        verifier("v1.0/01", "v1.0/01/01-01")
            + &tail(&on(&[held(
                "v1.0/02",
                "gsd-plan-checker",
                &["v1.0/02/02-01"]
            )]))
    );
    assert_eq!(help.matches("'v1.0/02/02-01'").count(), 1, "{help}");
}

/// The entities of a group are in the order a person reads ids (a trailing number counted, not spelled), whatever the
/// order the input gives them in.
#[test]
fn the_entities_are_named_in_id_order() {
    let found = e516(&project(
        ship(None),
        &["01", "02"],
        &[
            ("v1.0/01/01-01/TR10", stale_truth()),
            ("v1.0/01/01-01/TR9", stale_truth()),
            ("v1.0/01/01-01/TR2", stale_truth()),
            ("v1.0/02/02-01", stale_check("gsd/W311")),
        ],
    ));
    assert_eq!(
        how(&found, "v1.0/02/02-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
            + &tail(&on(&[held(
                "v1.0/01",
                "gsd-verifier",
                &[
                    "v1.0/01/01-01/TR2",
                    "v1.0/01/01-01/TR9",
                    "v1.0/01/01-01/TR10"
                ]
            )]))
    );
}

/// Runs on two subjects: a group names only the entities whose verdict refuses both. A missing verdict refuses only a
/// run whose scope holds it, so it is named by no tail of runs on two phases, though its group (holder and judge)
/// refuses both runs through the stale verdict beside it.
#[test]
fn only_the_entities_whose_verdict_refuses_every_named_run_are_named() {
    assert!(!judges_run_by("plan_phase").contains(VERIFIER));
    let (spec, rejudge) = rejudging("gsd-plan-checker")
        .into_iter()
        .find(|(spec, _)| spec.id == "plan_phase")
        .expect("plan_phase records the plan checker's verdicts again");
    let remedy = Remedy {
        spec,
        rejudge,
        kind: spec.subject_kind().expect("plan_phase runs on a subject"),
    };
    let named = [(remedy, "v1.0/02"), (remedy, "v1.0/03")];
    let both = project(
        ship(None),
        &["02", "03"],
        &[
            ("v1.0/02/02-01/TR1", stale_truth()),
            ("v1.0/02/02-01/TR2", judged("gsd/E501", "missing", &[])),
        ],
    );
    assert_eq!(
        Index::new(&both).tail(&named),
        tail(&on(&[held(
            "v1.0/02",
            "gsd-verifier",
            &["v1.0/02/02-01/TR1"]
        )]))
    );
    let missing_alone = project(
        ship(None),
        &["02", "03"],
        &[("v1.0/02/02-01/TR2", judged("gsd/E501", "missing", &[]))],
    );
    let mut index = Index::new(&missing_alone);
    assert!(!index.blockers(remedy, "v1.0/02").is_empty());
    assert_eq!(index.tail(&named), "");
}

/// The ids `v1.0/<nn>/<nn>-01/TR<k>` for `k` in `ks`, each owing a stale verifier verdict.
fn stale_truths(nn: &str, ks: impl IntoIterator<Item = usize>) -> Vec<(String, Value)> {
    ks.into_iter()
        .map(|k| (format!("v1.0/{nn}/{nn}-01/TR{k}"), stale_truth()))
        .collect()
}

/// The plan check of `v1.0/02/02-01` at a gate on `v1.0/02`, beside the stale truths `owed` of other phases: its help.
fn plan_check_beside(phases: &[&str], owed: &[(String, Value)]) -> String {
    let mut all: Vec<(&str, Value)> = owed
        .iter()
        .map(|(id, j)| (id.as_str(), j.clone()))
        .collect();
    all.push(("v1.0/02/02-01", stale_check("gsd/W311")));
    how(
        &e516(&project(ship(Some("v1.0/02")), phases, &all)),
        "v1.0/02/02-01",
    )
}

/// Five entities owing one holder's verdicts to one judge are all named, with no count.
#[test]
fn five_entities_owing_one_holder_are_all_named() {
    let help = plan_check_beside(&["01", "02"], &stale_truths("01", 1..=5));
    assert!(
        help.ends_with(
            "owed on v1.0/01 (gsd-verifier: 'v1.0/01/01-01/TR1', 'v1.0/01/01-01/TR2', 'v1.0/01/01-01/TR3', \
             'v1.0/01/01-01/TR4', 'v1.0/01/01-01/TR5'), outside its scope or of a judge it does not run: record \
             those first, a gsd rule's with `specforge gsd record-verdict`, which no gate refuses"
        ),
        "{help}"
    );
    assert!(!help.contains(" more"), "{help}");
}

/// A sixth entity is counted, not named.
#[test]
fn a_sixth_entity_is_counted_not_named() {
    let help = plan_check_beside(&["01", "02"], &stale_truths("01", 1..=6));
    assert!(
        help.contains(
            "v1.0/01 (gsd-verifier: 'v1.0/01/01-01/TR1', 'v1.0/01/01-01/TR2', 'v1.0/01/01-01/TR3', \
             'v1.0/01/01-01/TR4', 'v1.0/01/01-01/TR5', and 1 more)"
        ),
        "{help}"
    );
    assert!(!help.contains("'v1.0/01/01-01/TR6'"), "{help}");
}

/// Past five, the names are the first five in `id_order` (`TR2` before `TR10`), whatever the input's order, and the
/// count is the rest.
#[test]
fn many_entities_name_the_first_five_in_id_order_and_count_the_rest() {
    let help = plan_check_beside(&["01", "02"], &stale_truths("01", (1..=12).rev()));
    assert!(
        help.contains(
            "v1.0/01 (gsd-verifier: 'v1.0/01/01-01/TR1', 'v1.0/01/01-01/TR2', 'v1.0/01/01-01/TR3', \
             'v1.0/01/01-01/TR4', 'v1.0/01/01-01/TR5', and 7 more)"
        ),
        "{help}"
    );
    for k in 6..=12 {
        assert!(
            !help.contains(&format!("'v1.0/01/01-01/TR{k}'")),
            "TR{k}: {help}"
        );
    }
}

/// Each holder-and-judge group has its own bound: two groups of six each name five and count one.
#[test]
fn the_bound_is_per_holder_and_judge() {
    let mut owed = stale_truths("01", 1..=6);
    owed.extend(stale_truths("03", 1..=6));
    assert_eq!(
        plan_check_beside(&["01", "02", "03"], &owed),
        again("gsd/W311", "gsd-plan-checker", "v1.0/02/02-01", "v1.0/02")
            + &tail(
                "on v1.0/01 (gsd-verifier: 'v1.0/01/01-01/TR1', 'v1.0/01/01-01/TR2', 'v1.0/01/01-01/TR3', \
                 'v1.0/01/01-01/TR4', 'v1.0/01/01-01/TR5', and 1 more), v1.0/03 (gsd-verifier: 'v1.0/03/03-01/TR1', \
                 'v1.0/03/03-01/TR2', 'v1.0/03/03-01/TR3', 'v1.0/03/03-01/TR4', 'v1.0/03/03-01/TR5', and 1 more)"
            )
    );
}

/// `e516` is a pure function of its input: the same help per message with the input's entities and edges reversed,
/// and on a second call.
#[test]
fn the_tail_is_the_same_whatever_the_input_order_and_on_repeat() {
    let mut owed = stale_truths("01", 1..=7);
    owed.extend(stale_truths("03", [2, 1]));
    owed.extend(stale_truths("04", [1]));
    owed.push(("v1.0/02/02-01".to_string(), stale_check("gsd/W311")));
    let owed: Vec<(&str, Value)> = owed
        .iter()
        .map(|(id, j)| (id.as_str(), j.clone()))
        .collect();
    let forward = project(ship(None), &["01", "02", "03", "04"], &owed);
    let mut reversed = project(ship(None), &["01", "02", "03", "04"], &owed);
    reversed.entities.reverse();
    reversed.edges.reverse();
    let helps = |input: &PassInput| -> BTreeMap<String, String> {
        e516(input)
            .into_iter()
            .map(|d| (d.message, d.suggestion.unwrap_or_default()))
            .collect()
    };
    let first = helps(&forward);
    assert_eq!(first.len(), 11, "{first:?}");
    let check = how(&e516(&forward), "v1.0/02/02-01");
    assert!(
        check.contains(
            "also refused by the verdicts owed on v1.0/01 (gsd-verifier: 'v1.0/01/01-01/TR1'"
        ),
        "{check}"
    );
    for (n, other) in [helps(&forward), helps(&reversed), helps(&reversed)]
        .iter()
        .enumerate()
    {
        assert_eq!(&first, other, "run {n}");
    }
}

/// An id holding a quote, a space and a non-ASCII letter is written in a tail exactly as its own message writes it.
#[test]
fn an_entity_id_is_named_in_a_tail_as_its_message_names_it() {
    let id = "v1.0/01/01-01/T'R é";
    let found = e516(&project(
        ship(None),
        &["01", "02"],
        &[
            (id, stale_truth()),
            ("v1.0/02/02-01", stale_check("gsd/W311")),
        ],
    ));
    let quoted = format!("'{id}'");
    assert!(how(&found, "v1.0/02/02-01").contains(&format!("(gsd-verifier: {quoted})")));
    assert!(
        found
            .iter()
            .any(|d| d.message
                == format!("the gsd/E501 verdict on {quoted} is stale: cited changed")),
        "{found:?}"
    );
}

/// `input` with the entities `extra` beside its own.
fn with(mut input: PassInput, extra: Vec<Value>) -> PassInput {
    input.entities.extend(
        extra
            .into_iter()
            .map(|e| serde_json::from_value(e).expect("an entity")),
    );
    input
}

/// A quick plan `260321-a1b`, judged by no one, and its truth `260321-a1b/TR1`, owing a stale verifier verdict: no
/// phase or milestone scopes either.
fn quick_truth() -> Vec<Value> {
    vec![
        json!({ "id": "260321-a1b", "kind": "plan" }),
        json!({ "id": "260321-a1b/TR1", "kind": "truth", "judgements": [stale_truth()] }),
    ]
}

/// Research `outer`, which no phase holds, and its package audits `<outer>/PA<k>` for each `k` of `ks`, each owing a
/// stale researcher verdict.
fn stale_audits(outer: &str, ks: impl IntoIterator<Item = usize>) -> Vec<Value> {
    let mut entities = vec![json!({ "id": outer, "kind": "research" })];
    entities.extend(ks.into_iter().map(|k| {
        json!({ "id": format!("{outer}/PA{k}"), "kind": "package_audit", "fields": { "research": outer },
                "judgements": [judged("gsd/W205", "stale", &["package"])] })
    }));
    entities
}

/// The E516s at a gate on `v1.0/01`, whose plan `v1.0/01/01-01` owes a stale plan check, beside the verdicts `owed` of
/// the phases `phases` and the entities `extra` no phase scopes.
fn plan_check_with(
    phases: &[&str],
    owed: &[(&str, Value)],
    extra: Vec<Value>,
) -> Vec<PassDiagnostic> {
    let mut all = owed.to_vec();
    all.push(("v1.0/01/01-01", stale_check("gsd/W311")));
    e516(&with(project(ship(Some("v1.0/01")), phases, &all), extra))
}

/// The help of a stale verdict of `code` on `entity`, judged by `judge`, which no phase or milestone scopes.
fn unheld(entity: &str, code: &str, judge: &str) -> String {
    format!(
        "no prompt judges it again where no phase or milestone scopes it: record its verdict with `specforge gsd \
         record-verdict {entity} {code} <outcome> <model> '<basis>'` over its inputs (`specforge query {entity} \
         --inputs gsd/{code}`), which no gate refuses (its judge: {judge})"
    )
}

/// No help of `found` writes one of `outers` as a holder (`on <outer> (...)`, or `<outer> (...)` after another).
fn no_outer_holder(found: &[PassDiagnostic], outers: &[&str]) {
    for d in found {
        let help = d.suggestion.as_deref().unwrap_or_default();
        for outer in outers {
            assert!(
                !help.contains(&format!("on {outer} ")) && !help.contains(&format!(" {outer} (")),
                "{outer} as a holder: {help}"
            );
        }
    }
}

/// A truth of a quick plan, stale, refuses the verifier's run on a phase from outside its scope: the help of the
/// phase's truth names it by its own id, owed by it itself, and the quick truth's own help names `record-verdict`.
#[test]
fn a_verdict_no_phase_holds_is_named_by_its_own_entity() {
    let found = e516(&with(
        project(ship(None), &["01"], &[("v1.0/01/01-01/TR1", stale_truth())]),
        quick_truth(),
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        verifier("v1.0/01", "v1.0/01/01-01")
            + "; every run named here is also refused by the verdicts owed by '260321-a1b/TR1' itself \
               (gsd-verifier), outside its scope or of a judge it does not run: record those first, a gsd rule's \
               with `specforge gsd record-verdict`, which no gate refuses"
    );
    assert_eq!(
        how(&found, "260321-a1b/TR1"),
        "no prompt judges it again where no phase or milestone scopes it: record its verdict with `specforge gsd \
         record-verdict 260321-a1b/TR1 E501 <outcome> <model> '<basis>'` over its inputs (`specforge query \
         260321-a1b/TR1 --inputs gsd/E501`), which no gate refuses (its judge: gsd-verifier)"
    );
    no_outer_holder(&found, &["260321-a1b"]);
}

/// The groups a phase or milestone holds come first, behind one `on`, then a group no holder scopes, after `, and`.
#[test]
fn held_groups_come_first_then_a_verdict_no_phase_holds() {
    let found = e516(&with(
        project(
            ship(None),
            &["01", "02"],
            &[
                ("v1.0/01/01-01/TR1", stale_truth()),
                ("v1.0/02/02-01", stale_check("gsd/W311")),
            ],
        ),
        quick_truth(),
    ));
    assert_eq!(
        how(&found, "v1.0/01/01-01/TR1"),
        verifier("v1.0/01", "v1.0/01/01-01")
            + "; every run named here is also refused by the verdicts owed on v1.0/02 (gsd-plan-checker: \
               'v1.0/02/02-01'), and by '260321-a1b/TR1' itself (gsd-verifier), outside its scope or of a judge it \
               does not run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no gate \
               refuses"
    );
    no_outer_holder(&found, &["260321-a1b"]);
}

/// Two package audits of the project's research, which no phase holds, are named by their own ids, each owing the
/// researcher itself; each audit's own help names `record-verdict` with its id.
#[test]
fn the_entities_of_one_outer_id_are_named_each_itself() {
    let found = plan_check_with(&["01"], &[], stale_audits("RESEARCH", 1..=2));
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        how(&found, "v1.0/01/01-01").ends_with(
            "owed by 'RESEARCH/PA1', 'RESEARCH/PA2', each itself (gsd-phase-researcher), outside its scope or of a \
             judge it does not run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no \
             gate refuses"
        ),
        "{found:?}"
    );
    for id in ["RESEARCH/PA1", "RESEARCH/PA2"] {
        assert_eq!(how(&found, id), unheld(id, "W205", "gsd-phase-researcher"));
    }
    no_outer_holder(&found, &["RESEARCH"]);
}

/// The groups no holder scopes follow the held ones, each after `, and`, in `id_order` of the id they fall under (a
/// digit before a letter: `251101-abc-RESEARCH` before `RESEARCH`).
#[test]
fn the_groups_no_holder_scopes_follow_the_held_ones_in_id_order() {
    let mut extra = stale_audits("RESEARCH", [2, 1]);
    extra.extend(stale_audits("251101-abc-RESEARCH", [1]));
    let found = plan_check_with(
        &["01", "02"],
        &[("v1.0/02/02-01/TR1", stale_truth())],
        extra,
    );
    assert_eq!(
        how(&found, "v1.0/01/01-01"),
        again("gsd/W311", "gsd-plan-checker", "v1.0/01/01-01", "v1.0/01")
            + "; every run named here is also refused by the verdicts owed on v1.0/02 (gsd-verifier: \
               'v1.0/02/02-01/TR1'), and by '251101-abc-RESEARCH/PA1' itself (gsd-phase-researcher), and by \
               'RESEARCH/PA1', 'RESEARCH/PA2', each itself (gsd-phase-researcher), outside its scope or of a judge \
               it does not run: record those first, a gsd rule's with `specforge gsd record-verdict`, which no gate \
               refuses"
    );
    no_outer_holder(&found, &["RESEARCH", "251101-abc-RESEARCH"]);
}

/// A group no holder scopes is bounded like a held one: six audits name five and count one.
#[test]
fn the_bound_holds_for_the_entities_of_an_outer_id() {
    let found = plan_check_with(&["01"], &[], stale_audits("RESEARCH", 1..=6));
    let help = how(&found, "v1.0/01/01-01");
    assert!(
        help.contains(
            "by 'RESEARCH/PA1', 'RESEARCH/PA2', 'RESEARCH/PA3', 'RESEARCH/PA4', 'RESEARCH/PA5', and 1 more, each \
             itself (gsd-phase-researcher)"
        ),
        "{help}"
    );
    assert!(!help.contains("'RESEARCH/PA6'"), "{help}");
    no_outer_holder(&found, &["RESEARCH"]);
}

/// What holds a verdict is an entity of a kind some remedy prompt runs on, as the registry gives them: a milestone or
/// a phase holds one, a plan, a quick plan, a research or an absent id does not, and `holder()` falls back to the
/// outermost id, which then holds nothing.
#[test]
fn a_holder_is_a_kind_a_remedy_prompt_runs_on() {
    let given = input(
        ship(None),
        vec![
            json!({ "id": "v1.0", "kind": "milestone" }),
            json!({ "id": "v1.0/01", "kind": "phase" }),
            json!({ "id": "v1.0/01/01-01", "kind": "plan" }),
            json!({ "id": "260321-a1b", "kind": "plan" }),
            json!({ "id": "RESEARCH", "kind": "research" }),
            json!({ "id": "RESEARCH/PA1", "kind": "package_audit", "fields": { "research": "RESEARCH" } }),
        ],
        vec![],
    );
    let index = Index::new(&given);
    for (id, holds) in [
        ("v1.0", true),
        ("v1.0/01", true),
        ("v1.0/01/01-01", false),
        ("260321-a1b", false),
        ("RESEARCH", false),
        ("v1.0/09", false),
    ] {
        assert_eq!(index.holds(id), holds, "{id}");
    }
    assert_eq!(index.holder("RESEARCH/PA1"), "RESEARCH");
    assert!(!index.holds(index.holder("RESEARCH/PA1")));
    assert_eq!(index.holder("v1.0/01/01-01"), "v1.0/01");
    let every_kind = input(
        ship(None),
        crate::kinds::KINDS
            .iter()
            .map(|k| json!({ "id": k.name, "kind": k.name }))
            .collect(),
        vec![],
    );
    let index = Index::new(&every_kind);
    let accepted: BTreeSet<&str> = crate::kinds::KINDS
        .iter()
        .map(|k| k.name)
        .filter(|kind| index.holds(kind))
        .collect();
    let remedied: BTreeSet<&str> = crate::judged::JUDGED
        .iter()
        .flat_map(|j| rejudging(j.judge))
        .filter_map(|(spec, _)| spec.subject_kind())
        .collect();
    assert!(
        remedied.contains("phase") && remedied.contains("milestone"),
        "{remedied:?}"
    );
    assert_eq!(accepted, remedied);
}
