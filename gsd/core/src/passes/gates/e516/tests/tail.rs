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
