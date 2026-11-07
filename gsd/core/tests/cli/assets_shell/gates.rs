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

const PATH_SAFETY: &str = "gsd-core/references/worktree-path-safety.md";

/// Step 0c reads the task's commands from the graph, and any read that fails stops the block.
#[test]
fn the_step_0c_fence_reads_the_tasks_commands_and_stops_on_a_failed_answer() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let run = |p: &Project, task: &str| {
        let fence = fence_after(&served(p, PATH_SAFETY), "command guard — step 0c");
        let fence = fence.replacen("TASK_ID='{TASK_ID}'", &format!("TASK_ID='{task}'"), 1);
        sh.run_in(p.root(), &fence)
    };
    let p = project("execute");
    p.commit();
    assert_eq!(run(&p, "v1.0/01/01-01/T1").2, 0, "a clean command passes");
    assert_ne!(run(&p, "{TASK_ID}").2, 0, "an unbound template");
    assert_ne!(run(&p, "").2, 0, "an empty task");
    assert_ne!(run(&p, "v1.0/01/01-01/T9").2, 0, "no such task");

    let spec = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &spec.replacen(
            "automated \"node --test test/tasks.test.js\"",
            "automated \"cd ../elsewhere && make\"",
            1,
        ),
    );
    let (out, err, code) = run(&p, "v1.0/01/01-01/T1");
    assert_ne!(code, 0, "it leaves the worktree");
    assert!(format!("{out}{err}").contains("FATAL"), "{out} {err}");

    // Outside a repository `git repo` fails: the block stops, it does not pass with an empty common dir.
    let outside = project("execute");
    assert_ne!(run(&outside, "v1.0/01/01-01/T1").2, 0, "no repository");
}

const PROTECTED_BRANCH: &str = "gsd-core/workflows/execute-phase/steps/protected-branch.md";

/// `Shell` for the blocks that read the branch: specforge and git, and no `grep` to hold a rule.
fn branch_shell() -> Option<Shell> {
    Shell::only(&["git", "printf"])
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

/// The executor's commit guard reads the setting that lets it commit on the default branch.
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
    assert_eq!(code, 0, "the setting is read: {err}");
    p.settings(serde_json::json!({}));
    let (_, err, code) = sh.run_in(p.root(), &guard);
    assert_eq!(code, 1, "no override, no commit: {err}");
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
    assert!(warns(&p), "trunk is in the set");
    let p = on_branch("shared/x");
    p.settings(serde_json::json!({"git.protected_branches": ["shared/x"]}));
    assert!(warns(&p), "the setting is read");
}

/// What complete-milestone squash-merges: the branches its phases compute and that exist, never another local branch.
#[test]
fn complete_milestone_lists_only_the_phase_branches() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = crate::harness::taskboard();
    p.settings(serde_json::json!({"git.branching_strategy": "phase"}));
    p.commit();
    let at = "2026-01-01T00:00:00Z";
    p.git(&["branch", "feature/x"], at);
    p.git(&["branch", "gsd/phase-01-core"], at);
    let served = served(
        &p,
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
    );
    let listing = fence_after(&served, "**For \"phase\" strategy:**");
    let (out, err, _) = sh.run_in(
        p.root(),
        &format!(
            "MILESTONE=v1.0\n{listing}\nprintf '%s|%s' \"$BRANCHING_STRATEGY\" \"$PHASE_BRANCHES\""
        ),
    );
    assert_eq!(out, "phase|gsd/phase-01-core", "{err}");
}

/// The milestone strategy's one branch, once it exists.
#[test]
fn complete_milestone_finds_the_milestone_branch_when_it_exists() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = crate::harness::taskboard();
    p.settings(serde_json::json!({"git.branching_strategy": "milestone"}));
    p.commit();
    let served = served(
        &p,
        "gsd-core/workflows/complete-milestone/detail/elaboration.md",
    );
    let listing = fence_after(&served, "**For \"milestone\" strategy:**");
    let line = format!(
        "MILESTONE=v1.0\n{listing}\nprintf '%s|%s' \"$BRANCHING_STRATEGY\" \"$MILESTONE_BRANCH\""
    );
    let (out, err, _) = sh.run_in(p.root(), &line);
    assert_eq!(out, "milestone|", "{err}");
    p.git(&["branch", "feature/x"], "2026-01-01T00:00:00Z");
    p.git(
        &["branch", "gsd/v1.0-taskboard-1-0"],
        "2026-01-01T00:00:00Z",
    );
    let (out, err, _) = sh.run_in(p.root(), &line);
    assert_eq!(out, "milestone|gsd/v1.0-taskboard-1-0", "{err}");
}
