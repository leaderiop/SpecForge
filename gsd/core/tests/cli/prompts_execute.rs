//! `execute_phase` and `execute_plan` (T7): the execution loop, its gates and the plan prompt with no slash command.

use super::cmd_execute::stale_plan_check;
use super::commands_support::{plus, strict, twin};
use super::harness::project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/01";
const TWIN_PLAN: &str = "v1.0/02/02-01";
const CODES: [&str; 4] = ["W311", "W313", "W315", "W316"];

fn step_ids(rendered: &serde_json::Value) -> Vec<&str> {
    rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id"))
        .collect()
}

#[test]
fn execute_phase_gate_requires_checked_plans() {
    // Probe P2: a judged finding reaches a prompt's gate, so strict refuses a phase whose plans were not checked.
    let p = project("plan");
    strict(&p);
    let run = prompt(&p, "execute_phase", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("gate_failed"), "{}", run.all());
    assert!(run.all().contains("W311"), "{}", run.all());
    // With a fresh passing verdict on every rule of every plan, it renders.
    for plan in ["v1.0/01/01-01", "v1.0/01/01-02"] {
        for code in CODES {
            let run = p.record(plan, code, "passed", &[]);
            assert_eq!(run.code, 0, "{plan} {code}: {}", run.all());
        }
    }
    let rendered = payload(&prompt(&p, "execute_phase", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
}

#[test]
fn execute_phase_renders_under_default() {
    let p = project("plan");
    let rendered = payload(&prompt(&p, "execute_phase", &[PHASE], "claude-code"));
    assert_eq!(
        step_ids(&rendered),
        [
            "execute",
            "verify",
            "record_verdict",
            "record_window",
            "next"
        ]
    );
    assert_eq!(
        rendered["payload"]["steps"][0]["prompt"],
        "specforge.gsd.execute_plan"
    );
}

#[test]
fn execute_phase_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "execute_phase",
        &[PHASE, "--wave", "2", "--interactive"],
        "01 --wave 2 --interactive",
        "<objective>",
    );
}

#[test]
fn execute_plan_renders_with_its_plan_subject() {
    let p = twin();
    let rendered = payload(&prompt(&p, "execute_plan", &[TWIN_PLAN], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], TWIN_PLAN);
    assert_eq!(
        step_ids(&rendered),
        [
            "start",
            "execute",
            "checkpoint",
            "setup",
            "blocker",
            "complete"
        ]
    );
    let cli: Vec<&str> = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|s| s["cli"].as_str())
        .collect();
    assert_eq!(
        cli,
        [
            "specforge gsd start-plan",
            "specforge gsd answer-checkpoint",
            "specforge gsd confirm-setup",
            "specforge gsd raise-blocker",
            "specforge gsd complete-plan"
        ]
    );
    let text = text_of(&rendered);
    assert!(text.contains("<purpose>"), "the workflow is inlined");
    assert!(text.contains(&format!("`$PLAN_ID` is `{TWIN_PLAN}`")));
}

#[test]
fn execute_plan_gate_refuses_a_stale_plan_check_under_strict() {
    let p = twin();
    stale_plan_check(&p);
    strict(&p);
    let run = prompt(&p, "execute_plan", &[TWIN_PLAN], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("W318"), "{}", run.all());
}

#[test]
fn execute_plan_gate_refuses_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = prompt(&p, "execute_plan", &[TWIN_PLAN], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

#[test]
fn execute_plan_has_no_slash_command() {
    let declaration = gsd_core::build().declaration();
    assert!(
        declaration
            .surfaces
            .slash_commands
            .iter()
            .all(|s| s.prompt.as_deref() != Some("execute_plan"))
    );
    assert!(
        declaration
            .surfaces
            .slash_commands
            .iter()
            .any(|s| s.prompt.as_deref() == Some("execute_phase"))
    );
    let p = project("plan");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(p.exists(".claude/skills/gsd-execute-phase/SKILL.md"));
    assert!(!p.exists(".claude/skills/gsd-execute-plan/SKILL.md"));
}

#[test]
fn plan_phase_steps_into_execute_phase() {
    let p = project("plan");
    let rendered = payload(&prompt(&p, "plan_phase", &[PHASE], "claude-code"));
    let last = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .last()
        .expect("a step")
        .clone();
    assert_eq!(last["prompt"], "specforge.gsd.execute_phase");
}
