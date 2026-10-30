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

/// REVIEW-r43 M4 and M13: the promote line the backlog review shows is run on a backlogged project, and its jq key
/// reads the number the command answered (`.output.to`, a full id), not `.to` (null). A backlog item of another
/// milestone is listed and promoted.
#[test]
fn the_backlog_review_lists_and_promotes_across_milestones() {
    use super::cmd_phase::backlogged;
    let p = backlogged();
    p.write(
        "spec/n.spec",
        "milestone v1.1 \"N\" {\n  name \"N\"\n}\n\nphase 01 \"P01\" {\n  milestone v1.1\n  name \"P01\"\n  slug \"p01\"\n  goal \"g\"\n  success_criteria [\"a\"]\n}\n",
    );
    let text = text_of(&payload(&prompt(&p, "review_backlog", &[], "claude-code")));
    let line = |start: &str| {
        text.lines()
            .map(str::trim)
            .find(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no line starts with {start}"))
            .to_string()
    };
    let bin = std::path::PathBuf::from(std::env::var_os("SPECFORGE_BIN").expect("SPECFORGE_BIN"));
    let path = format!(
        "{}:{}",
        bin.parent().expect("a directory").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let run = |script: &str| {
        let out = std::process::Command::new("bash")
            .args(["-c", script])
            .env("PATH", &path)
            .current_dir(p.root())
            .output()
            .expect("bash");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    let listed = run(&format!("{}\necho \"$_BACKLOG\"", line("_BACKLOG=")));
    assert_eq!(
        listed, "v1.0/999.1\nv1.0/999.2",
        "the backlog of v1.0 is listed from the open milestone v1.1"
    );
    let promote = line("NEW_NUM=").replace("<id>", "v1.0/999.1");
    assert_eq!(
        run(&format!("MILESTONE=v1.1\n{promote}\necho \"$NEW_NUM\"")),
        "02"
    );
}

/// REVIEW-r43 Minor 10: the reviewer settings no lane runner reads are named inert in the review prompt.
#[test]
fn the_review_names_its_inert_reviewer_settings() {
    let p = twin();
    let text = text_of(&payload(&prompt(&p, "review", &[PHASE], "claude-code")));
    for key in [
        "review.models.*",
        "review.timeouts.*",
        "review.effort.*",
        "review.max_prompt_tokens*",
    ] {
        assert!(text.contains(key), "no {key}");
    }
    assert!(text.contains("declared but inert"));
}
