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

/// The milestone audit records the integration checker's `gsd/E518` verdict once, on the milestone's audit
/// `<milestone>/A1`, the entity the rule judges: the rendered prompt names it, and on a taskboard copy holding the
/// audit the record with the basis its inputs give is accepted and read back as the audit's `integration`.
#[test]
fn the_integration_verdict_is_recorded_on_the_milestones_audit() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "audit_milestone",
        &[MILESTONE],
        "claude-code",
    )));
    assert!(
        text.contains("record-verdict <milestone>/A1 E518"),
        "{text}"
    );
    assert!(!text.contains("record-verdict <phase> E518"), "{text}");
    let tb = crate::harness::taskboard();
    let run = tb.run(&[
        "new",
        "milestone_audit",
        "A1",
        "v1.0 audit",
        "--in",
        "spec/v1.0.spec",
        "--milestone",
        "v1.0",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let inputs = tb.run(&[
        "query", "v1.0/A1", "--inputs", "gsd/E518", "--format", "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let basis = inputs["basis"].to_string();
    let run = super::commands_support::gsd(
        &tb,
        "record-verdict",
        &[
            "v1.0/A1",
            "E518",
            "passed",
            "gsd-integration-checker",
            &basis,
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(tb.fields_of("v1.0/A1")["integration"], "passed");
}
