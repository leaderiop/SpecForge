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
