//! `undo` and `pr_branch` (T7): they run `git` in the person's repository, so the tests render and read the text.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn undo_renders() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "undo",
        &["--plan", "v1.0/01/01-01"],
        "claude-code",
    )));
    for word in [
        "--plan 01-01",
        "specforge query \"${TARGET_PLAN}\" --depth 2",
        "specforge check",
        "ask the user before reverting",
        "git revert --no-commit",
    ] {
        assert!(text.contains(word), "no '{word}'");
    }
}

#[test]
fn undo_refuses_a_missing_plan() {
    let p = twin();
    let run = prompt(&p, "undo", &["--plan", "v1.0/01/99-99"], "claude-code");
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E003"), "{}", run.all());
}

#[test]
fn undo_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(
        &p,
        "undo",
        &["--last", "2", "--phase", "v1.0/01"],
        "--last 2 --phase 01",
        "Safe git revert",
    );
}

#[test]
fn pr_branch_renders_for_three_hosts() {
    let p = twin();
    for (host, rendered) in
        render_three_hosts(&p, "pr_branch", &["--target", "main"], "--target", "PR")
    {
        let text = text_of(&rendered);
        for word in [
            "'*.spec'",
            "specforge-harness",
            "git cherry-pick --no-commit",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn pr_branch_description_names_no_planning() {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden/install/claude-code/skills/gsd-pr-branch.md"),
    )
    .expect("the skill golden");
    let description = skill
        .lines()
        .find(|l| l.starts_with("description:"))
        .expect("a description");
    assert!(!description.contains(".planning"), "{description}");
    assert!(description.contains("spec-only"), "{description}");
}
