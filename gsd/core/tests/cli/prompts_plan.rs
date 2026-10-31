//! `plan_phase` (T5): the largest workflow, its steps and its entry gate.

use serde_json::json;

use super::cmd_execute::stale_plan_check;
use super::commands_support::{plus, twin};
use super::harness::{Project, project};
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/02";

fn declared_gates(id: &str) -> Vec<String> {
    let declaration = gsd_core::build().declaration();
    let prompt = declaration
        .surfaces
        .prompts
        .iter()
        .find(|p| p.id == id)
        .expect("the prompt");
    prompt.gates.clone()
}

#[test]
fn a_steps_agent_is_its_agent_field() {
    let p = project("discuss");
    let rendered = payload(&prompt(&p, "plan_phase", &[PHASE], "claude-code"));
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    assert!(
        steps.iter().any(|s| s["agent"] == "gsd-planner"),
        "{steps:?}"
    );
    let titled: Vec<&str> = steps.iter().filter_map(|s| s["title"].as_str()).collect();
    assert!(titled.iter().all(|t| !t.contains("(gsd-")), "{titled:?}");
}

#[test]
fn plan_phase_steps_and_gates() {
    let p = project("discuss");
    let rendered = payload(&prompt(&p, "plan_phase", &[PHASE], "claude-code"));
    let steps: Vec<&str> = rendered["payload"]["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|s| s["id"].as_str().expect("an id"))
        .collect();
    assert_eq!(
        steps,
        [
            "research",
            "record_research",
            "patterns",
            "plan",
            "check",
            "record_verdict",
            "execute"
        ]
    );
    assert_eq!(
        declared_gates("plan_phase"),
        ["E201", "W204", "E206", "W406", "E516"]
    );
    assert_eq!(rendered["payload"]["subject"], PHASE);
    // The judges are agents: the payload lists no judged rule.
    assert!(rendered["payload"].get("judged").is_none());
}

#[test]
fn plan_phase_renders_on_the_twin() {
    let p = project("discuss");
    let text = text_of(&payload(&prompt(&p, "plan_phase", &[PHASE], "claude-code")));
    assert!(text.contains("<objective>"));
}

fn fixture(file: &str) -> String {
    let path = format!(
        "{}/tests/projects/discuss/spec/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn without_refs() -> Project {
    let p = project("discuss");
    let spec = fixture("discuss.spec");
    let field = "  canonical_refs [\n    { topic \"Storage\" paths [\"docs/storage.md\"] }\n  ]\n";
    assert!(spec.contains(field), "the fixture holds the refs");
    p.write("spec/discuss.spec", &spec.replace(field, ""));
    p
}

fn with_open_question() -> Project {
    let p = project("discuss");
    let spec = fixture("research.spec");
    let resolved = "  resolved_by D-02\n";
    assert!(spec.contains(resolved), "the fixture resolves OQ1");
    p.write("spec/research.spec", &spec.replace(resolved, ""));
    p
}

#[test]
fn plan_phase_gate_refuses_a_context_without_canonical_refs() {
    let p = without_refs();
    let run = prompt(&p, "plan_phase", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E201"), "{}", run.all());
}

#[test]
fn plan_phase_gate_refuses_an_open_research_question_under_strict() {
    let p = with_open_question();
    // Default preset: W204 is a warning, so the gate lets the prompt through.
    let rendered = payload(&prompt(&p, "plan_phase", &[PHASE], "claude-code"));
    assert_eq!(rendered["payload"]["subject"], PHASE);
    p.settings(json!({ "profile": "gsd-strict" }));
    let run = prompt(&p, "plan_phase", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("W204"), "{}", run.all());
}

#[test]
fn plan_phase_renders_for_three_hosts() {
    let p = project("discuss");
    let args = [
        PHASE,
        "--auto",
        "--research",
        "--skip-research",
        "--research-phase",
        "3",
        "--view",
        "--gaps",
        "--skip-verify",
        "--skip-ui",
        "--prd",
        "docs/prd.md",
        "--ingest",
        "docs/*.md",
        "--ingest-format",
        "madr",
        "--reviews",
        "--text",
        "--bounce",
        "--skip-bounce",
        "--chunked",
        "--tdd",
        "--mvp",
        "--granularity",
        "fine",
        "--no-tracer",
        "--no-reversibility-gates",
    ];
    let out = render_three_hosts(
        &p,
        "plan_phase",
        &args,
        "02 --auto --research --skip-research --research-phase \"3\" --view --gaps --skip-verify --skip-ui --prd \"docs/prd.md\" --ingest \"docs/*.md\" --ingest-format madr --reviews --text --bounce --skip-bounce --chunked --tdd --mvp --granularity fine --no-tracer --no-reversibility-gates",
        "<objective>",
    );
    for (host, rendered) in out {
        let text = text_of(&rendered);
        // Probe P6: the ~110 KB answer arrives whole.
        assert!(text.len() > 100_000, "{host}: {} bytes", text.len());
        assert!(text.contains("</process>"), "{host}: the text is cut");
    }
}

#[test]
fn mvp_phase_steps_into_plan_phase() {
    let p = project("discuss");
    let rendered = payload(&prompt(&p, "mvp_phase", &[PHASE], "claude-code"));
    let step = &rendered["payload"]["steps"][0];
    assert_eq!(step["prompt"], "specforge.gsd.plan_phase");
    let discuss = payload(&prompt(&p, "discuss_phase", &[PHASE], "claude-code"));
    assert!(
        discuss["payload"]["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|s| s["prompt"] == "specforge.gsd.plan_phase")
    );
}

#[test]
fn the_plan_phase_slash_command_has_effort_max() {
    // The declaration carries `max`. The claude-code skill writer does not write a skill's effort today (a host
    // gap, not this extension's), so the installed frontmatter is not asserted.
    let declaration = gsd_core::build().declaration();
    let slash = declaration
        .surfaces
        .slash_commands
        .iter()
        .find(|s| s.prompt.as_deref() == Some("plan_phase"))
        .expect("the slash command");
    assert_eq!(slash.effort.as_deref(), Some("max"));
    let p = project("discuss");
    let run = p.install("claude-code", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(p.exists(".claude/skills/gsd-plan-phase/SKILL.md"));
}

const CODEX_CYCLES: &[&str] = &[PHASE, "--codex", "--max-cycles", "3"];

#[test]
fn convergence_renders() {
    let p = twin();
    let rendered = payload(&prompt(
        &p,
        "plan_review_convergence",
        CODEX_CYCLES,
        "claude-code",
    ));
    let steps = rendered["payload"]["steps"].as_array().expect("steps");
    assert_eq!(steps[0]["id"], "replan");
    assert_eq!(steps[0]["prompt"], "specforge.gsd.plan_phase");
    assert_eq!(steps[1]["id"], "check");
    assert_eq!(steps[1]["agent"], "gsd-plan-checker");
    assert!(text_of(&rendered).contains("02 --codex --max-cycles 3"));
}

#[test]
fn convergence_renders_for_three_hosts() {
    let p = twin();
    let args = [
        PHASE,
        "--claude",
        "--codex",
        "--coderabbit",
        "--opencode",
        "--qwen",
        "--cursor",
        "--antigravity",
        "--agy",
        "--ollama",
        "--lm-studio",
        "--llama-cpp",
        "--kimi-code",
        "--text",
        "--ws",
        "alpha",
        "--all",
        "--max-cycles",
        "3",
    ];
    render_three_hosts(
        &p,
        "plan_review_convergence",
        &args,
        "02 --claude --codex --coderabbit --opencode --qwen --cursor --antigravity --agy --ollama --lm-studio --llama-cpp --kimi-code --text --ws \"alpha\" --all --max-cycles 3",
        "<objective>",
    );
}

#[test]
fn ultraplan_phase_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "ultraplan_phase", &[PHASE], "02", "<objective>");
}

#[test]
fn ultraplan_phase_gate_refuses_a_context_without_canonical_refs() {
    let p = without_refs();
    let run = prompt(&p, "ultraplan_phase", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E201"), "{}", run.all());
}

#[test]
fn convergence_gate_refuses_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = prompt(&p, "plan_review_convergence", &[PHASE], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E516"), "{}", run.all());
}

#[test]
fn plan_phase_with_no_phase_plans_the_next() {
    let p = twin();
    let none = payload(&prompt(&p, "plan_phase", &[], "claude-code"));
    let named = payload(&prompt(&p, "plan_phase", &["02.1"], "claude-code"));
    assert_eq!(none["payload"]["subject"], "v1.0/02.1");
    assert_eq!(text_of(&none), text_of(&named));
    assert!(text_of(&none).contains("`$PHASE_ID` is `v1.0/02.1`"));

    // Once 02.1 is planned, the next one is 03.
    let plans = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{plans}\nplan 02.1-01 \"Fix the title\" {{\n  phase v1.0/02.1\n  type execute\n  objective \"Fix it.\"\n}}\n"),
    );
    let next = payload(&prompt(&p, "plan_phase", &[], "claude-code"));
    assert_eq!(next["payload"]["subject"], "v1.0/03");

    // Every phase planned: upstream's ask, and no subject.
    let plans = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{plans}\nplan 03-01 \"List\" {{\n  phase v1.0/03\n  type execute\n  objective \"List.\"\n}}\nplan 01-09 \"x\" {{\n  phase v1.0/01\n  type execute\n  objective \"x.\"\n}}\n"),
    );
    let ask = payload(&prompt(&p, "plan_phase", &[], "claude-code"));
    assert!(
        text_of(&ask)
            .contains("No unplanned phase in this milestone: ask the user which phase to plan.")
    );
    assert!(ask.get("subject").is_none(), "{ask}");
}
