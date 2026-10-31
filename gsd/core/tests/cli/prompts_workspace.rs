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
        &["--new", "feature-x"],
        "--new feature-x",
        "workspace",
    ) {
        let text = text_of(&rendered);
        for word in [
            "git worktree add",
            "specforge init",
            "specforge add @gsd/core",
            "specforge install",
            "are not accepted by the slash command",
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
        &["--remove", "x"],
        "--remove x",
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

/// REVIEW-r43 Minor 4: remove-workspace refuses a name that leaves `~/gsd-workspaces` and a workspace that does not
/// exist, before anything is deleted. The setup line of the rendered `--remove` is run with a temp HOME.
#[test]
fn remove_workspace_guards_the_name_and_the_path() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "workspace",
        &["--remove", "--name", "x"],
        "claude-code",
    )));
    let line = text
        .lines()
        .find(|l| l.starts_with("[ -n \"$WORKSPACE_NAME\" ] ||"))
        .expect("the guard line")
        .to_string();
    let home = tempfile::tempdir().expect("a home");
    std::fs::create_dir_all(home.path().join("gsd-workspaces/real")).expect("a workspace");
    let run = |name: &str| {
        std::process::Command::new("bash")
            .args([
                "-c",
                &format!("WORKSPACE_NAME='{name}'\nRESPONSE_LANGUAGE=\n{line}\necho passed"),
            ])
            .env("HOME", home.path())
            .output()
            .expect("bash")
    };
    let passed = |name: &str| String::from_utf8_lossy(&run(name).stdout).contains("passed");
    assert!(passed("real"));
    for bad in ["../../etc", "a/b", "nope", ""] {
        assert!(!passed(bad), "{bad} was accepted");
    }
}
