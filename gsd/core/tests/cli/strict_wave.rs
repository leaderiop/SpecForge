//! A plan's strict wave, end to end on the `plan` project: where the plan runs once the plans of its phase that share
//! a path are kept apart (ADR 0378 D1), and `gsd/W526`, which says so on each pushed plan (D2). `wave` is gsd-core's
//! and never moves.

use crate::harness::{Project, project};
use crate::plan_rules::{auto, third_plan, with};

/// `(wave, strict_wave)` of plan `id`, `None` for a field the plan does not have.
fn placed(p: &Project, id: &str) -> (Option<i64>, Option<i64>) {
    let fields = p.fields_of(id);
    (fields["wave"].as_i64(), fields["strict_wave"].as_i64())
}

/// The messages of the `gsd/W526` findings, sorted.
fn w526(p: &Project) -> Vec<String> {
    p.findings()
        .into_iter()
        .filter(|(code, _)| code == "gsd/W526")
        .map(|(_, message)| message)
        .collect()
}

/// A fourth plan of phase 01 with the must-haves W309 wants, modifying `modified`.
fn fourth_plan(depends: &str, modified: &str) -> String {
    format!(
        "plan 01-04 \"Fourth\" {{\n  phase          v1.0/01\n  type           execute\n{depends}  files_modified [{modified}]\n  requirements   [TASK-01]\n  objective      \"A fourth plan\"\n}}\n\ntruth TR1 \"Fourth\" {{\n  plan v1.0/01/01-04\n  text \"Fourth\"\n}}\n\nartifact AR1 \"Fourth\" {{\n  plan     v1.0/01/01-04\n  path     \"src/fourth.js\"\n  provides \"fourth\"\n}}\n"
    )
}

