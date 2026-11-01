//! G4 (plan 05 T2): the lines that built an upstream-shaped object with jq, and the lines that read it, run as the
//! asset holds them under bash with no jq on the `PATH`. The taskboard fixture (`gsd/parity/fixtures/taskboard`):
//! phases 01 (2 of 2 plans summarised), 02 (0 of 1), 02.1 and 03 (no plan); the execute project, committed, for a
//! quick task's files. A plan's commits are read in `scripts.rs`.

use super::{Shell, asset, block, line};
use crate::harness::{Project, project, taskboard};

const T0: &str = "2026-10-29T09:00:00Z";
const T1: &str = "2026-10-29T09:30:00Z";

/// `script` in the project's root: `(stdout, stderr, exit code)`.
fn run(sh: &Shell, p: &Project, script: &str) -> (String, String, i32) {
    sh.run_in(p.root(), script)
}

/// `setup`, then the lines of `path` that start with each of `starts`, in order, then `printf '[%s]'` of each of
/// `vars`.
fn values(
    sh: &Shell,
    p: &Project,
    path: &str,
    setup: &str,
    starts: &[&str],
    vars: &[&str],
) -> String {
    let text = asset(path);
    let code: Vec<&str> = starts.iter().map(|s| line(&text, s)).collect();
    let show: String = vars
        .iter()
        .map(|v| format!("printf '[%s]' \"${v}\"\n"))
        .collect();
    let (out, err, _) = run(sh, p, &format!("{setup}\n{}\n{show}", code.join("\n")));
    assert!(!err.contains("jq"), "{path}: {err}");
    out
}

/// The execute project, committed once, then `commits` (subject, files) at T0, T1, …
fn committed(commits: &[(&str, &[(&str, &str)])]) -> Project {
    let p = project("execute");
    p.commit();
    for ((subject, files), at) in commits.iter().zip([T0, T1]) {
        p.git_commit(subject, files, at);
    }
    p
}

#[test]
fn a_count_of_plans_and_summaries_is_picked() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let starts = ["PLAN_COUNT=$(", "SUMMARY_COUNT=$("];
    let counts = |path, setup| {
        values(
            &sh,
            &p,
            path,
            setup,
            &starts,
            &["PLAN_COUNT", "SUMMARY_COUNT"],
        )
    };
    let plan = "gsd-core/workflows/execute-plan.md";
    assert_eq!(counts(plan, "PHASE_ID=v1.0/01"), "[2][2]");
    assert_eq!(counts(plan, "PHASE_ID=v1.0/02"), "[1][0]");
    let progress = "gsd-core/workflows/progress.md";
    let xy = values(
        &sh,
        &p,
        progress,
        "CURRENT_PHASE=v1.0/02",
        &["X=$(", "Y=$("],
        &["X", "Y"],
    );
    assert_eq!(xy, "[1][0]");
}

#[test]
fn a_phase_is_read_field_by_field() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let mvp = "gsd-core/workflows/mvp-phase.md";
    let starts = ["PHASE_FOUND=", "PHASE_MODE=$(", "DISK_STATUS=$("];
    let vars = ["PHASE_FOUND", "PHASE_MODE", "DISK_STATUS"];
    assert_eq!(
        values(&sh, &p, mvp, "PHASE_ID=v1.0/02", &starts, &vars),
        "[true][][planned]"
    );
    assert_eq!(
        values(&sh, &p, mvp, "PHASE_ID=v1.0/nope", &starts, &vars),
        "[false][][]"
    );
    let name = values(
        &sh,
        &p,
        mvp,
        "PHASE_ID=v1.0/02",
        &["PHASE_NAME=$("],
        &["PHASE_NAME"],
    );
    assert!(name.len() > 2, "{name}");
    let execute = "gsd-core/workflows/execute-phase.md";
    let starts = ["PHASE_STATUS=$(", "case \"$PHASE_STATUS\""];
    assert_eq!(
        values(
            &sh,
            &p,
            execute,
            "PHASE_ID=v1.0/02",
            &starts,
            &["PHASE_MARKED"]
        ),
        "[false]"
    );
}

#[test]
fn a_milestone_is_verified_only_when_every_phase_is() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/complete-milestone.md";
    let verified = values(
        &sh,
        &p,
        path,
        "MILESTONE=v1.0",
        &["ALL_PHASES_VERIFIED="],
        &["ALL_PHASES_VERIFIED"],
    );
    assert_eq!(verified, "[false]");
}

#[test]
fn the_open_windows_are_counted_and_an_unreadable_ledger_is_unknown() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/ship.md";
    let starts = ["WINDOWS_STATUSES=$(", "WINDOWS_OPEN_COUNT=$("];
    let count = |setup| values(&sh, &p, path, setup, &starts, &["WINDOWS_OPEN_COUNT"]);
    assert_eq!(count("MILESTONE=v1.0"), "[0]");
    assert_eq!(count("MILESTONE=nope"), "[?]");
}

#[test]
fn a_verification_status_routes_in_the_shell() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let goal = "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md";
    let starts = ["STATUS=$(", "case \"$STATUS\""];
    assert_eq!(
        values(
            &sh,
            &p,
            goal,
            "PHASE_ID=v1.0/02",
            &starts,
            &["STATUS", "ROUTE"]
        ),
        "[pending][verify]"
    );
    let work = "gsd-core/workflows/verify-work.md";
    let starts = [
        "VERIFICATION_STATUS_VALUE=$(",
        "case \"$VERIFICATION_STATUS_VALUE\"",
    ];
    let vars = ["VERIFICATION_STATUS_VALUE", "VERIFICATION_ROUTE"];
    assert_eq!(
        values(&sh, &p, work, "PHASE_ID=v1.0/02", &starts, &vars),
        "[pending][verify]"
    );
}

#[test]
fn a_quick_tasks_changed_files_are_its_commits_files() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = committed(&[
        (
            "feat(01-01): Store module",
            &[("src/b.js", "x\n"), ("src/a.js", "y\n")],
        ),
        ("fix(01-01): Store fix", &[("src/a.js", "z\n")]),
    ]);
    let text = asset("gsd-core/workflows/quick.md");
    let code = block(&text, "QUICK_SCOPE_RC=0", "CHANGED_FILES=$(");
    let (out, err, _) = run(
        &sh,
        &p,
        &format!("quick_id=v1.0/01/01-01\n{code}\nprintf '[%s]' \"$CHANGED_FILES\""),
    );
    assert_eq!(out, "[src/a.js src/b.js]", "{err}");
}

#[test]
fn a_plans_files_are_read_from_its_entity() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let path = "gsd-core/workflows/execute-phase/steps/per-plan-worktree-gate.md";
    let id = run(
        &sh,
        &p,
        "specforge query v1.0/01 --kinds plan --pick '.nodes[kind=plan].id' | head -1",
    )
    .0;
    let setup = format!("plan_id={}", id.trim());
    let files = values(&sh, &p, path, &setup, &["PLAN_FILES=$("], &["PLAN_FILES"]);
    assert!(files.len() > 2 && !files.contains('\n'), "{files}");
}
