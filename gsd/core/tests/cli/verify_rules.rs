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

fn override_block(p: &Project) -> String {
    let text = p.read("spec/verify.spec");
    let at = text.find("override OV1").expect("the override");
    let end = text[at..].find("\n}\n").expect("its end") + at + 3;
    text[at..end].to_string()
}

#[test]
fn an_override_without_by_is_e503() {
    let p = verify();
    assert_eq!(p.codes(), Vec::<String>::new());
    p.edit("spec/verify.spec", "  by           \"ada\"\n", "");
    assert_eq!(p.codes(), ["gsd/E503"], "{:?}", p.report());
    assert!(p.report()[0].1.contains("no by:"), "{:?}", p.report());
    p.edit("spec/verify.spec", "  at           2026-10-29\n", "");
    assert!(p.report()[0].1.contains("no by and at"), "{:?}", p.report());
    assert_eq!(p.codes(), ["gsd/E503"]);
}

#[test]
fn four_overrides_is_w504() {
    let p = verify();
    let one = override_block(&p);
    let mut text = p.read("spec/verify.spec");
    for n in 2..=3 {
        text.push_str(&format!("\n{}", one.replace("OV1", &format!("OV{n}"))));
    }
    p.write("spec/verify.spec", &text);
    assert_eq!(
        p.codes(),
        Vec::<String>::new(),
        "three overrides are allowed: {:?}",
        p.report()
    );
    text.push_str(&format!("\n{}", one.replace("OV1", "OV4")));
    p.write("spec/verify.spec", &text);
    assert_eq!(p.codes(), ["gsd/W504"], "{:?}", p.report());
}

/// Records the answer to UAT test `name`, with `extra` lines (a reason, a report) beside it.
fn answer(p: &Project, name: &str, criterion: u8, result: &str, extra: &str) {
    p.edit(
        "spec/uat.spec",
        &format!("  criterion {criterion}\n}}"),
        &format!("  criterion {criterion}\n  result {result}\n{extra}}}"),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    p.record_harness(
        &format!("v1.0/01/01-UAT/{name}"),
        "result",
        &format!("\"{result}\""),
    );
}

#[test]
fn a_skip_without_reason_is_e509() {
    let p = verify();
    answer(&p, "U1", 1, "skipped", "");
    assert_eq!(p.codes(), ["gsd/E509"], "{:?}", p.report());
    let p = verify();
    answer(
        &p,
        "U1",
        1,
        "skipped",
        "  reason \"Deferred follow-up: needs a device\"\n",
    );
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn an_issue_without_a_report_is_e509() {
    let p = verify();
    answer(&p, "U2", 2, "issue", "");
    assert_eq!(p.codes(), ["gsd/E509"], "{:?}", p.report());
    let p = verify();
    answer(&p, "U2", 2, "issue", "  reported \"The list ignores it\"\n");
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn each_verify_code_fires_on_its_fixture() {
    // E503, W504 and E509 are the tests above; E505 is in `scan`, E314 and W510 in this file.
    let p = verify();
    p.edit("spec/verify.spec", "  by           \"ada\"\n", "");
    assert_eq!(p.codes(), ["gsd/E503"]);
}

#[test]
fn each_verify_code_is_silent_on_its_counterpart() {
    let p = verify();
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

const WAIVER: &str = r#"{"at":"2026-10-29T11:00:00Z","by":"ada","reason":"Tracked as issue 42"}"#;
const FIXED: &str = "2026-10-29T12:00:00Z";

/// Window BW1 resolved: waived, fixed, or both, each in the file and blessed in the ledger.
fn resolve(p: &Project, waived: bool, fixed: bool) {
    let mut lines = String::new();
    if waived {
        lines += "  waiver {\n    reason \"Tracked as issue 42\"\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n";
    }
    if fixed {
        lines += &format!("  fixed_at \"{FIXED}\"\n");
    }
    p.edit(
        "spec/ops/windows.spec",
        "  description \"A TODO remains in the store\"\n",
        &format!("  description \"A TODO remains in the store\"\n{lines}"),
    );
    if waived {
        p.record_harness("BW1", "waiver", WAIVER);
    }
    if fixed {
        p.record_harness("BW1", "fixed_at", &format!("\"{FIXED}\""));
    }
}

#[test]
fn waived_and_fixed_is_e513() {
    let p = verify();
    resolve(&p, true, true);
    assert_eq!(p.codes(), ["gsd/E513"], "{:?}", p.report());
    assert!(p.report()[0].1.contains("BW1"), "{:?}", p.report());
}

#[test]
fn waived_only_is_silent() {
    let p = verify();
    resolve(&p, true, false);
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

#[test]
fn fixed_only_is_silent() {
    let p = verify();
    resolve(&p, false, true);
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}
