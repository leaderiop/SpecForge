//! A plan's strict wave, end to end on the `plan` project: where the plan runs once the plans of its phase that share
//! a path are kept apart (ADR 0378 D1). `wave` is gsd-core's and never moves.

use crate::harness::{Project, project};
use crate::plan_rules::{auto, third_plan, with};

/// `(wave, strict_wave)` of plan `id`, `None` for a field the plan does not have.
fn placed(p: &Project, id: &str) -> (Option<i64>, Option<i64>) {
    let fields = p.fields_of(id);
    (fields["wave"].as_i64(), fields["strict_wave"].as_i64())
}

#[test]
fn the_plan_project_runs_every_plan_in_its_wave() {
    let p = project("plan");
    assert_eq!(placed(&p, "v1.0/01/01-01"), (Some(1), Some(1)));
    assert_eq!(placed(&p, "v1.0/01/01-02"), (Some(2), Some(2)));
    assert_eq!(placed(&p, "v1.0/02/02-01"), (Some(1), Some(1)));
    assert_eq!(placed(&p, "261028-a1b"), (None, None), "a quick plan");
}

#[test]
fn a_same_wave_overlap_places_the_later_plan_one_wave_on() {
    // 01-03 is in wave 1 beside 01-01 and modifies its `src/tasks.js`.
    let p = with(&third_plan("", "\"src/tasks.js\"", ""));
    assert_eq!(placed(&p, "v1.0/01/01-01"), (Some(1), Some(1)));
    assert_eq!(placed(&p, "v1.0/01/01-03"), (Some(1), Some(2)));
    // 01-02 depends on 01-01 only: nothing it shares pushes it.
    assert_eq!(placed(&p, "v1.0/01/01-02"), (Some(2), Some(2)));
}

#[test]
fn a_task_sharing_a_path_pushes_its_plan() {
    // 01-03's own lists name `src/third.js`; its task names 01-01's `src/tasks.js` (gsd/W306 says it is not 01-03's).
    let p = with(&format!(
        "{}{}",
        third_plan("", "\"src/third.js\"", ""),
        auto("T1", "v1.0/01/01-03")
    ));
    assert_eq!(placed(&p, "v1.0/01/01-03"), (Some(1), Some(2)));
}

#[test]
fn a_cycle_leaves_the_phase_without_strict_waves() {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
        "  depends_on     [01-02]\n  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
    );
    assert_eq!(placed(&p, "v1.0/01/01-01"), (None, None));
    assert_eq!(placed(&p, "v1.0/01/01-02"), (None, None));
    assert_eq!(placed(&p, "v1.0/02/02-01"), (Some(1), Some(1)));
}