/// The cycle of `plan_rules::plan_cycle_is_e102`: 01-01 and 01-02 depend on each other.
fn with_a_cycle() -> Project {
    let p = project("plan");
    p.edit(
        "spec/plans.spec",
        "  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
        "  depends_on     [01-02]\n  files_modified [\"src/tasks.js\", \"test/tasks.test.js\"]\n",
    );
    p
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
fn a_same_wave_overlap_pushes_the_later_plan_w526() {
    let p = with(&third_plan("", "\"src/tasks.js\"", ""));
    assert_eq!(p.codes(), ["gsd/W305", "gsd/W526"]);
    assert_eq!(
        w526(&p),
        [
            "plan 'v1.0/01/01-03' shares 'src/tasks.js' with 'v1.0/01/01-01' (tasks: v1.0/01/01-01/T1): it runs in \
             strict wave 2, after 'v1.0/01/01-01' in strict wave 1, not in its wave 1"
        ]
    );
}

#[test]
fn a_plan_behind_a_pushed_plan_is_w526_naming_it() {
    // 01-04 has a file of its own and depends on 01-03, which 01-01 pushes to strict wave 2.
    let p = with(&format!(
        "{}{}",
        third_plan("", "\"src/tasks.js\"", ""),
        fourth_plan("  depends_on     [01-03]\n", "\"src/fourth.js\"")
    ));
    assert_eq!(placed(&p, "v1.0/01/01-04"), (Some(2), Some(3)));
    assert_eq!(p.codes(), ["gsd/W305", "gsd/W526", "gsd/W526"]);
    assert!(w526(&p).contains(
        &"plan 'v1.0/01/01-04' depends on 'v1.0/01/01-03', which runs in strict wave 2: it runs in strict wave 3, \
          not in its wave 2"
            .to_string()
    ));
}

#[test]
fn a_deleted_path_pushes_like_a_modified_one() {
    let p = with(&third_plan("", "\"src/third.js\"", "\"src/tasks.js\""));
    // A pending plan deletes a file that is there (E016 otherwise, ADR 0350).
    p.write("src/tasks.js", "// to delete\n");
    assert_eq!(placed(&p, "v1.0/01/01-03"), (Some(1), Some(2)));
    assert_eq!(p.codes(), ["gsd/W305", "gsd/W526"]);
    assert!(w526(&p)[0].contains("shares 'src/tasks.js' with 'v1.0/01/01-01'"));
}

#[test]
fn a_later_wave_plan_sharing_a_path_with_a_pushed_plan_is_pushed() {
    // 01-03 (wave 1) shares `src/tasks.js` with 01-01 and runs in strict wave 2; 01-04 (wave 2, on 01-01) shares
    // `src/y.js` with 01-03, so it runs after it. Their declared waves differ: W305 says nothing of 01-04.
    let p = with(&format!(
        "{}{}",
        third_plan("", "\"src/tasks.js\", \"src/y.js\"", ""),
        fourth_plan("  depends_on     [01-01]\n", "\"src/y.js\"")
    ));
    assert_eq!(placed(&p, "v1.0/01/01-04"), (Some(2), Some(3)));
    assert_eq!(p.codes(), ["gsd/W305", "gsd/W526", "gsd/W526"]);
    assert!(
        p.findings()
            .iter()
            .all(|(code, message)| code != "gsd/W305" || !message.contains("01-04")),
        "{:#?}",
        p.findings()
    );
    assert!(w526(&p).contains(
        &"plan 'v1.0/01/01-04' shares 'src/y.js' with 'v1.0/01/01-03' (no task names them): it runs in strict \
          wave 3, after 'v1.0/01/01-03' in strict wave 2, not in its wave 2"
            .to_string()
    ));
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
    assert_eq!(p.codes(), ["gsd/W306", "gsd/W526"]);
    assert_eq!(
        w526(&p),
        [
            "plan 'v1.0/01/01-03' shares 'src/tasks.js' with 'v1.0/01/01-01' (tasks: v1.0/01/01-01/T1, \
             v1.0/01/01-03/T1): it runs in strict wave 2, after 'v1.0/01/01-01' in strict wave 1, not in its wave 1"
        ]
    );
}

#[test]
fn plans_of_different_waves_sharing_a_path_are_not_pushed() {
    // 01-03 depends on 01-01, whose `src/tasks.js` it modifies: no gsd/W526, and it runs in its wave.
    let p = with(&third_plan(
        "  depends_on     [01-01]\n",
        "\"src/tasks.js\"",
        "",
    ));
    assert_eq!(placed(&p, "v1.0/01/01-03"), (Some(2), Some(2)));
    assert!(!p.codes().contains(&"gsd/W526".to_string()));
    assert_eq!(p.codes(), Vec::<String>::new());
}

#[test]
fn a_cycle_leaves_the_phase_without_strict_waves() {
    let p = with_a_cycle();
    assert_eq!(placed(&p, "v1.0/01/01-01"), (None, None));
    assert_eq!(placed(&p, "v1.0/01/01-02"), (None, None));
    assert_eq!(placed(&p, "v1.0/02/02-01"), (Some(1), Some(1)));
}

#[test]
fn a_cycle_raises_no_w526() {
    assert_eq!(with_a_cycle().codes(), ["W061", "gsd/E102", "gsd/E102"]);
}

#[test]
fn w526_is_a_warning_under_gsd_strict_where_w305_is_an_error() {
    let p = with(&third_plan("", "\"src/tasks.js\"", ""));
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let run = p.run(&["check", "--format", "json"]);
    let found: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("check prints JSON ({e}): {}", run.all()));
    let severity = |code: &str| -> String {
        found
            .as_array()
            .expect("a list")
            .iter()
            .find(|d| d["code"] == code)
            .unwrap_or_else(|| panic!("no {code}: {found:#}"))["severity"]
            .to_string()
    };
    assert_eq!(severity("gsd/W305"), "\"Error\"", "{found:#}");
    assert_eq!(severity("gsd/W526"), "\"Warning\"", "{found:#}");
}
