//! A count a block takes from a graph query: a query that fails is not a count of zero.

use super::{Shell, asset, line};

/// The line of `path` that starts with `start`, run after `setup` in a project without that id, then `echo`ed:
/// `(stdout, stderr, exit code)`.
fn run_count(sh: &Shell, path: &str, start: &str, setup: &str, var: &str) -> (String, String, i32) {
    let text = asset(path);
    let line = line(&text, start);
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

/// plan-phase's walking-skeleton gate, end to end: its fence reads its own answers (the `--mvp` flag, the phase's
/// `mode`, the phase's milestone, its summaries) and fails closed on an empty one, never a variable no line of the
/// fence sets. Only phase 01 of an mvp milestone with no summary yet starts a walking skeleton.
#[test]
fn the_walking_skeleton_gate_reads_its_own_answers() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let text = asset("gsd-core/workflows/plan-phase.md");
    let fence = {
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.starts_with("WALKING_SKELETON=false;"))
            .expect("the gate");
        let end = at
            + lines[at..]
                .iter()
                .position(|l| *l == "```")
                .expect("its fence's end");
        lines[at..end].join("\n")
    };
    let p = crate::harness::taskboard();
    p.write(
        "spec/v2.spec",
        "milestone v2.0 \"Next\" {\n  name \"Next\"\n}\n\nphase 01 \"Start\" {\n  milestone v2.0\n  name \"Start\"\n  slug \"start\"\n  goal \"Start again\"\n  mode mvp\n}\n\nphase 02 \"More\" {\n  milestone v2.0\n  name \"More\"\n  slug \"more\"\n  goal \"More\"\n  mode mvp\n}\n",
    );
    let gate = |phase: &str, mvp: &str| {
        let filled = fence.replace("{{arg:mvp|false}}", mvp);
        sh.run_in(
            p.root(),
            &format!("PHASE_ID={phase}\n{filled}\nprintf '%s' \"$WALKING_SKELETON\""),
        )
    };
    for (phase, mvp, want) in [
        ("v2.0/01", "false", "true"), // mvp by its mode, phase 01, no summary in v2.0
        ("v2.0/02", "false", "false"), // not phase 01
        ("v1.0/01", "false", "false"), // standard mode
        ("v1.0/01", "true", "false"), // --mvp, but v1.0 has summaries
    ] {
        let (out, err, code) = gate(phase, mvp);
        assert_eq!(
            (code, out.as_str()),
            (0, want),
            "{phase} --mvp={mvp}: {err}"
        );
    }
    // An unknown phase, or none, stops the block.
    for phase in ["v9.9/01", ""] {
        let (out, err, code) = gate(phase, "true");
        assert_ne!(code, 0, "{phase:?}: {out}{err}");
        assert!(out.is_empty(), "{phase:?}: {out}");
    }
}
