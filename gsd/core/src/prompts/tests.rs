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

fn running(judge: &str) -> Vec<&'static str> {
    prompts_running(judge).into_iter().map(|s| s.id).collect()
}

/// The prompts that run a judge, in registry order: the inverse of `judges_run_by` over the workflows with a subject.
#[test]
fn the_prompts_running_a_judge_are_the_workflows_with_a_subject_whose_steps_run_it() {
    assert_eq!(running("gsd-verifier"), ["execute_phase"]);
    assert_eq!(
        running("gsd-plan-checker"),
        [
            "plan_phase",
            "plan_review_convergence",
            "ultraplan_phase",
            "verify_work"
        ]
    );
    assert_eq!(running("gsd-integration-checker"), ["audit_milestone"]);
    assert_eq!(running("prompt:spec_phase"), ["spec_phase"]);
    assert_eq!(running("gsd-phase-researcher"), ["plan_phase"]);
}

/// Every judged rule of `@gsd/core` has a prompt a user can run on a phase or milestone to give its verdict, and each
/// such prompt runs the rule's judge; a workflow without a subject is never one of them.
#[test]
fn every_judged_rule_has_a_prompt_that_runs_its_judge() {
    for rule in crate::judged::JUDGED {
        let prompts = prompts_running(rule.judge);
        assert!(
            !prompts.is_empty(),
            "{}: no prompt runs {}",
            rule.code,
            rule.judge
        );
        for spec in prompts {
            assert!(
                spec.subject.is_some(),
                "{}: {} has no subject",
                rule.code,
                spec.id
            );
            assert!(
                judges_run_by(spec.id).contains(rule.judge),
                "{}: {} does not run {}",
                rule.code,
                spec.id,
                rule.judge
            );
        }
    }
}

/// `quick` and `quick_batch` run the plan checker, the verifier and the researcher on their own quick plan: no
/// judge's prompts name them, since a user cannot remedy a phase's verdict with them.
#[test]
fn a_workflow_without_a_subject_runs_no_remedy() {
    let subjectless: Vec<&str> = workflows()
        .filter(|s| s.subject.is_none())
        .map(|s| s.id)
        .collect();
    assert!(subjectless.contains(&"quick"), "{subjectless:?}");
    let judges: BTreeSet<String> = workflows().flat_map(|s| judges_run_by(s.id)).collect();
    for judge in &judges {
        for id in running(judge) {
            assert!(!subjectless.contains(&id), "{judge}: {id}");
        }
    }
    assert!(judges_run_by("quick").contains("gsd-plan-checker"));
}

#[test]
fn no_prompt_runs_a_judge_no_workflow_has() {
    assert!(prompts_running("prompt:secure_phase").is_empty());
    assert!(prompts_running("no-such-judge").is_empty());
    assert!(prompts_running("").is_empty());
}
