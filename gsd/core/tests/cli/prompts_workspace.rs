//! `workspace` and `update` (T10): the workspace workflows work in the shell, and `update` renders our text because
//! upstream's installs upstream GSD over this one.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of};

#[test]
fn workspace_renders() {
    let p = twin();
    for (host, rendered) in render_three_hosts(
        &p,
        "workspace",
        &["--new", "--name", "feature-x"],
        "--new --name \"feature-x\"",
        "workspace",
    ) {
        let text = text_of(&rendered);
        for word in [
            "git worktree add",
            "specforge init",
            "specforge add @gsd/core",
            "specforge install",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn workspace_renders_for_three_hosts() {
    let p = twin();
    render_three_hosts(&p, "workspace", &["--list"], "--list", "workspace");
    render_three_hosts(
        &p,
        "workspace",
        &["--remove", "--name", "x"],
        "--remove --name \"x\"",
        "workspace",
    );
}

#[test]
fn update_renders_our_text() {
    let p = twin();
    for (host, rendered) in render_three_hosts(&p, "update", &[], "(no arguments)", "@gsd/core") {
        let text = text_of(&rendered);
        for word in ["specforge update @gsd/core", "specforge install"] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
        for word in ["npm", "npx", "@opengsd"] {
            assert!(!text.contains(word), "{host}: '{word}'");
        }
        assert!(
            rendered["payload"]
                .get("attribution")
                .is_none_or(|a| a.is_null()),
            "{host}: {}",
            rendered["payload"]
        );
    }
}

#[test]
fn update_flags_say_what_they_do_here() {
    let p = twin();
    for (flag, word) in [
        ("--sync", "only step 3"),
        ("--reapply", "nothing to reapply"),
        ("--rc", "no pre-release channels"),
    ] {
        let text = text_of(&payload(&prompt(&p, "update", &[flag], "claude-code")));
        assert!(text.contains(flag), "{flag}");
        assert!(text.contains(word), "{flag}: no '{word}'");
    }
}
