//! `discuss_phase` (T3): refused for a missing phase, rendered for three hosts, its modes served by reference.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn discuss_phase_gate_refuses_a_missing_phase() {
    let p = twin();
    let before = p.read("specforge.json");
    let run = prompt(&p, "discuss_phase", &["v1.0/09"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    let all = run.all();
    // The argument is a reference, so the host refuses an unknown phase before any prompt text exists (E003).
    assert!(
        all.contains("E003") && all.contains("unresolved entity 'v1.0/09'"),
        "{all}"
    );
    assert_eq!(p.read("specforge.json"), before);
}

#[test]
fn discuss_phase_renders_for_three_hosts() {
    let p = twin();
    let out = render_three_hosts(
        &p,
        "discuss_phase",
        &[
            "v1.0/02",
            "--all",
            "--auto",
            "--chain",
            "--batch",
            "--analyze",
            "--text",
            "--power",
            "--assumptions",
        ],
        "02 --all --auto --chain --batch --analyze --text --power --assumptions",
        "<objective>",
    );
    for (host, rendered) in out {
        assert_eq!(rendered["payload"]["subject"], "v1.0/02", "{host}");
    }
}

#[test]
fn discuss_phase_names_its_modes_by_reference() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "discuss_phase",
        &["v1.0/02"],
        "claude-code",
    )));
    // The command routes in prose and reads its workflow on demand.
    let workflow = "specforge gsd reference gsd-core/workflows/discuss-phase.md";
    assert!(
        text.contains(workflow),
        "the text does not name the workflow"
    );
    let served = p.run(&["gsd", "reference", "gsd-core/workflows/discuss-phase.md"]);
    assert_eq!(served.code, 0, "{}", served.all());
    // The workflow names a mode by its path under the install root; the preamble says to read such a path with
    // `specforge gsd reference`, and the command takes it with or without `gsd-core/`.
    let power = "workflows/discuss-phase/modes/power.md";
    assert!(
        served.stdout.contains(power),
        "the workflow does not name the power mode"
    );
    assert!(
        text.contains("specforge gsd reference <path>"),
        "the preamble does not say how to read a path"
    );
    let mode = p.run(&["gsd", "reference", power]);
    assert_eq!(mode.code, 0, "{}", mode.all());
    assert!(!mode.stdout.is_empty());
}

#[test]
fn the_routing_line_reads_the_discuss_mode_setting() {
    let p = twin();
    let routing = |p: &super::harness::Project| {
        text_of(&payload(&prompt(
            p,
            "discuss_phase",
            &["v1.0/02"],
            "claude-code",
        )))
    };
    let default = routing(&p);
    assert!(
        default.contains("DISCUSS_MODE=\"discuss\""),
        "default routing"
    );
    p.settings(serde_json::json!({ "workflow.discuss_mode": "assumptions" }));
    let text = routing(&p);
    // the settings block and the line the prompt routes on say the same
    assert!(
        text.contains("\"workflow.discuss_mode\": \"assumptions\""),
        "settings block"
    );
    assert!(
        text.contains("DISCUSS_MODE=\"assumptions\""),
        "routing line"
    );
    assert!(
        !text.contains("DISCUSS_MODE=\"discuss\""),
        "stale routing line"
    );
    assert!(!text.contains("{{setting:"), "an unfilled token");
}

#[test]
fn discuss_phase_steps_spawn_core_agents_only() {
    let p = twin();
    let rendered = payload(&prompt(&p, "discuss_phase", &["v1.0/02"], "claude-code"));
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    // Two agent steps and the step into plan_phase.
    assert_eq!(steps.len(), 3);
    let text = steps
        .iter()
        .map(|s| s["title"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("gsd-assumptions-analyzer") && text.contains("gsd-advisor-researcher"));
    assert!(
        gsd_core::agents::roster::AGENTS
            .iter()
            .any(|a| a.id == "gsd-assumptions-analyzer")
    );
    assert!(
        gsd_core::agents::roster::AGENTS
            .iter()
            .any(|a| a.id == "gsd-advisor-researcher")
    );
}

#[test]
fn discuss_phase_requires_its_phase() {
    let p = twin();
    let run = prompt(&p, "discuss_phase", &[], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
}

/// `$MILESTONE` and `$PROJECT` are in the shell blocks of the workflows (REVIEW-r42 minor 2): the prompt says what they are.
#[test]
fn the_shell_ids_are_named_in_the_text() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "discuss_phase",
        &["v1.0/02"],
        "claude-code",
    )));
    assert!(
        text.contains(
            "`$PHASE_ID` is `v1.0/02`; `$MILESTONE` is `v1.0` and `$PROJECT` is `taskboard`."
        ),
        "{}",
        &text[text.len().saturating_sub(600)..]
    );
    let none = super::harness::project("empty");
    let text = text_of(&payload(&prompt(
        &none,
        "new_milestone",
        &["--name", "v1.1"],
        "claude-code",
    )));
    assert!(
        text.contains("`$PROJECT` is not in the spec yet"),
        "{}",
        &text[text.len().saturating_sub(600)..]
    );
}

/// REVIEW-r42 N6: `$MILESTONE` is never an abandoned milestone, as `specforge gsd phase` never acts on one.
#[test]
fn the_shell_ids_skip_an_abandoned_milestone() {
    let p = twin();
    p.write(
        "spec/v0.spec",
        "milestone v0.5 \"Dropped\" {\n  name \"Dropped\"\n  abandoned { reason \"dropped\" ruled_by \"tester\" at 2026-10-01 }\n}\n",
    );
    let text = text_of(&payload(&prompt(
        &p,
        "new_milestone",
        &["--name", "v1.1"],
        "claude-code",
    )));
    assert!(
        text.contains("`$MILESTONE` is `v1.0`"),
        "{}",
        &text[text.len().saturating_sub(400)..]
    );
}
