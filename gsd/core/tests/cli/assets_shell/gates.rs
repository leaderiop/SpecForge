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

/// The gate's fence stops on a manifest it could not read: the gate does not pass by not running.
#[cfg(unix)]
#[test]
fn the_gate_fences_stop_on_a_manifest_present_but_unread() {
    let Some(sh) = Shell::only(&["git", "touch", "mkdir", "printf", "bash"]) else {
        return;
    };
    let p = project("execute");
    std::os::unix::fs::symlink("/nonexistent/Makefile", p.root().join("Makefile")).expect("a link");
    let step_a = fence_after(&served(&p, POST_MERGE), "**Step A");
    let (out, err, code) = sh.run_in(p.root(), &step_a);
    assert_ne!(code, 0, "{out} {err}");
    assert!(out.contains("Makefile"), "{out} {err}");
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

/// A protected list the answer cannot read refuses the commit, on the branch it meant to protect.
#[test]
fn the_commit_guard_refuses_on_a_broken_protected_list() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = on_branch("feat");
    let guard = fence_after(
        &asset("agents/gsd-executor.md"),
        "**0. Pre-commit HEAD safety",
    );
    let (_, err, code) = sh.run_in(p.root(), &guard);
    assert_eq!(code, 0, "a free branch commits: {err}");
    for broken in [serde_json::json!("feat"), serde_json::json!(["feat", 1])] {
        p.settings(serde_json::json!({"git.protected_branches": broken}));
        let (_, err, code) = sh.run_in(p.root(), &guard);
        assert_eq!(code, 1, "{broken}: {err}");
        assert!(err.contains("git.protected_branches"), "{broken}: {err}");
    }
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

/// A name git refuses stops the fence that would hand it to `git switch`, on the branch it started on.
#[test]
fn a_phase_with_no_branch_name_stops_the_branch_creation_fence() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = crate::harness::taskboard();
    p.settings(serde_json::json!({
        "git.branching_strategy": "phase",
        "git.phase_branch_template": "--help",
    }));
    p.commit();
    let fence = fence_after(
        &served(&p, "gsd-core/references/planning-config.md"),
        "**Branch creation:**",
    );
    let (out, err, code) = sh.run_in(
        p.root(),
        &format!(
            "BRANCHING_STRATEGY=phase
PHASE_ID=v1.0/01
{fence}
echo went-on"
        ),
    );
    assert_ne!(code, 0, "{out} {err}");
    assert!(err.contains("gsd/W409"), "{out} {err}");
    assert!(!out.contains("went-on"), "{out} {err}");
    let head = p.run(&["git", "head", "--pick", ".branch"]);
    assert_eq!(head.stdout.trim(), "main", "{}", head.all());
}

/// A HEAD no one can read is not "not protected": both warnings say they could not tell, and a free branch says nothing.
#[test]
fn a_broken_head_warns_that_it_could_not_determine() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let fences = |p: &Project| {
        let ship = served(p, "gsd-core/workflows/ship.md");
        let ship = ship
            .split_once("**On correct branch?**")
            .and_then(|(_, after)| after.split_once("```bash\n"))
            .and_then(|(_, body)| body.split_once("\n   ```"))
            .expect("ship's fence")
            .0
            .to_string();
        [
            fence_after(&served(p, PROTECTED_BRANCH), "# Protected-branch warning"),
            ship,
        ]
    };
    let p = on_branch("feat");
    for fence in fences(&p) {
        let (out, err, code) = sh.run_in(p.root(), &fence);
        assert_eq!(code, 0, "{fence}: {err}");
        assert!(
            !format!("{out}{err}").contains('⚠'),
            "a free branch warns: {out} {err}"
        );
    }
    let body = fences(&p);
    std::fs::write(p.root().join(".git/HEAD"), "not a ref\n").expect("written");
    for fence in body {
        let (out, err, code) = sh.run_in(p.root(), &fence);
        assert_eq!(code, 0, "it warns and goes on: {fence}: {err}");
        assert!(err.contains("Could not determine"), "{fence}: {out} {err}");
    }
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

/// A taskboard with the phase strategy and work on phase 01's branch, back on `main`.
fn with_a_phase_branch() -> (Project, String) {
    let p = crate::harness::taskboard();
    p.settings(serde_json::json!({"git.branching_strategy": "phase"}));
    p.commit();
    p.git(&["checkout", "-q", "-b", "gsd/phase-01-core"], AT);
    p.write("work.txt", "done\n");
    p.git(&["add", "work.txt"], AT);
    p.git(&["commit", "-q", "-m", "work"], AT);
    p.git(&["checkout", "-q", "main"], AT);
    let main = p.git(&["rev-parse", "main"], AT);
    (p, main)
}

const AT: &str = "2026-01-01T00:00:00Z";
const ELABORATION: &str = "gsd-core/workflows/complete-milestone/detail/elaboration.md";

/// Each option's fence runs alone: a host keeps no shell between fences, so nothing it reads comes from another.
#[test]
fn each_complete_milestone_option_runs_from_its_own_fence() {
    let Some(sh) = branch_shell() else {
        return;
    };
    for (option, moves) in [
        ("**Squash merge:**", true),
        ("**Merge with history:**", true),
    ] {
        let (p, before) = with_a_phase_branch();
        let fence = fence_after(&served(&p, ELABORATION), option);
        let (out, err, code) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{fence}"));
        assert_eq!(code, 0, "{option}: {out} {err}");
        assert_eq!(
            p.git(&["rev-parse", "main"], AT) != before,
            moves,
            "{option}: {out} {err}"
        );
        assert!(p.root().join("work.txt").exists(), "{option}: {out} {err}");
    }
    let (p, _) = with_a_phase_branch();
    let fence = fence_after(&served(&p, ELABORATION), "**Delete without merging:**");
    let (out, err, code) = sh.run_in(p.root(), &format!("MILESTONE=v1.0\n{fence}"));
    assert_eq!(code, 0, "{out} {err}");
    let branches = p.git(&["branch", "--list", "gsd/phase-01-core"], AT);
    assert!(
        branches.is_empty(),
        "the branch is gone: {branches} {out} {err}"
    );
}

/// A value no fence bound stops the fence loudly, not as "no branches found".
#[test]
fn an_unset_milestone_or_phase_fails_loudly() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let (p, before) = with_a_phase_branch();
    let served_body = served(&p, ELABORATION);
    for option in [
        "**For \"phase\" strategy:**",
        "**For \"milestone\" strategy:**",
        "**Squash merge:**",
        "**Merge with history:**",
        "**Delete without merging:**",
    ] {
        let fence = fence_after(&served_body, option);
        let (out, err, code) = sh.run_in(p.root(), &format!("{fence}\necho went-on"));
        assert_ne!(code, 0, "{option}: {out} {err}");
        assert!(err.contains("MILESTONE"), "{option}: {out} {err}");
        assert!(!out.contains("went-on"), "{option}: {out} {err}");
    }
    assert_eq!(
        p.git(&["rev-parse", "main"], AT),
        before,
        "main did not move"
    );
    let config = served(&p, "gsd-core/references/planning-config.md");
    let creation = fence_after(&config, "**Branch creation:**");
    let (out, err, code) = sh.run_in(p.root(), &format!("BRANCHING_STRATEGY=phase\n{creation}"));
    assert_ne!(code, 0, "{out} {err}");
    assert!(err.contains("PHASE_ID"), "{out} {err}");
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
