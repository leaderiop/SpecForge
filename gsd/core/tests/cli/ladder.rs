//! The phase ladder and the project's window counts, end to end on the `verify` project. A phase is `complete` only
//! on a passed verification, which needs r40's verdicts, so here it stays `executed`.

use crate::harness::{Project, project};

fn verify() -> Project {
    project("verify")
}

#[test]
fn windows_are_counted_by_status() {
    let p = verify();
    let count = |p: &Project| {
        let found = p.fields_of("taskboard");
        [
            "windows_open",
            "windows_waived",
            "windows_fixed",
            "windows_total",
        ]
        .map(|f| found[f].clone())
    };
    assert_eq!(count(&p), [1, 0, 0, 1].map(serde_json::Value::from));
    p.edit(
        "spec/ops/windows.spec",
        "  description \"A TODO remains in the store\"\n",
        "  description \"A TODO remains in the store\"\n  waiver {\n    reason \"Tracked as issue 42\"\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
    );
    p.record_harness(
        "BW1",
        "waiver",
        r#"{"at":"2026-10-29T11:00:00Z","by":"ada","reason":"Tracked as issue 42"}"#,
    );
    assert_eq!(count(&p), [0, 1, 0, 1].map(serde_json::Value::from));
}

#[test]
fn an_unverified_phase_stays_executed() {
    let p = verify();
    // Nothing is left for a person to check (the fixture's human item is removed), and both plans of phase 01 have a
    // summary, so the ladder stops at the verification, which is pending without verdicts.
    let text = p.read("spec/verify.spec");
    let at = text.find("override OV1").expect("the override");
    p.write("spec/verify.spec", &format!("{}\n", text[..at].trim_end()));
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{text}\nsummary 01-02-SUMMARY \"Completion summary\" {{\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks can be completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}}\n"),
    );
    assert_eq!(p.fields_of("v1.0/01")["status"], "executed");
    assert_eq!(p.fields_of("v1.0")["progress"]["phases_complete"], 0);
}

#[test]
fn an_open_human_item_makes_the_phase_need_review() {
    let p = verify();
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{text}\nsummary 01-02-SUMMARY \"Completion summary\" {{\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks can be completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}}\n"),
    );
    assert_eq!(p.fields_of("v1.0/01")["status"], "needs_review");
}
