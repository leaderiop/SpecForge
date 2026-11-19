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
