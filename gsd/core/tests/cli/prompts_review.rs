//! `review` and `review_backlog` (T5): the reviewers are shell commands and the reviews are shown; a backlog phase is
//! promoted or removed through `phase`.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

const PHASE: &str = "v1.0/02";

#[test]
fn review_renders() {
    let p = twin();
    for (host, rendered) in render_three_hosts(&p, "review", &[PHASE, "--codex"], "02", "Cross-AI")
    {
        assert_eq!(rendered["payload"]["subject"], PHASE, "{host}");
        assert!(rendered["payload"]["steps"].is_null(), "{host}: a step");
        let text = text_of(&rendered);
        for word in ["--codex", "coderabbit", "cursor-agent", "not stored"] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
        assert!(text.contains("specforge query \"$PHASE_ID\""), "{host}");
    }
}

#[test]
fn review_refuses_a_missing_phase() {
    let p = twin();
    let run = prompt(&p, "review", &["v1.0/99"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

#[test]
fn review_backlog_renders_for_three_hosts() {
    let p = twin();
    for (host, rendered) in render_three_hosts(&p, "review_backlog", &[], "Backlog", "Backlog") {
        let text = text_of(&rendered);
        for word in [
            "specforge gsd phase --promote",
            "specforge gsd phase --remove",
            "--kinds phase",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn review_backlog_step_is_the_phase_command() {
    let p = twin();
    let rendered = payload(&prompt(&p, "review_backlog", &[], "claude-code"));
    assert_eq!(rendered["payload"]["steps"][0]["id"], "phase");
    assert_eq!(
        rendered["payload"]["steps"][0]["cli"],
        "specforge gsd phase"
    );
}
