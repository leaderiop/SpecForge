//! `spec_phase` and `mvp_phase` (T4): the judge renders upstream's workflow and keeps judging; the MVP prompt is the
//! first with a gate.

use serde_json::json;

use super::cmd_execute::stale_plan_check;
use super::commands_support::{plus, twin};
use super::harness::project;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/02";
const SPEC: &str = "v1.0/02/02-SPEC";

#[test]
fn spec_phase_renders() {
    let p = project("discuss");
    let rendered = payload(&prompt(&p, "spec_phase", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
    let steps: Vec<&str> = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(steps, ["record_spec", "discuss"]);
    let text = text_of(&rendered);
    // Upstream's workflow first, then r40's judging section for the phase's spec.
    assert!(text.contains("<objective>"), "{text}");
    assert!(
        text.find("<objective>") < text.find("## Judging the spec"),
        "the judging section comes last"
    );
    for needle in [
        "gsd/E202",
        "gsd/W203",
        "--inputs gsd/<code>",
        "record-verdict",
        "scores",
        SPEC,
        "ambiguity <= 20",
    ] {
        assert!(text.contains(needle), "{needle}");
    }
}

#[test]
fn spec_phase_renders_for_three_hosts() {
    let p = project("discuss");
    let out = render_three_hosts(
        &p,
        "spec_phase",
        &[PHASE, "--auto", "--text"],
        "02 --auto --text",
        "<objective>",
    );
    for (host, rendered) in out {
        assert!(text_of(&rendered).contains(SPEC), "{host}");
    }
}

#[test]
fn mvp_phase_renders_for_three_hosts() {
    let p = twin();
    let out = render_three_hosts(&p, "mvp_phase", &["v1.0/02"], "02", "<objective>");
    assert_eq!(out.len(), 3);
}

#[test]
fn spec_phase_renders_while_its_verdicts_are_stale() {
    // A judge has no gate: it must run exactly when its verdicts are stale.
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let rendered = payload(&prompt(&p, "spec_phase", &[PHASE], "claude-code"));
    assert!(text_of(&rendered).contains("Judging the spec"));
}

#[test]
fn mvp_phase_gate_refuses_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = prompt(&p, "mvp_phase", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

#[test]
fn mvp_phase_renders_under_default() {
    let p = twin();
    stale_plan_check(&p);
    p.settings(json!({}));
    let rendered = payload(&prompt(&p, "mvp_phase", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
}
