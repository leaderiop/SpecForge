//! What the host's E080 lets through (ADR 0297; REVIEW-r40 C1, M5): a judged rule's finding is never an error a
//! write adds, so a failing verdict replaces a pass, a judged target is created with its verdict owed, and an edit that
//! stales a passing verdict is written. The finding is reported, and every gate still fails on it.

use super::harness::{Project, Run, project};
use super::judged::{PLAN, record_all, state, status_of};
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
    assert_eq!(
        state(&p, "v1.0/01/01-01/TR1", "E501"),
        ("fresh".to_string(), false)
    );
}

#[test]
fn a_plan_check_pass_is_replaced_by_issues() {
    let p = project("plan");
    p.settings(json!({ "profile": "gsd-strict" }));
    record_all(&p, PLAN);
    assert_eq!(status_of(&p, PLAN), "checked");
    let run = p.record(PLAN, "W311", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    // Fail closed: the failing verdict is fresh and passes nothing, and the plan is no longer checked.
    assert_eq!(state(&p, PLAN, "W311"), ("fresh".to_string(), false));
    assert_ne!(status_of(&p, PLAN), "checked");
}

const AUDIT: &str = "v1.0/A1";

#[test]
fn e518_issues_records() {
    let p = project("verify");
    let run = p.record(AUDIT, "E518", "issues", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of(AUDIT)["integration"], "issues");
    assert_eq!(state(&p, AUDIT, "E518"), ("fresh".to_string(), false));
    assert_ne!(p.fields_of(AUDIT)["status"], "passed");
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
    let run = p.record(&req, "W203", "failed", &[]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(state(&p, &req, "W203"), ("fresh".to_string(), false));
}

/// M5: creating a judged target adds a `no verdict` finding, which is owed a verdict and refuses nothing.
// An artifact is not created here: `--path` is the CLI's own project option, so the CLI cannot spell its `path` field.
// Each creation, with the id it makes and the rule that owes it a verdict.
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
    // Each is owed a verdict and has none.
    for (id, rule) in [
        ("v1.0/01/01-02/TR9", "E501"),
        ("v1.0/01/01-02/KL9", "E501"),
        ("v1.0/A2", "E518"),
    ] {
        assert_eq!(state(&p, id, rule), ("missing".to_string(), false), "{id}");
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
    // Fail closed: the verdict is stale, not passing, and the verification no longer reads passed.
    assert_eq!(
        state(&p, "v1.0/01/01-01/TR1", "E501"),
        ("stale".to_string(), false)
    );
    assert_ne!(verification(&p)["status"], "passed");
    let p = project("verify");
    assert_eq!(p.record(AUDIT, "E518", "passed", &[]).code, 0);
    assert_eq!(
        p.run(&["set", "TASK-01", "text", "Add a task, fast"]).code,
        0
    );
    assert_eq!(state(&p, AUDIT, "E518"), ("stale".to_string(), false));
    assert_ne!(p.fields_of(AUDIT)["status"], "passed");
}
