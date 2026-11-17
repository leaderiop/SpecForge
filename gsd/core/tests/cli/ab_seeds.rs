//! The GSD A/B's seeded defects on the taskboard and ledger fixtures: each `b_signals` row of `evaluation/gsd-ab/tasks.json` is
//! what arm B's checker reports on a copy `seed.py` made, and B0 (no test facts) reports none of the thread's.

use super::harness::ab::{assert_signals, seeded};

#[test]
fn taskboard_execute_02_01_reports_its_seeds() {
    assert_signals("taskboard-execute-02-01");
}

#[test]
fn taskboard_verify_01_reports_its_seeds() {
    assert_signals("taskboard-verify-01");
}

#[test]
fn taskboard_progress_resume_reports_its_seeds() {
    assert_signals("taskboard-progress-resume");
}

#[test]
fn taskboard_ship_task_01_reports_its_seeds() {
    assert_signals("taskboard-ship-task-01");
}

/// B0 is B without the test facts, not without the route: on a seeded copy `trace` follows `@gsd/core`'s route from the
/// requirement to its plan, tasks, the seeded commit (linked by its subject's title), the plan's truth and the
/// requirement's coverage item, and says it skips the `item` step and the start's tagged-test annotation, whose kinds
/// no B0 extension provides. With no test fact the requirement's `proof` reads `not_run`.
#[test]
fn b0_traces_the_route_with_its_item_step_and_tagged_tests_skipped() {
    let p = seeded("taskboard-execute-02-01", "B0", &[]).expect("a B0 copy");
    p.collect_seeded("taskboard-execute-02-01", true);
    let run = p.run(&["trace", "TASK-01", "--partial"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let lines: Vec<&str> = run.stdout.lines().collect();
    let count = |text: &str| lines.iter().filter(|l| l.contains(text)).count();
    assert_eq!(lines.first(), Some(&"TASK-01 (not_run)"), "{}", run.all());
    assert_eq!(
        (
            count("→ plan 01-01"),
            count("→ T1"),
            count("→ T2"),
            count("(title)"),
            count("→ TR1"),
            count("→ deliverable D1 (test/tasks.test.js#a task is added with the next id)")
        ),
        (1, 1, 1, 1, 1, 1),
        "{}",
        run.all()
    );
    // The start, plan, T1 and its commit, T2 and its missing commit, the truth and the coverage item.
    assert_eq!(
        (count("js:"), lines.len()),
        (0, 8),
        "no item or tagged test is printed: {}",
        run.all()
    );
    let skipped: Vec<&str> = run.stderr.lines().filter(|l| l.contains("I210")).collect();
    assert_eq!(skipped.len(), 2, "{}", run.all());
    assert!(
        skipped[0].contains("annotation 'status' of the start")
            && skipped[1].contains("step 'item'"),
        "{skipped:?}"
    );
}

#[test]
fn ledger_execute_02_01_reports_its_seeds() {
    assert_signals("ledger-execute-02-01");
}

#[test]
fn ledger_verify_01_reports_its_seeds() {
    assert_signals("ledger-verify-01");
}

#[test]
fn ledger_progress_resume_reports_its_seeds() {
    assert_signals("ledger-progress-resume");
}

#[test]
fn ledger_ship_ledg_02_reports_its_seeds() {
    assert_signals("ledger-ship-ledg-02");
}
