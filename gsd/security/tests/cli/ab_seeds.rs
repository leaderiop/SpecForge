//! The GSD A/B's seeded defects on the vault fixture: each `b_signals` row of `evaluation/gsd-ab/tasks.json` is
//! what arm B's checker reports on a copy `seed.py` made, and B0 (no test facts) reports none of the thread's.

use super::harness::ab::assert_signals;

fn signals(task: &str) {
    assert_signals(task);
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
