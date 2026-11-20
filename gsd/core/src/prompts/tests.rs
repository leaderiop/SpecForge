use std::collections::BTreeSet;

use super::*;

/// The registry is every prompt `@gsd/core` declares, once: what E516 reads of a workflow is what the host serves.
#[test]
fn the_workflow_registry_is_every_declared_prompt() {
    let declared: Vec<String> = crate::build()
        .declaration()
        .surfaces
        .prompts
        .iter()
        .map(|p| p.id.clone())
        .collect();
    let registered: Vec<String> = workflows().map(|s| s.id.to_string()).collect();
    assert_eq!(registered, declared, "the registry, in declaration order");
    let unique: BTreeSet<&String> = registered.iter().collect();
    assert_eq!(
        unique.len(),
        registered.len(),
        "a workflow is registered once"
    );
}

fn judges(id: &str) -> Vec<String> {
    judges_run_by(id).into_iter().collect()
}

#[test]
fn a_workflow_runs_the_judges_of_its_agent_steps_and_itself() {
    assert_eq!(
        judges("execute_phase"),
        ["gsd-verifier", "prompt:execute_phase"]
    );
    assert_eq!(
        judges("execute_plan"),
        ["gsd-executor", "prompt:execute_plan"]
    );
    assert_eq!(judges("spec_phase"), ["prompt:spec_phase"]);
    assert_eq!(judges("ship"), ["prompt:ship"]);
    assert_eq!(judges("complete_milestone"), ["prompt:complete_milestone"]);
}

#[test]
fn a_workflow_it_continues_with_runs_behind_its_own_gate() {
    // execute_phase continues with verify_work, whose gap plans the plan checker judges: not execute_phase's judge.
    assert!(!judges_run_by("execute_phase").contains("gsd-plan-checker"));
    // mvp_phase continues with plan_phase, which runs the plan checker: mvp_phase runs none of it.
    assert_eq!(judges("mvp_phase"), ["prompt:mvp_phase"]);
    assert!(judges_run_by("plan_phase").contains("gsd-plan-checker"));
}

#[test]
fn an_id_that_is_no_workflow_runs_no_judge() {
    assert!(judges_run_by("close_milestone").is_empty());
    assert!(judges_run_by("").is_empty());
}

/// The prompts that record `judge`'s verdicts again, with the flags their run needs.
fn rejudged(judge: &str) -> Vec<(&'static str, &'static [&'static str])> {
    rejudging(judge)
        .into_iter()
        .map(|(s, r)| (s.id, r.flags))
        .collect()
}

/// The prompts that record each judge's verdicts again, in registry order, as each workflow declares them.
#[test]
fn the_prompts_that_rejudge_a_verdict_are_declared_by_each_workflow() {
    let none: &[&str] = &[];
    assert_eq!(rejudged("gsd-verifier"), [("execute_phase", none)]);
    assert_eq!(
        rejudged("gsd-plan-checker"),
        [("plan_phase", none), ("plan_review_convergence", none)]
    );
    assert_eq!(
        rejudged("gsd-integration-checker"),
        [("audit_milestone", none)]
    );
    assert_eq!(rejudged("prompt:spec_phase"), [("spec_phase", none)]);
    // plan-phase reuses a research it finds unless `--research` is given (`plan-phase.md` § 5).
    assert_eq!(
        rejudged("gsd-phase-researcher"),
        [("plan_phase", &["research"][..])]
    );
}

/// `verify_work` checks the fix plans it writes and `ultraplan_phase` the plans it imports: each runs the plan checker
/// (so the gate excuses its verdicts inside their scope) but records no check of a plan already there again.
#[test]
fn a_prompt_that_judges_only_what_it_makes_rejudges_nothing() {
    for id in ["verify_work", "ultraplan_phase"] {
        assert!(judges_run_by(id).contains("gsd-plan-checker"), "{id}");
        let spec = workflows().find(|s| s.id == id).expect("a workflow");
        assert!(spec.rejudges.is_empty(), "{id}");
    }
}

/// Every judged rule of `@gsd/core` has a prompt whose run records its verdict again on a phase or milestone. Each such
/// prompt runs the rule's judge, has a slash command (E516 names it by it) and a subject that references an entity
/// kind, and each flag it needs is one of its flags.
#[test]
fn every_judged_rule_has_a_prompt_that_rejudges_it() {
    for rule in crate::judged::JUDGED {
        let prompts = rejudging(rule.judge);
        assert!(
            !prompts.is_empty(),
            "{}: no prompt rejudges {}",
            rule.code,
            rule.judge
        );
        for (spec, rejudge) in prompts {
            assert!(
                judges_run_by(spec.id).contains(rule.judge),
                "{}: {} does not run {}",
                rule.code,
                spec.id,
                rule.judge
            );
            assert!(
                spec.slash.is_some(),
                "{}: {} has no slash command",
                rule.code,
                spec.id
            );
            // One kind: E516 climbs from the verdict's entity to the subject of that kind.
            let kind = spec.subject_kind().unwrap_or_default();
            assert!(
                !kind.is_empty() && !kind.contains('|'),
                "{}: {} subject kind {kind:?}",
                rule.code,
                spec.id
            );
            for flag in rejudge.flags {
                assert!(
                    spec.args
                        .iter()
                        .any(|a| a.name == *flag && matches!(a.kind, args::Kind::Flag)),
                    "{}: --{flag} is no flag of {}",
                    rule.code,
                    spec.id
                );
            }
        }
    }
}

/// A workflow's subject names one of its arguments, which references the entity kind the subject is.
#[test]
fn every_subject_names_an_argument_that_references_a_kind() {
    for spec in workflows().filter(|s| s.subject.is_some()) {
        let kind = spec.subject_kind().unwrap_or_else(|| panic!("{}", spec.id));
        assert!(!kind.is_empty(), "{}", spec.id);
    }
    let kind = |id: &str| {
        workflows()
            .find(|s| s.id == id)
            .and_then(|s| s.subject_kind())
    };
    assert_eq!(kind("execute_phase"), Some("phase"));
    assert_eq!(kind("plan_phase"), Some("phase"));
    assert_eq!(kind("audit_milestone"), Some("milestone"));
    assert_eq!(kind("execute_plan"), Some("plan"));
    assert_eq!(kind("quick"), None);
}

/// A declared judge is one the workflow runs: no workflow rejudges a judge it does not run, and `quick`, which has no
/// subject, rejudges none of the judges it runs on its own quick plan.
#[test]
fn a_workflow_rejudges_only_judges_it_runs() {
    for spec in workflows() {
        for rejudge in spec.rejudges {
            assert!(
                judges_run_by(spec.id).contains(rejudge.judge),
                "{}: {}",
                spec.id,
                rejudge.judge
            );
            assert!(spec.subject.is_some(), "{}", spec.id);
        }
    }
    assert!(judges_run_by("quick").contains("gsd-plan-checker"));
    assert!(rejudging("no-such-judge").is_empty());
    assert!(rejudging("prompt:secure_phase").is_empty());
}
