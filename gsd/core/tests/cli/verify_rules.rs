//! The rules of the verify passes, each on a fixture that fires it and a counterpart that does not, on the `verify`
//! project. Every test asserts the whole list of codes.

use crate::harness::{Project, project};

fn verify() -> Project {
    project("verify")
}

/// Plan 01-02's task T1 loses its `verify`: a task with no automated check.
fn drop_verify(p: &Project) {
    p.edit(
        "spec/plans.spec",
        "  action \"Write src/complete.js with complete(id)\"\n  verify [\n    { automated \"node --test test/tasks.test.js\" fails_when \"the test run exits non-zero\" }\n    { automated \"node src/complete.js --check\" fails_when \"the script exits non-zero\" }\n  ]\n",
        "  action \"Write src/complete.js with complete(id)\"\n  verify_text \"Complete a task by hand\"\n",
    );
}

#[test]
fn a_task_without_verify_or_wave0_is_e314() {
    let p = verify();
    assert_eq!(p.codes(), Vec::<String>::new());
    drop_verify(&p);
    assert_eq!(p.codes(), ["gsd/E314"], "{:?}", p.report());
    let report = p.report();
    assert!(report[0].1.contains("v1.0/01/01-02/T1"), "{report:?}");
}

#[test]
fn a_wave0_item_for_the_requirement_satisfies_e314() {
    let p = verify();
    drop_verify(&p);
    // 01-02 serves TASK-02, so a wave-0 item for TASK-01 does not cover it ...
    assert_eq!(p.codes(), ["gsd/E314"], "{:?}", p.report());
    p.edit(
        "spec/validation.spec",
        "  for        TASK-01\n",
        "  for        TASK-02\n",
    );
    // ... and one for TASK-02 does.
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn a_phase_without_validation_is_not_judged_by_e314() {
    let p = verify();
    drop_verify(&p);
    p.write("spec/validation.spec", "");
    assert_eq!(p.run(&["harness", "gc"]).code, 0);
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

/// A research of phase 01 that asks for a validation, and a phase with none.
fn research_without_validation(p: &Project) {
    p.write("spec/validation.spec", "");
    assert_eq!(p.run(&["harness", "gc"]).code, 0);
    p.write(
        "spec/research.spec",
        "research 01-RESEARCH \"Core research\" {\n  phase      v1.0/01\n  researched 2026-10-28\n  confidence high\n  summary    \"## Validation Architecture: node:test, one file per module\"\n}\n",
    );
}

#[test]
fn a_phase_with_validation_architecture_and_no_validation_is_w510() {
    let p = verify();
    research_without_validation(&p);
    assert_eq!(p.codes(), ["gsd/W510"], "{:?}", p.report());
    assert!(
        p.report()[0].1.contains("no validation"),
        "{:?}",
        p.report()
    );
}

#[test]
fn a_research_without_the_section_is_not_w510() {
    let p = verify();
    research_without_validation(&p);
    p.edit("spec/research.spec", "## Validation Architecture: ", "");
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn a_verification_before_validated_is_w510() {
    let p = verify();
    p.edit(
        "spec/validation.spec",
        "  approval {\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
        "",
    );
    assert_eq!(p.run(&["harness", "gc"]).code, 0);
    assert_eq!(p.codes(), ["gsd/W510"], "{:?}", p.report());
    assert!(
        p.report()[0].1.contains("still a draft"),
        "{:?}",
        p.report()
    );
}

#[test]
fn nyquist_validation_off_silences_e314_and_w510() {
    let p = verify();
    drop_verify(&p);
    p.edit(
        "spec/validation.spec",
        "  approval {\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
        "",
    );
    assert_eq!(p.run(&["harness", "gc"]).code, 0);
    assert_eq!(p.codes(), ["gsd/E314", "gsd/W510"], "{:?}", p.report());
    p.settings(serde_json::json!({ "workflow.nyquist_validation": false }));
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}
