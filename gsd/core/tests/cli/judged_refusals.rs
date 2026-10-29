//! What the host's E080 refuses today (REVIEW-r40 C1, M5), pinned as live tests, each with an `#[ignore]`d twin that
//! asserts the behaviour wanted once the kernel stops treating a judged rule's own state as a new error (K17,
//! `crates/app/ops/src/authoring/edit/validate.rs`: a judged diagnostic is a verdict owed, not a defect of the edit).
//!
//! The gsd side cannot fix these: a rule's message changes with the verdict, E080 keys a diagnostic by its message
//! shape, and a recorded `passed` emits no diagnostic at all. A passing verdict is therefore never withdrawn through
//! `record_verdict`; only a changed input stales it, and a typed edit that would stale an Error rule's pass is refused too.

use super::harness::{Project, Run, project};
use super::verify_verdicts::{bare, verification, verified};
use serde_json::json;

const VERIFICATION_ID: &str = "v1.0/01/01-VERIFICATION";

const ITEMS: [&str; 6] = [
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
];

/// E080 refused it whole: a non-zero exit, the code and "nothing was written" in the answer, the file unchanged.
fn refused_whole(run: &Run, p: &Project, file: &str, before: &str) {
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("E080"), "{}", run.all());
    assert!(run.all().contains("nothing was written"), "{}", run.all());
    assert_eq!(p.read(file), before);
}

/// Phase 01 verified: six `verified` verdicts, status `passed`.
fn passed() -> Project {
    let p = bare();
    for item in ITEMS {
        verified(&p, item);
    }
    assert_eq!(verification(&p)["status"], "passed");
    p
}

#[test]
fn a_failing_verdict_cannot_replace_a_pass_for_now() {
    let p = passed();
    let before = p.read("spec/plans.spec");
    let run = p.record("v1.0/01/01-01/TR1", "E501", "failed", &[]);
    refused_whole(&run, &p, "spec/plans.spec", &before);
    // The regression is invisible: the old pass stays fresh, so the phase is still passed.
    assert_eq!(verification(&p)["status"], "passed");
    assert_eq!(p.fields_of(VERIFICATION_ID)["score"], "2/2");
}

