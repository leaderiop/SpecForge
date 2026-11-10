//! The GSD A/B's seeded defects on the taskboard fixture: each `b_signals` row of `evaluation/gsd-ab/tasks.json` is
//! what arm B's checker reports on a copy `seed.py` made, and B0 (no test facts) reports none of the thread's.

use super::harness::ab::{Today, assert_signals};

/// `(task, defect, arm, today)`: the rows that do not yet give what `tasks.json` says, asserted as they are.
const KNOWN_GAPS: &[(&str, &str, &str, Today)] = &[
    // The seeded commit's subject names no plan, so `gsd-git` skips it; E521 is off under gsd-strict anyway.
    ("taskboard-execute-02-01", "S4", "B", Today::Counts(0, 0)),
    ("taskboard-execute-02-01", "S4", "B0", Today::Counts(0, 0)),
    // Phase 02 is planned, not executed: its truths' plan stays pending for the whole verify run.
    ("taskboard-verify-02", "S5", "B", Today::Counts(1, 1)),
    ("taskboard-verify-02", "S6", "B", Today::Counts(0, 0)),
];

fn gaps(task: &str) -> Vec<(&'static str, &'static str, Today)> {
    KNOWN_GAPS
        .iter()
        .filter(|(t, ..)| *t == task)
        .map(|(_, defect, arm, today)| (*defect, *arm, *today))
        .collect()
}

fn signals(task: &str) {
    assert_signals(task, &gaps(task));
}

#[test]
fn taskboard_execute_02_01_reports_its_seeds() {
    signals("taskboard-execute-02-01");
}

#[test]
fn taskboard_verify_02_reports_its_seeds() {
    signals("taskboard-verify-02");
}

#[test]
fn taskboard_progress_resume_reports_its_seeds() {
    signals("taskboard-progress-resume");
}

#[test]
fn taskboard_ship_task_01_reports_its_seeds() {
    signals("taskboard-ship-task-01");
}
