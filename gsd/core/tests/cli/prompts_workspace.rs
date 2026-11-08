//! `workspace` and `update` (T10): the workspace workflows work in the shell, and `update` renders our text because
//! upstream's installs upstream GSD over this one.

use super::commands_support::twin;
use super::prompts_support::{payload, prompt, render_three_hosts, text_of, with_specforge_first};

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
            "`--strategy worktree|clone`",
        ] {
            assert!(text.contains(word), "{host}: no '{word}'");
        }
    }
}

#[test]
fn workspace_takes_upstreams_flags() {
    let p = twin();
    render_three_hosts(
        &p,
        "workspace",
        &[
            "--new",
            "--name",
            "w",
            "--path",
            "../w",
            "--repos",
            "a,b",
            "--strategy",
            "clone",
            "--branch",
            "feat",
            "--auto",
        ],
        "--new --repos \"a,b\" --path \"../w\" --strategy clone --branch \"feat\" --auto w",
        "workspace",
    );
    let run = prompt(
        &p,
        "workspace",
        &["--new", "--strategy", "copy"],
        "claude-code",
    );
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("strategy"), "{}", run.all());
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

/// Remove-workspace refuses a name that leaves `~/gsd-workspaces` and a workspace that does not
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
    let home = approved_home();
    std::fs::create_dir_all(home.path().join("gsd-workspaces/real")).expect("a workspace");
    let run = |name: &str| {
        std::process::Command::new("bash")
            .args([
                "-c",
                &format!("WORKSPACE_NAME='{name}'\nRESPONSE_LANGUAGE=\n{line}\necho passed"),
            ])
            .current_dir(p.root())
            .env("HOME", home.path())
            .env("PATH", with_specforge_first())
            .output()
            .expect("bash")
    };
    let passed = |name: &str| String::from_utf8_lossy(&run(name).stdout).contains("passed");
    assert!(passed("real"));
    // The verbs hold a name that gets past the argument's kind, and what does not exist is not found.
    for bad in ["../../etc", "..", "nope", ""] {
        assert!(!passed(bad), "{bad} was accepted");
    }
    // A name the prompt is given is one path segment before anything renders.
    for bad in ["../../etc", "a/b", "..", "."] {
        let run = prompt(&p, "workspace", &["--remove", "--name", bad], "claude-code");
        assert_ne!(run.code, 0, "{bad}: {}", run.all());
        assert!(
            run.all().contains("one path segment"),
            "{bad}: {}",
            run.all()
        );
    }
}

/// A home whose machine root `~/gsd-workspaces` is approved, as the workspace's first block asks.
fn approved_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("a home");
    let out = std::process::Command::new("bash")
        .args([
            "-c",
            "specforge grants approve --root workspaces --at '~/gsd-workspaces'",
        ])
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("PATH", with_specforge_first())
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    home
}

/// A repository whose status cannot be read must not count as clean: `rm -rf` follows in step 5. Its `.git` names a
/// git directory that does not exist, so no status can read it, with git installed or not.
#[test]
fn remove_workspace_counts_a_repo_it_cannot_read_as_dirty() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "workspace",
        &["--remove", "--name", "ws"],
        "claude-code",
    )));
    let line = text
        .lines()
        .find(|l| l.starts_with("[ -n \"$WORKSPACE_NAME\" ] ||"))
        .expect("the guard line")
        .to_string();
    let home = approved_home();
    let repo = home.path().join("gsd-workspaces/ws/repo");
    std::fs::create_dir_all(&repo).expect("a repository");
    std::fs::write(repo.join(".git"), "gitdir: /nonexistent\n").expect("its .git");
    let out = std::process::Command::new("bash")
        .args([
            "-c",
            &format!("WORKSPACE_NAME=ws\nRESPONSE_LANGUAGE=\n{line}\necho \"[$DIRTY_REPOS]\""),
        ])
        .current_dir(home.path())
        .env("HOME", home.path())
        .env("PATH", with_specforge_first())
        .output()
        .expect("bash");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "[repo]");
}

