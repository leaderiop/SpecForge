//! The lines that built an upstream-shaped object with jq, and the lines that read it, run as the
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
    // the resume scan reads a phase's counts from the progress report by its number, not by its index
    let scan = values(
        &sh,
        &p,
        progress,
        "PHASE_NUM=02",
        &[
            "PLAN_COUNT=$(specforge gsd progress-report",
            "SUMMARY_COUNT=$(specforge gsd progress-report",
        ],
        &["PLAN_COUNT", "SUMMARY_COUNT"],
    );
    assert_eq!(scan, "[1][0]");
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
    let starts = ["WINDOWS_OPEN_COUNT=$("];
    let count = |setup| values(&sh, &p, path, setup, &starts, &["WINDOWS_OPEN_COUNT"]);
    assert_eq!(count("MILESTONE=v1.0"), "[0]");
    assert_eq!(count("MILESTONE=nope"), "[?]");
    // a window with neither a waiver nor a fix is open
    let mut verify = p.read("spec/verify.spec");
    verify.push_str(OPEN_WINDOW);
    p.write("spec/verify.spec", &verify);
    assert_eq!(count("MILESTONE=v1.0"), "[1]");
}

/// A broken window of phase 01 that is open: no waiver, no fix.
pub(super) const OPEN_WINDOW: &str = "\nbroken_window BW1 \"A stub\" {\n  kind stub\n  phase v1.0/01\n  description \"a stub left in\"\n}\n";

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
    // an empty scope says why, in the words of what was read (there is no resolver's status)
    let why = block(
        &text,
        "QUICK_SCOPE_RC=0",
        "Quick-task review scope is empty",
    ) + "\nfi";
    let (out, err, _) = run(&sh, &p, &format!("quick_id=v1.0/01/01-02\n{why}"));
    assert_eq!(
        out, "Quick-task review scope is empty (no commit names this task).\n",
        "{err}"
    );
}

/// execute-phase's worktree handoff: the commits ahead of the base and the uncommitted paths, counted whole and
/// listed indented, at most 20 of them.
#[test]
fn the_worktree_handoff_counts_and_lists_what_is_left_indented() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = committed(&[
        ("feat: one", &[("a.txt", "1\n")]),
        ("feat: two", &[("b.txt", "2\n")]),
    ]);
    let base = p.git(&["rev-list", "--max-parents=0", "HEAD"], T1);
    for n in 1..=22 {
        p.write(&format!("u{n:02}.txt"), "left\n");
    }
    let text = asset("gsd-core/workflows/execute-phase.md");
    let code = [
        "_WT_DIRTY_ALL=$(",
        "[ -n \"$_WT_DIRTY_ALL\" ] && { _WT_DIRTY_N=",
        "printf '    %s\\n' \"$(specforge git log",
        "printf '    %s\\n' \"$(specforge git status",
    ]
    .map(|start| line(&text, start))
    .join("\n");
    let script = format!(
        "_WT_BASE={}\n{code}\nprintf '[%s]' \"$_WT_DIRTY_N\"",
        base.trim()
    );
    let (out, err, _) = run(&sh, &p, &script);
    let err_lines: Vec<&str> = err.lines().collect();
    assert_eq!(err_lines[..2], ["    feat: two", "    feat: one"], "{err}");
    assert_eq!(err_lines.len(), 22, "{err}");
    assert_eq!(err_lines[2], "    u01.txt", "{err}");
    assert!(
        err_lines[2..].iter().all(|l| l.starts_with("    u")),
        "{err}"
    );
    assert_eq!(out, "[22]");
}

/// onboard's entities are one `--fields kind,id` row per project, milestone, phase and requirement: a kind and an
/// id 0x1f-separated, from one export, where a loop over the kinds prefixed each id with `sed`.
#[test]
fn the_onboarding_entities_are_one_row_per_entity() {
    let Some(sh) = Shell::without_jq() else {
        return;
    };
    let p = taskboard();
    let read = line(&asset("gsd-core/workflows/onboard.md"), "ENTITIES=$(").to_string();
    let (out, err, _) = run(&sh, &p, &format!("{read}\nprintf '%s\\n' \"$ENTITIES\""));
    let rows: Vec<&str> = out.lines().collect();
    for want in [
        "project\u{1f}taskboard",
        "milestone\u{1f}v1.0",
        "phase\u{1f}v1.0/01",
        "phase\u{1f}v1.0/02.1",
    ] {
        assert!(rows.contains(&want), "{want:?} in {out}{err}");
    }
    assert!(
        rows.iter().any(|row| row.starts_with("requirement\u{1f}")),
        "{out}"
    );
    assert!(
        rows.iter().all(|row| row.split('\u{1f}').count() == 2),
        "{out}"
    );
}
