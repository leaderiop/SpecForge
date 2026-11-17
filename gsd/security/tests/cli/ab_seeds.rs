//! The GSD A/B's seeded defects on the vault fixture: each `b_signals` row of `evaluation/gsd-ab/tasks.json` is
//! what arm B's checker reports on a copy `seed.py` made, and B0 (no test facts) reports none of the thread's.

use super::harness::ab::{ab_dir, ab_task, ab_tasks, assert_signals};

#[test]
fn vault_execute_01_02_reports_its_seeds() {
    assert_signals("vault-execute-01-02");
}

#[test]
fn vault_verify_01_reports_its_seeds() {
    assert_signals("vault-verify-01");
}

/// The vault twin has no summary coverage item, so no `gsd/W408` can name a failed or deleted test there: its verify
/// task seeds neither S5 nor S6 and says why (`not_seeded`).
#[test]
fn the_vault_twin_has_no_coverage_item_so_its_verify_task_seeds_no_test_defect() {
    let spec = ab_dir().join("../../gsd/parity/fixtures/vault/spec");
    let mut files = 0;
    for entry in std::fs::read_dir(&spec).expect("the vault's spec/") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|e| e == "spec") {
            let text = std::fs::read_to_string(&path).expect("a spec file");
            assert!(
                !text.lines().any(|l| l.starts_with("deliverable ")),
                "{} holds a deliverable",
                path.display()
            );
            files += 1;
        }
    }
    assert!(files > 0, "the vault's spec files were read");
    let tasks = ab_tasks();
    let task = ab_task(&tasks, "vault-verify-01");
    assert_eq!(task["seeded"].as_array().map(Vec::len), Some(0));
    let said: Vec<&str> = task["not_seeded"]
        .as_array()
        .expect("not_seeded")
        .iter()
        .filter_map(|e| e["defect"].as_str())
        .collect();
    assert_eq!(said, ["S5", "S6"]);
}

#[test]
fn vault_progress_resume_reports_its_seeds() {
    assert_signals("vault-progress-resume");
}

#[test]
fn vault_ship_01_reports_its_seeds() {
    assert_signals("vault-ship-01");
}
