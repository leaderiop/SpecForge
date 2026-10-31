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
    let agents: Vec<&str> = steps.iter().filter_map(|s| s["agent"].as_str()).collect();
    assert_eq!(
        agents,
        ["gsd-assumptions-analyzer", "gsd-advisor-researcher"]
    );
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

/// The workflows that load the phase's spec (#5629), as `specforge gsd reference` serves them.
const SPEC_READERS: [&str; 6] = [
    "discuss-phase.md",
    "discuss-phase-assumptions.md",
    "discuss-phase-power.md",
    "discuss-phase/modes/power.md",
    "spec-phase.md",
    "plan-phase.md",
];

/// The graph read that stands for upstream's `has_spec` / `spec_path` init facts.
const PHASE_SPEC_READ: &str =
    r#"jq -r '[.nodes[] | select(.kind == "phase_spec") | .id] | first // empty'"#;

fn served(p: &super::harness::Project, file: &str) -> String {
    let run = p.run(&["gsd", "reference", &format!("gsd-core/workflows/{file}")]);
    assert_eq!(run.code, 0, "{file}: {}", run.all());
    run.stdout
}

/// #5629: upstream's init emits `has_spec` / `spec_path`; SpecForge's INIT is a `specforge query`, which has
/// neither, so a verbatim copy reads empty. The phase's spec is its `phase_spec` entity, never a `*-SPEC.md` glob.
#[test]
fn the_discuss_prompts_read_the_phase_spec_from_the_graph() {
    let p = twin();
    for file in SPEC_READERS {
        let text = served(&p, file);
        for upstream in [
            "/*-SPEC.md 2>/dev/null",
            "`has_spec`, `spec_path`",
            "Init already resolved it",
            "`has_spec` and `spec_path` from init",
            "init facts `has_spec`",
            "_gsd_field \"$INIT\" spec_path",
        ] {
            assert!(!text.contains(upstream), "{file}: `{upstream}`");
        }
        assert!(text.contains(PHASE_SPEC_READ), "{file}: no graph read");
    }
}

/// The `check_spec` line of each discuss workflow, run in bash on the `discuss` project: phase 02 has a
/// `phase_spec`, phase 01 has none.
#[test]
fn check_spec_finds_the_phase_spec_in_the_graph() {
    let p = super::harness::project("discuss");
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    for file in &SPEC_READERS[..5] {
        let text = served(&p, file);
        let at = text
            .find("`SPEC_ID=$(")
            .unwrap_or_else(|| panic!("{file}: no SPEC_ID line"))
            + 1;
        let code = &text[at..at + text[at..].find('`').expect("a closing backtick")];
        for (phase, want) in [("v1.0/02", "true v1.0/02/02-SPEC"), ("v1.0/01", "false")] {
            let out = std::process::Command::new("bash")
                .args(["-c", &format!("{code}\necho $HAS_SPEC $SPEC_ID")])
                .env("PATH", &path)
                .env("PHASE_ID", phase)
                .current_dir(p.root())
                .output()
                .expect("bash");
            let got = String::from_utf8_lossy(&out.stdout);
            assert_eq!(
                got.trim(),
                want,
                "{file} {phase}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}
