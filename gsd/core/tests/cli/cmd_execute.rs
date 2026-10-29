//! `start_plan` and `complete_plan` (D-r41-3): the times of a plan's summary, and the gates that fire at their entry.

use serde_json::json;

use super::commands_support::*;

const PLAN: &str = "v1.0/02/02-01";
const SUMMARY: &str = "v1.0/02/02-01/02-01-SUMMARY";
const PLANTED: &str = "summary 02-01-SUMMARY \"Save and load: summary\" {\n  plan                   v1.0/02/02-01\n  status                 complete\n  one_liner              \"Tasks are saved after every change.\"\n  subsystem              \"store\"\n  requirements_completed [STORE-01, STORE-02]\n}\n";
const STARTED: &str = "2026-10-29T09:00:00Z";

#[test]
fn complete_plan_writes_both_times_and_their_ledger_lines() {
    let p = twin();
    plant(&p, "spec/plans.spec", PLANTED);
    let status = p.fields_of("v1.0/02")["status"].clone();
    let before = p.codes();
    let run = gsd(&p, "complete-plan", &[SUMMARY, "--started", STARTED]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of(SUMMARY);
    assert_eq!(fields["started"], STARTED);
    let completed = fields["completed"].as_str().expect("completed").to_string();
    assert!(
        completed.len() == 20 && completed.ends_with('Z') && completed.as_str() > STARTED,
        "{completed}"
    );
    assert!(ledgered(&p, SUMMARY, "started", &format!("\"{STARTED}\"")));
    assert!(ledgered(
        &p,
        SUMMARY,
        "completed",
        &format!("\"{completed}\"")
    ));
    // No E080, no E083: the host accepted the harness values, and the check adds nothing.
    let after = p.codes();
    assert!(
        after.iter().all(|c| before.contains(c)),
        "{before:?} -> {after:?}"
    );
    assert_eq!(
        p.fields_of("v1.0/02")["status"],
        status,
        "the times move no status"
    );
}

#[test]
fn start_plan_answers_the_hosts_now() {
    let p = twin();
    let run = gsd(&p, "start-plan", &[PLAN, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = json(&run);
    let started = answer["started"].as_str().expect("started");
    assert_eq!(answer["plan"], PLAN);
    assert!(started.len() == 20 && started.ends_with('Z'), "{started}");
    // A short form is resolved; nothing is written.
    let before = bytes(&p);
    assert_eq!(gsd(&p, "start-plan", &["02-01"]).code, 0);
    assert_eq!(bytes(&p), before);
}

/// A stale plan-check verdict on plan 02-01 of the twin: recorded, then its input (the instructions) edited.
fn stale_plan_check(p: &crate::harness::Project) {
    p.write("CLAUDE.md", "# Rules\n\nNo task is longer than an hour.\n");
    for code in ["W311", "W313", "W315", "W316"] {
        let run = p.record(PLAN, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
    p.write(
        "CLAUDE.md",
        "# Rules\n\nNo task is longer than half an hour.\n",
    );
    assert_eq!(p.inputs(PLAN, "W311")["verdict"]["state"], "stale");
}

#[test]
fn start_plan_is_refused_by_a_stale_plan_check_under_strict() {
    let p = twin();
    stale_plan_check(&p);
    strict(&p);
    let before = bytes(&p);
    let run = gsd(&p, "start-plan", &[PLAN]);
    refused(&run, "gate_failed", &["W318", PLAN]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn start_plan_is_allowed_under_default() {
    let p = twin();
    stale_plan_check(&p);
    // W318 is a warning under gsd-compat: the gate lets the plan start.
    let run = gsd(&p, "start-plan", &[PLAN]);
    assert_eq!(run.code, 0, "{}", run.all());
    p.settings(json!({ "profile": "gsd-strict", "workflow.plan_check": false }));
    assert_eq!(
        gsd(&p, "start-plan", &[PLAN]).code,
        0,
        "a setting off never refuses"
    );
}

#[test]
fn start_plan_is_refused_by_a_stale_verdict_under_plus() {
    let p = twin();
    stale_plan_check(&p);
    plus(&p);
    let run = gsd(&p, "start-plan", &[PLAN]);
    refused(&run, "gate_failed", &["E516"]);
}

#[test]
fn complete_plan_is_refused_without_a_tdd_review_under_strict() {
    let p = twin();
    plant(&p, "spec/plans.spec", PLANTED);
    p.edit("spec/plans.spec", "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             execute", "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             tdd");
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": true }));
    let before = bytes(&p);
    let run = gsd(&p, "complete-plan", &[SUMMARY, "--started", STARTED]);
    refused(&run, "gate_failed", &["W404", PLAN]);
    assert_eq!(bytes(&p), before);
    // Allowed by default (W404 is a warning there) and with tdd_mode off.
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": false }));
    assert_eq!(
        gsd(
            &p,
            "complete-plan",
            &[SUMMARY, "--started", STARTED, "--dry-run"]
        )
        .code,
        0
    );
}

#[test]
fn complete_plan_refuses() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(
        &p,
        "complete-plan",
        &["v1.0/02/02-01/02-01-SUMMARY", "--started", STARTED],
    );
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(run.all().contains("not found"), "{}", run.all());
    plant(&p, "spec/plans.spec", PLANTED);
    let planted = bytes(&p);
    assert_ne!(planted, before);
    for (args, words) in [
        (vec![SUMMARY], vec!["--started is required"]),
        (vec![SUMMARY, "--started", "yesterday"], vec!["RFC 3339"]),
        (
            vec![SUMMARY, "--started", "2026-10-29T09:00:00+01:00"],
            vec!["RFC 3339"],
        ),
        (
            vec![SUMMARY, "--started", "2999-01-01T00:00:00Z"],
            vec!["after now"],
        ),
    ] {
        let run = gsd(&p, "complete-plan", &args);
        refused(&run, "invalid_input", &words);
        assert_eq!(bytes(&p), planted, "{args:?}");
    }
    assert_eq!(
        gsd(&p, "complete-plan", &[SUMMARY, "--started", STARTED]).code,
        0
    );
    let done = bytes(&p);
    let again = gsd(&p, "complete-plan", &[SUMMARY, "--started", STARTED]);
    refused(&again, "invalid_input", &["already completed"]);
    assert_eq!(bytes(&p), done);
}
