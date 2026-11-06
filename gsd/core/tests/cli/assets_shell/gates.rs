//! The build and test gates' and the branch rules' blocks, as `specforge gsd reference` serves them or as the agent
//! file holds them, run under bash.

use super::{Shell, asset};
use crate::harness::{Project, project};

const POST_MERGE: &str = "gsd-core/workflows/execute-phase/steps/post-merge-gate.md";

/// The reference `path` as `specforge gsd reference` serves it, its settings filled.
fn served(p: &Project, path: &str) -> String {
    let run = p.run(&["gsd", "reference", path]);
    assert_eq!(run.code, 0, "{path}: {}", run.all());
    run.stdout
}

/// The first shell fence of `text` after the first line holding `mark`.
fn fence_after(text: &str, mark: &str) -> String {
    let after = text
        .split_once(mark)
        .unwrap_or_else(|| panic!("no {mark}"))
        .1;
    let body = after
        .split_once("```bash\n")
        .unwrap_or_else(|| panic!("no fence after {mark}"))
        .1;
    body.split_once("\n```")
        .unwrap_or_else(|| panic!("the fence after {mark} ends"))
        .0
        .to_string()
}

/// A file a project holds can name a command: the python build gate spliced the name into the string `bash -c` read.
#[test]
fn a_python_file_name_never_runs_as_a_command() {
    let Some(sh) = Shell::only(&["git", "touch", "mkdir", "printf", "bash"]) else {
        return;
    };
    let p = project("execute");
    p.write("pyproject.toml", "[project]\nname = \"x\"\n");
    p.write("x;touch pwned;y.py", "print(1)\n");
    let step_a = fence_after(&served(&p, POST_MERGE), "**Step A");
    sh.run_in(p.root(), &step_a);
    assert!(
        !p.root().join("pwned").exists(),
        "a file's name never runs as a command"
    );
}

const PROTECTED_BRANCH: &str = "gsd-core/workflows/execute-phase/steps/protected-branch.md";

/// `Shell` for the blocks that read the branch: specforge, git and what the blocks filter with.
fn branch_shell() -> Option<Shell> {
    Shell::only(&["git", "printf", "grep"])
}

/// A repository on `main`, with a commit, then switched to `branch` when it is another.
fn on_branch(branch: &str) -> Project {
    let p = project("execute");
    p.commit();
    if branch != "main" {
        p.git(&["checkout", "-q", "-b", branch], "2026-01-01T00:00:00Z");
    }
    p
}

/// The executor's commit guard names a setting that lets it commit on the default branch.
#[test]
fn the_commit_guard_reads_the_default_branch_override() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = on_branch("main");
    p.settings(serde_json::json!({"git.allow_default_branch_commits": true}));
    let guard = fence_after(
        &asset("agents/gsd-executor.md"),
        "**0. Pre-commit HEAD safety",
    );
    let (_, err, code) = sh.run_in(p.root(), &guard);
    assert_eq!(code, 1, "today the setting is not read: {err}");
    assert!(err.contains("protected/default branch"), "{err}");
}

/// The `none` strategy warns on every protected branch: `trunk` and a listed one too.
#[test]
fn protected_branch_warns_on_trunk_and_on_a_listed_branch() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let warns = |p: &Project| {
        let step = fence_after(&served(p, PROTECTED_BRANCH), "# Protected-branch warning");
        sh.run_in(p.root(), &step)
            .1
            .contains("is a protected branch")
    };
    let p = on_branch("trunk");
    assert!(!warns(&p), "today trunk is not in the set");
    let p = on_branch("shared/x");
    p.settings(serde_json::json!({"git.protected_branches": ["shared/x"]}));
    assert!(!warns(&p), "today the setting is not read");
}
