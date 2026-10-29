//! What the host's E080 lets through (ADR 0297; REVIEW-r40 C1, M5): a judged rule's finding is never an error a
//! write adds, so a failing verdict replaces a pass, a judged target is created with its verdict owed, and an edit that
//! stales a passing verdict is written. The finding is reported, and every gate still fails on it.

use super::harness::{Project, Run, project};
use super::verify_verdicts::{bare, verification, verified};
use serde_json::json;

const ITEMS: [&str; 6] = [
    "v1.0/01/01-01/TR1",
    "v1.0/01/01-01/AR1",
    "v1.0/01/01-01/KL1",
    "v1.0/01/01-02/TR1",
    "v1.0/01/01-02/AR1",
    "v1.0/01/01-02/KL1",
];

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
fn a_failing_verdict_replaces_a_pass() {
    let p = passed();
    let run = p.record("v1.0/01/01-01/TR1", "E501", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(verification(&p)["status"], "gaps_found");
}

#[test]
fn a_plan_check_pass_is_replaced_by_issues() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    assert_eq!(p.record("v1.0/01/01-02", "W311", "passed", &[]).code, 0);
    let run = p.record("v1.0/01/01-02", "W311", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
}

const AUDIT: &str = "v1.0/A1";

#[test]
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
fn e202_failed_records() {
    let p = project("discuss");
    let run = p.record(SPEC, "E202", "failed", &["--values", LOW]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(SPEC)["scores"]["goal"], 10);
}

#[test]
fn w203_failed_records_under_strict() {
    let p = project("discuss");
    p.settings(json!({ "profile": "gsd-strict" }));
    let req = p.nodes_of("locked_req")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    assert_eq!(p.record(&req, "W203", "failed", &[]).code, 0);
}

/// M5: creating a judged target adds a `no verdict` finding, which is owed a verdict and refuses nothing.
// An artifact is not created here: `--path` is the CLI's own project option, so the CLI cannot spell its `path` field.
const CREATIONS: [(&str, &[&str]); 3] = [
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
fn an_agent_creates_a_judged_target() {
    let p = project("verify");
    for (file, args) in CREATIONS {
        let run = create(&p, file, args);
        assert_eq!(run.code, 0, "{args:?}: {}", run.all());
    }
}

#[test]
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
