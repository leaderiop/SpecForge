//! `autonomous` (T3): an agent-followed workflow. Its unattended settings are declared and inert, and its bounds are
//! optional references that must name a phase.

use serde_json::json;

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn autonomous_renders() {
    let out = render_three_hosts(
        &twin(),
        "autonomous",
        &["--from", "01", "--to", "02", "--converge"],
        "--from 01 --to 02 --converge",
        "<purpose>",
    );
    let steps = out[0].1["payload"]["steps"].as_array().expect("steps");
    let prompts: Vec<&str> = steps.iter().filter_map(|s| s["prompt"].as_str()).collect();
    assert_eq!(
        prompts,
        [
            "specforge.gsd.discuss_phase",
            "specforge.gsd.plan_phase",
            "specforge.gsd.plan_review_convergence",
            "specforge.gsd.execute_phase",
            "specforge.gsd.verify_work",
            "specforge.gsd.audit_milestone",
            "specforge.gsd.complete_milestone",
        ]
    );
    assert_eq!(steps[7]["id"], "cleanup");
    assert_eq!(steps[7]["cli"], "specforge gsd cleanup");
}

#[test]
fn autonomous_says_its_unattended_settings_are_inert() {
    let p = twin();
    p.settings(json!({ "claude_orchestration.enabled": true }));
    let text = text_of(&payload(&prompt(&p, "autonomous", &[], "claude-code")));
    assert!(text.contains("claude_orchestration.enabled"), "{text}");
    assert!(text.contains("declared settings with no effect"), "{text}");
    assert!(
        text.contains("`specforged`) is not part of `@gsd/core`"),
        "{text}"
    );
}

/// Probe P3: an optional reference is resolved when it is given.
#[test]
fn autonomous_refuses_an_unknown_from_phase() {
    let run = prompt(&twin(), "autonomous", &["--from", "09"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
    assert!(run.all().contains("unresolved entity"), "{}", run.all());
}

#[test]
fn autonomous_names_code_review_and_the_ui_gate_as_not_installed() {
    let text = text_of(&payload(&prompt(&twin(), "autonomous", &[], "claude-code")));
    assert!(text.contains("/gsd-code-review"), "{text}");
    assert!(text.contains("ui-plan-gate"), "{text}");
    assert!(text.contains("not installed"), "{text}");
}
