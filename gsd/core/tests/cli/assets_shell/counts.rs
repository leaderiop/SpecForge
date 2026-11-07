//! A count a block takes from a graph query: a query that fails is not a count of zero.

use super::{Shell, asset};

/// The line of `path` that starts with `start`, run after `setup` in a project without that id, then `echo`ed:
/// `(stdout, stderr, exit code)`.
fn run_count(sh: &Shell, path: &str, start: &str, setup: &str, var: &str) -> (String, String, i32) {
    let text = asset(path);
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("{path}: no line starting {start}"));
    let p = crate::harness::project("empty");
    sh.run_in(p.root(), &format!("{setup}\n{line}\necho \"{var}=${var}\""))
}

/// The walking-skeleton gate and the inline-or-spawn route count summaries and tasks: an id the graph does not
/// hold makes the query fail (E003), and the block stops instead of reading that as "no summaries" (a new project)
/// or "no tasks" (run inline).
#[test]
fn a_failed_count_query_is_not_zero() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    for (path, start, setup, var) in [
        (
            "gsd-core/workflows/plan-phase.md",
            "PRIOR_SUMMARIES=$(",
            "MILESTONE=no-such-milestone",
            "PRIOR_SUMMARIES",
        ),
        (
            "gsd-core/workflows/execute-plan.md",
            "TASK_COUNT=$(",
            "PLAN_ID=no-such-plan",
            "TASK_COUNT",
        ),
        (
            "gsd-core/workflows/plan-phase.md",
            "DISK_PLANS=$(",
            "PHASE_ID=v1.0/99",
            "DISK_PLANS",
        ),
    ] {
        let (out, err, code) = run_count(&sh, path, start, setup, var);
        assert_ne!(code, 0, "{path}: {out}");
        assert!(out.is_empty(), "{path}: {out}");
        assert!(err.contains("E003"), "{path}: {err}");
    }
}

/// The ahead-of-origin warning counts commits: a log that fails (no repository here) stops the block instead of
/// reading as "nothing ahead".
#[test]
fn a_failed_ahead_count_is_not_zero() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let (out, err, code) = run_count(
        &sh,
        "gsd-core/workflows/execute-phase.md",
        "AHEAD",
        "DEFAULT_BRANCH=main",
        "AHEAD",
    );
    assert_ne!(code, 0, "{out}");
    assert!(out.is_empty(), "{out}{err}");
}

/// The ahead count of a repository that answers: commits on the local default branch past origin's, and none when
/// there is no local default branch (only origin's copy).
#[test]
fn the_ahead_count_reads_the_local_branch_when_there_is_one() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/execute-phase.md");
    let line = super::line(&text, "AHEAD");
    let p = crate::harness::project("empty");
    p.commit();
    let at = "2026-01-01T00:00:00Z";
    p.git(&["update-ref", "refs/remotes/origin/main", "HEAD"], at);
    p.git(&["update-ref", "refs/remotes/origin/trunk", "HEAD"], at);
    p.git_commit("ahead", &[("a.txt", "a\n")], at);
    for (branch, want) in [("main", "1"), ("trunk", "0")] {
        let (out, err, code) = sh.run_in(
            p.root(),
            &format!("DEFAULT_BRANCH={branch}\n{line}\nprintf '%s' \"$AHEAD\""),
        );
        assert_eq!((code, out.as_str()), (0, want), "{branch}: {err}");
    }
}
