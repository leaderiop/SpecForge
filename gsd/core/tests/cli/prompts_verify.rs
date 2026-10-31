//! `verify_work`, `validate_phase` and `add_tests` (T8).

use super::cmd_execute::stale_plan_check;
use super::commands_support::{plus, twin};
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/01";

#[test]
fn verify_work_renders() {
    let p = twin();
    let rendered = payload(&prompt(&p, "verify_work", &["v1.0/01"], "claude-code"));
    let steps: Vec<&str> = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(steps, ["uat", "plan_gaps", "check"]);
    let text = text_of(&rendered);
    assert!(text.contains("specforge gsd set-uat-result"));
    // REVIEW-r42 Minor 9: a skipped needs --reason and a blocked needs --blocked-by, or the command refuses
    assert!(text.contains("--reason"), "a skipped takes --reason");
    assert!(
        text.contains("--blocked-by"),
        "a blocked takes --blocked-by"
    );
}

#[test]
fn verify_work_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "verify_work",
        &[PHASE, "--ws", "alpha"],
        "01 --ws \"alpha\"",
        "<objective>",
    );
}

#[test]
fn validate_phase_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "validate_phase", &[PHASE], "01", "<purpose>");
}

#[test]
fn add_tests_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "add_tests",
        &[PHASE, "cover", "the", "empty", "list"],
        "01 cover the empty list",
        "<objective>",
    );
}

#[test]
fn verify_work_gate_refuses_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = prompt(&p, "verify_work", &["v1.0/02"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

#[test]
fn execute_phase_steps_into_verify_work() {
    let p = twin();
    let rendered = payload(&prompt(&p, "execute_phase", &[PHASE], "claude-code"));
    let last = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .last()
        .expect("a step")
        .clone();
    assert_eq!(last["prompt"], "specforge.gsd.verify_work");
}
