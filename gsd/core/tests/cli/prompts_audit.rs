//! `audit_uat` and `audit_milestone` (T9).

use super::cmd_execute::stale_plan_check;
use super::commands_support::{plus, twin};
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const MILESTONE: &str = "v1.0";

#[test]
fn audit_milestone_renders() {
    let p = twin();
    let rendered = payload(&prompt(&p, "audit_milestone", &[MILESTONE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], MILESTONE);
    let steps: Vec<&str> = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(steps, ["integrate", "record_verdict", "plan_gaps"]);
    assert_eq!(
        rendered["payload"]["steps"][2]["prompt"],
        "specforge.gsd.plan_phase"
    );
    let text = text_of(&rendered);
    assert!(text.contains("E518"), "the text names the integration rule");
    assert!(text.contains("record-verdict"));
    assert!(text.contains("`$MILESTONE` is `v1.0`"));
}

#[test]
fn audit_milestone_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "audit_milestone", &[MILESTONE], "v1.0", "<objective>");
}

#[test]
fn audit_uat_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "audit_uat", &[], "", "<objective>");
}

#[test]
fn audit_milestone_refuses_a_missing_milestone() {
    let p = twin();
    let run = prompt(&p, "audit_milestone", &["v9.9"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

#[test]
fn audit_milestone_gate_refuses_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = prompt(&p, "audit_milestone", &[MILESTONE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

/// The milestone audit tells the orchestrator to record the integration checker's `gsd/E518` verdict on each phase,
/// and the rule refuses a phase: it judges the milestone's audit, which no fence of the workflow creates.
#[test]
fn the_integration_verdict_names_a_phase_the_rule_refuses() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "audit_milestone",
        &[MILESTONE],
        "claude-code",
    )));
    assert!(text.contains("record-verdict <phase> E518"), "{text}");
    let tb = crate::harness::taskboard();
    let run = super::commands_support::gsd(
        &tb,
        "record-verdict",
        &["v1.0/01", "E518", "passed", "gsd-integration-checker", "{}"],
    );
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.stderr.contains("verdict_refused"), "{}", run.all());
    let asset = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/gsd-core/workflows/audit-milestone.md"),
    )
    .expect("the workflow");
    assert!(
        !gsd_kit::markdown::fences(&asset)
            .iter()
            .any(|f| f.language == "bash" && f.text.contains("specforge new milestone_audit")),
        "a fence creates the audit"
    );
}
