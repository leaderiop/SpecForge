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
/// requirement to its plan, tasks, the seeded commit (linked by its subject's title) and the truth, and says it skips
/// the `item` and `test` steps, whose kinds no B0 extension provides.
#[test]
fn b0_traces_the_route_with_its_item_and_test_steps_skipped() {
    let p = seeded("taskboard-execute-02-01", "B0", &[]).expect("a B0 copy");
    p.collect_seeded("taskboard-execute-02-01", true);
    let run = p.run(&["trace", "TASK-01", "--partial"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let lines: Vec<&str> = run.stdout.lines().collect();
    let count = |text: &str| lines.iter().filter(|l| l.contains(text)).count();
    assert_eq!(
        (
            count("→ plan 01-01"),
            count("→ T1"),
            count("→ T2"),
            count("(title)"),
            count("→ TR1")
        ),
        (1, 1, 1, 1, 1),
        "{}",
        run.all()
    );
    assert_eq!(
        (count("js:"), lines.len()),
        (0, 7),
        "no item or test step is printed: {}",
        run.all()
    );
    let skipped: Vec<&str> = run.stderr.lines().filter(|l| l.contains("I210")).collect();
    assert_eq!(skipped.len(), 2, "{}", run.all());
    assert!(
        skipped[0].contains("step 'item'") && skipped[1].contains("step 'test'"),
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