/// The whole round through a machine root, by the rendered lines: not approved, the blocks read nothing; approved, a worktree and a clone land in it, `list` finds the workspace, and `remove` takes the worktree off
/// its source repository's records and deletes the workspace.
#[test]
fn a_workspace_round_trips_through_the_machine_root() {
    let p = twin();
    let text = |args: &[&str]| text_of(&payload(&prompt(&p, "workspace", args, "claude-code")));
    let new = text(&["--name", "ws"]);
    let list = text(&["--list"]);
    let remove = text(&["--remove", "--name", "ws"]);
    let starting = |text: &str, start: &str| -> String {
        text.lines()
            .find(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no line starts {start}"))
            .to_string()
    };
    let reachable = starting(&new, "PROJECT_ROOT=$(specforge git repo");
    let listed = starting(&list, "WORKSPACE_BASE=");
    let guard = starting(&remove, "[ -n \"$WORKSPACE_NAME\" ] ||");
    let linked = starting(&remove, "LINKED_REPOS=");
    let source = starting(&remove, "SOURCE_REPO_PATH=");
    let removed = starting(&remove, "if ! specforge git worktree remove");
    let delete = starting(&remove, "specforge files remove -r");
    let mkdir = starting(&new, "specforge files mkdir");
    let add = starting(
        &new,
        "specforge git worktree add \"$TARGET_PATH/$REPO_NAME\" --branch \"$BRANCH_NAME\"",
    );
    let clone = starting(&new, "specforge git clone");
    let suffix = starting(&new, "HEAD_SHA=");
    let retry = starting(
        &new,
        "specforge git worktree add \"$TARGET_PATH/$REPO_NAME\" --branch \"${BRANCH_NAME}-${HEAD_SHA}\"",
    );

    let home = tempfile::tempdir().expect("a home");
    let bash = |script: &str| {
        let out = std::process::Command::new("bash")
            .args(["-c", script])
            .current_dir(p.root())
            .env("HOME", home.path())
            .env("PATH", with_specforge_first())
            .output()
            .expect("bash");
        (
            String::from_utf8_lossy(&out.stdout).trim().to_string(),
            String::from_utf8_lossy(&out.stderr).to_string(),
        )
    };
    p.git_commit("first", &[("README.md", "first\n")], "2026-01-01T00:00:00");

    // Not approved: nothing is read, and the blocks say so by what they leave empty.
    let (empty, _) = bash(&format!("{reachable}\necho \"[$WORKSPACES_REACHABLE]\""));
    assert_eq!(empty, "[]");
    let (empty, _) = bash(&format!("{listed}\necho \"[$WORKSPACE_COUNT]\""));
    assert_eq!(empty, "[]");

    let (_, why) = bash("specforge grants approve --root workspaces --at '~/gsd-workspaces'");
    let (yes, _) = bash(&format!("{reachable}\necho \"[$WORKSPACES_REACHABLE]\""));
    assert_eq!(yes, "[true]", "{why}");

    // Two repositories land in it, one a worktree.
    let target = home.path().join("gsd-workspaces/ws");
    let vars = format!(
        "TARGET_PATH='{}'; REPO_NAME=wt; BRANCH_NAME=workspace/ws; SOURCE_REPO_PATH='{}'",
        target.display(),
        p.root().display(),
    );
    let (_, why) = bash(&format!("{vars}\n{mkdir}\n{add}\nREPO_NAME=cl\n{clone}"));
    assert!(target.join("wt/.git").is_file(), "{why}");
    assert!(target.join("cl/.git").is_dir(), "{why}");
    // The branch taken already, the retry names one for the source's head.
    let (_, why) = bash(&format!("{vars}\nREPO_NAME=rt\n{suffix}\n{retry}"));
    assert!(target.join("rt/.git").is_file(), "{why}");
    let (_, why) = bash(&format!(
        "{vars}\nREPO_NAME=rt\nspecforge git worktree remove \"$TARGET_PATH/$REPO_NAME\" --force"
    ));
    assert!(!target.join("rt").exists(), "{why}");

    let (count, _) = bash(&format!("{listed}\necho \"[$WORKSPACE_COUNT]\""));
    assert_eq!(count, "[1]");

    // Only the worktree is taken off its source repository's records; the whole workspace goes.
    let setup = format!("WORKSPACE_NAME=ws\nRESPONSE_LANGUAGE=\n{guard}");
    let (repos, _) = bash(&format!(
        "{setup}\n{linked}\necho \"[$LINKED_REPOS] [$DIRTY_REPOS]\""
    ));
    assert_eq!(repos, "[wt] []");
    let (_, why) = bash(&format!(
        "{setup}\nREPO_NAME=wt\nREMOVE_FAILED=false\n{source}\n{removed}\n  REMOVE_FAILED=true\nfi\n\
         [ \"$REMOVE_FAILED\" = false ] || exit 1\n{delete}"
    ));
    assert!(!target.exists(), "{why}");
    let (worktrees, _) = bash("specforge git worktrees --pick '.worktrees[].path' --count");
    assert_eq!(
        worktrees, "1",
        "the source repository still records the worktree"
    );
}

/// `child_repos` is what has a `.git` entry, the file of a linked worktree included, and its count is the same.
#[test]
fn new_workspace_counts_a_child_repo_by_its_git_entry() {
    let p = twin();
    let text = text_of(&payload(&prompt(
        &p,
        "workspace",
        &["--name", "ws"],
        "claude-code",
    )));
    let line = text
        .lines()
        .find(|l| l.starts_with("CHILD_REPOS="))
        .expect("the child repos line")
        .to_string();
    let dir = tempfile::tempdir().expect("a directory");
    let run = || {
        let out = std::process::Command::new("bash")
            .args([
                "-c",
                &format!("{line}\necho \"$CHILD_REPO_COUNT:\" $CHILD_REPOS"),
            ])
            .current_dir(dir.path())
            .env("HOME", dir.path())
            .env("PATH", with_specforge_first())
            .output()
            .expect("bash");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    assert_eq!(run(), "0:");
    std::fs::create_dir_all(dir.path().join("plain")).expect("a directory");
    std::fs::create_dir_all(dir.path().join("cloned/.git")).expect("a repository");
    std::fs::create_dir_all(dir.path().join("linked")).expect("a worktree");
    std::fs::write(dir.path().join("linked/.git"), "gitdir: elsewhere\n").expect("its .git");
    assert_eq!(run(), "2: cloned linked");
}
