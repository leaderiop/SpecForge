//! The build and test gates' and the branch rules' blocks, as `specforge gsd reference` serves them or as the agent
//! file holds them, run under bash.

use super::{Shell, asset, fence_after};
use crate::harness::{Project, project};

const POST_MERGE: &str = "gsd-core/workflows/execute-phase/steps/post-merge-gate.md";

/// The reference `path` as `specforge gsd reference` serves it, its settings filled.
fn served(p: &Project, path: &str) -> String {
    let run = p.run(&["gsd", "reference", path]);
    assert_eq!(run.code, 0, "{path}: {}", run.all());
    run.stdout
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
    assert!(err.contains("Makefile"), "{out} {err}");
}

/// A commit guard with no answer says it could not read, and never that the branch is protected.
#[test]
fn the_commit_guard_says_could_not_read_when_it_has_no_answer() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = on_branch("feat");
    let guard = fence_after(
        &asset("agents/gsd-executor.md"),
        "**0. Pre-commit HEAD safety",
    );
    let (_, err, code) = sh.run_in(p.root(), &format!("specforge() {{ return 1; }}\n{guard}"));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("could not read"), "{err}");
    assert!(!err.contains("protected"), "{err}");
    std::fs::write(p.root().join(".git/HEAD"), "not a ref\n").expect("written");
    let (_, err, code) = sh.run_in(p.root(), &guard);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("could not read"), "{err}");
    assert!(!err.contains("protected"), "{err}");
}

/// Step 0b's path check is the guard's, with a boundary: `<top>-evil` is not inside `<top>`, and `..` leaves it.
#[test]
fn the_absolute_path_guard_reads_the_answer() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = on_branch("main");
    let top = p.root().canonicalize().expect("a real path");
    let top = top.display();
    let fence = fence_after(&served(&p, PATH_SAFETY), "## Absolute-path guard");
    for (path, warns) in [
        (format!("{top}/src/a.rs"), false),
        (format!("{top}"), false),
        (format!("{top}-evil/a.rs"), true),
        (format!("{top}/../a.rs"), true),
        ("/etc/hosts".to_string(), true),
    ] {
        let (_, err, code) = sh.run_in(p.root(), &format!("ABS_PATH='{path}'\n{fence}"));
        assert_eq!(code, 0, "{path}: {err}");
        assert_eq!(err.contains("outside the worktree"), warns, "{path}: {err}");
    }
}

/// Step 0a keeps its sentinel in a linked worktree: `.linked` says it is one, whatever the directory is called.
#[test]
fn the_sentinel_fence_reads_linked_from_the_answer() {
    let Some(sh) = branch_shell() else {
        return;
    };
    let p = on_branch("main");
    let linked = tempfile::tempdir().expect("a directory");
    let at = linked.path().join("wt");
    p.git(
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "agent-1",
            &at.display().to_string(),
        ],
        "2026-01-01T00:00:00Z",
    );
    let fence = fence_after(
        &served(&p, PATH_SAFETY),
        "detects drift before every commit",
    );
    let (_, err, code) = sh.run_in(&at, &fence);
    assert_eq!(code, 0, "{err}");
    let dir = p.root().join(".git/worktrees/wt/gsd-spawn-toplevel");
    assert!(dir.exists(), "the sentinel: {err}");
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
        let ship = fence_after(
            &served(p, "gsd-core/workflows/ship.md"),
            "**On correct branch?**",
        );
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
    // The fences commit through specforge, which reads the repository's identity, not the harness's `-c` flags.
    p.git(&["config", "user.name", "gsd"], AT);
    p.git(&["config", "user.email", "gsd@example.com"], AT);
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

/// Ship's wait for the PR's state prints nothing, as the `sleep` it replaced, so the loop shows only its own lines,
/// and keeps the wait's exit: a wait that cannot wait still fails the step.
#[test]
fn ships_wait_for_the_pr_prints_nothing_and_keeps_its_exit() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/ship.md");
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("specforge wait --for 3s"))
        .expect("ship's wait")
        .trim();
    let p = project("execute");
    let home = p.root().join("home");
    std::fs::create_dir_all(&home).expect("home/");
    for (length, exit) in [("0", "0"), ("soon", "2")] {
        let (out, err, code) = sh.run_in(
            p.root(),
            &format!(
                "export HOME={}; {}; echo \"exit $?\"",
                super::q(&home.to_string_lossy()),
                line.replace("3s", length)
            ),
        );
        assert_eq!(
            (out.as_str(), code),
            (format!("exit {exit}\n").as_str(), 0),
            "{err}"
        );
    }
}

/// execute-plan's blocker fence reads the person's words through a quoted heredoc, whose `read -rd ''` keeps the
/// heredoc's last newline: the blocker records the words alone, with no newline after them.
#[test]
fn a_blocker_raised_from_execute_plan_holds_the_words_alone() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = crate::commands_support::twin();
    let text = asset("gsd-core/workflows/execute-plan.md");
    let start = text
        .lines()
        .position(|l| l.starts_with("IFS= read -rd '' BLOCKER_TEXT"))
        .expect("the blocker fence");
    let fence = text
        .lines()
        .skip(start)
        .take(3)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(fence.ends_with("\nGSD_BLOCKER_END"), "{fence}");
    let fence = fence.replace("<the blocker, verbatim>", "The file format is undecided");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE=02\n{fence}"));
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(p.fields_of("BL1")["text"], "The file format is undecided");
}

/// A tripped TDD gate stops the plan with one open blocker on its task, so the person who resumes finds the trip;
/// tripping again reuses the open blocker.
#[test]
fn a_tripped_tdd_gate_raises_a_blocker_on_its_task() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = crate::harness::taskboard();
    let gate = fence_after(
        &asset("gsd-core/workflows/execute-phase.md"),
        "**TDD gate.**",
    );
    assert!(
        gate.starts_with("if [ \"$TDD_MODE\" = \"true\" ]; then"),
        "{gate}"
    );
    let before = p.nodes_of("blocker").len();
    for _ in 0..2 {
        let (out, err, code) = sh.run_in(
            p.root(),
            &format!("TDD_MODE=true\nPLAN_ID=02-01\nTASK_ID=T1\n{gate}"),
        );
        assert_eq!(code, 1, "{out}{err}");
        assert_eq!(
            out, "TDD GATE TRIPPED: missing RED commit for 02-01/T1\n",
            "{err}"
        );
        assert_eq!(p.nodes_of("blocker").len(), before + 1);
    }
    let gates: Vec<serde_json::Value> = p
        .nodes_of("blocker")
        .iter()
        .map(|n| n["fields"]["gate"].clone())
        .filter(|g| !g.is_null())
        .collect();
    assert_eq!(gates, [serde_json::json!("v1.0/02/02-01/T1")]);
}
