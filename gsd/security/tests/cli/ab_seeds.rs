//! The GSD A/B's seeded defects on the vault fixture: each `b_signals` row of `evaluation/gsd-ab/tasks.json` is
//! what arm B's checker reports on a copy `seed.py` made, and B0 (no test facts) reports none of the thread's.

use super::harness::ab::{Today, assert_signals};

/// `(task, defect, arm, today)`: the rows that do not yet give what `tasks.json` says, asserted as they are.
const KNOWN_GAPS: &[(&str, &str, &str, Today)] = &[];

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
fn vault_execute_01_02_reports_its_seeds() {
    signals("vault-execute-01-02");
}

#[test]
fn vault_verify_01_reports_its_seeds() {
    signals("vault-verify-01");
}

#[test]
fn vault_progress_resume_reports_its_seeds() {
    signals("vault-progress-resume");
}

#[test]
fn vault_ship_01_reports_its_seeds() {
    signals("vault-ship-01");
}
