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
    ] {
        let (out, err, code) = run_count(&sh, path, start, setup, var);
        assert_eq!(code, 0, "{path}: {err}");
        assert_eq!(out, format!("{var}=0\n"), "{path}: {err}");
    }
}