#[test]
#[ignore = "K17: a verdict's own judged state is not a new error"]
fn a_failing_verdict_replaces_a_pass() {
    let p = passed();
    let run = p.record("v1.0/01/01-01/TR1", "E501", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(verification(&p)["status"], "gaps_found");
}

#[test]
fn a_plan_check_pass_cannot_be_replaced_by_issues_for_now() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    let run = p.record("v1.0/01/01-02", "W311", "passed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    let before = p.read("spec/plans.spec");
    let run = p.record("v1.0/01/01-02", "W311", "issues", &[]);
    refused_whole(&run, &p, "spec/plans.spec", &before);
}

#[test]
#[ignore = "K17: a verdict's own judged state is not a new error"]
fn a_plan_check_pass_is_replaced_by_issues() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    assert_eq!(p.record("v1.0/01/01-02", "W311", "passed", &[]).code, 0);
    let run = p.record("v1.0/01/01-02", "W311", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
}

const AUDIT: &str = "v1.0/A1";

#[test]
fn e518_issues_is_refused_for_now() {
    let p = project("verify");
    let before = p.read("spec/audit.spec");
    let run = p.record(AUDIT, "E518", "issues", &[]);
    refused_whole(&run, &p, "spec/audit.spec", &before);
}

#[test]
#[ignore = "K17: a verdict's own judged state is not a new error"]
fn e518_issues_records() {
    let p = project("verify");
    let run = p.record(AUDIT, "E518", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["integration"], "issues");
}

const SPEC: &str = "v1.0/02/02-SPEC";
const LOW: &str =
    r#"{"scores":{"goal":10,"boundary":5,"constraint":0,"acceptance":0,"ambiguity":95}}"#;

#[test]
fn e202_failed_is_refused_for_now() {
    let p = project("discuss");
    let before = p.read("spec/discuss.spec");
    let run = p.record(SPEC, "E202", "failed", &["--values", LOW]);
    refused_whole(&run, &p, "spec/discuss.spec", &before);
}

#[test]
#[ignore = "K17: a verdict's own judged state is not a new error"]
fn e202_failed_records() {
    let p = project("discuss");
    let run = p.record(SPEC, "E202", "failed", &["--values", LOW]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(SPEC)["scores"]["goal"], 10);
}

/// W203 `failed` is an Error under `gsd-strict` (W205 `suspicious` is proved in `discuss.rs`).
#[test]
fn w203_failed_is_refused_under_strict_for_now() {
    let p = project("discuss");
    p.settings(json!({ "profile": "gsd-strict" }));
    let req = p.nodes_of("locked_req")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    let before = p.read("spec/discuss.spec");
    let run = p.record(&req, "W203", "failed", &[]);
    refused_whole(&run, &p, "spec/discuss.spec", &before);
}

#[test]
#[ignore = "K17: a verdict's own judged state is not a new error"]
fn w203_failed_records_under_strict() {
    let p = project("discuss");
    p.settings(json!({ "profile": "gsd-strict" }));
    let req = p.nodes_of("locked_req")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    assert_eq!(p.record(&req, "W203", "failed", &[]).code, 0);
}

/// M5: creating a judged target adds a `no verdict` Error of the same shape, which E080 counts as new.
const CREATIONS: [(&str, &[&str]); 4] = [
    (
        "spec/plans.spec",
        &[
            "new",
            "truth",
            "TR9",
            "A new claim",
            "--plan",
            "v1.0/01/01-02",
            "--text",
            "A new claim",
        ],
    ),
    (
        "spec/plans.spec",
        &[
            "new",
            "artifact",
            "AR9",
            "Another store",
            "--plan",
            "v1.0/01/01-02",
            "--path",
            "src/tasks.js",
        ],
    ),
    (
        "spec/plans.spec",
        &[
            "new",
            "key_link",
            "KL9",
            "Another link",
            "--plan",
            "v1.0/01/01-02",
            "--from",
            "a.js",
            "--to",
            "b.js",
            "--via",
            "x",
        ],
    ),
    (
        "spec/audit.spec",
        &[
            "new",
            "milestone_audit",
            "A2",
            "Second audit",
            "--milestone",
            "v1.0",
        ],
    ),
];

fn create(p: &Project, file: &str, args: &[&str]) -> Run {
    let mut all = args.to_vec();
    all.extend(["--in", file]);
    p.run(&all)
}

#[test]
fn an_agent_cannot_create_a_judged_target_for_now() {
    let p = project("verify");
    for (file, args) in CREATIONS {
        let before = p.read(file);
        let run = create(&p, file, args);
        refused_whole(&run, &p, file, &before);
    }
    // A phase spec, in the project that has them.
    let p = project("discuss");
    let before = p.read("spec/discuss.spec");
    let run = p.run(&[
        "new",
        "phase_spec",
        "03-SPEC",
        "Third spec",
        "--phase",
        "v1.0/02",
        "--goal",
        "g",
        "--in",
        "spec/discuss.spec",
    ]);
    refused_whole(&run, &p, "spec/discuss.spec", &before);
}

#[test]
#[ignore = "K17: a judged target may be created; its verdict is owed"]
fn an_agent_creates_a_judged_target() {
    let p = project("verify");
    for (file, args) in CREATIONS {
        let run = create(&p, file, args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.all());
    }
}

#[test]
fn an_edit_that_stales_a_passing_error_verdict_is_refused_for_now() {
    let p = bare();
    verified(&p, "v1.0/01/01-01/TR1");
    let before = p.read("spec/plans.spec");
    let run = p.run(&[
        "set",
        "v1.0/01/01-01/TR1",
        "text",
        "A user can add a task and see it",
    ]);
    refused_whole(&run, &p, "spec/plans.spec", &before);
}

/// After E518 passes, every requirement edit is refused: the audit reads all of them (`requirements`).
#[test]
fn after_e518_passes_a_requirement_edit_is_refused_for_now() {
    let p = project("verify");
    assert_eq!(p.record(AUDIT, "E518", "passed", &[]).code, 0);
    let before = p.read("spec/project.spec");
    let run = p.run(&["set", "TASK-01", "text", "Add a task, fast"]);
    refused_whole(&run, &p, "spec/project.spec", &before);
}

#[test]
#[ignore = "K17: staling a verdict is the edit's effect, not its defect"]
fn an_edit_that_stales_a_pass_is_allowed() {
    let p = bare();
    verified(&p, "v1.0/01/01-01/TR1");
    let run = p.run(&[
        "set",
        "v1.0/01/01-01/TR1",
        "text",
        "A user can add a task and see it",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let p = project("verify");
    assert_eq!(p.record(AUDIT, "E518", "passed", &[]).code, 0);
    assert_eq!(
        p.run(&["set", "TASK-01", "text", "Add a task, fast"]).code,
        0
    );
}
