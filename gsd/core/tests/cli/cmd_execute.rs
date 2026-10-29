//! `start_plan` and `complete_plan` (D-r41-3): the times of a plan's summary, and the gates that fire at their entry.

use serde_json::json;

use super::commands_support::*;

pub(super) const PLAN: &str = "v1.0/02/02-01";
pub(super) const SUMMARY: &str = "v1.0/02/02-01/02-01-SUMMARY";
pub(super) const PLANTED: &str = "summary 02-01-SUMMARY \"Save and load: summary\" {\n  plan                   v1.0/02/02-01\n  status                 complete\n  one_liner              \"Tasks are saved after every change.\"\n  subsystem              \"store\"\n  requirements_completed [STORE-01, STORE-02]\n}\n";
pub(super) const STARTED: &str = "2026-10-29T09:00:00Z";

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
pub(super) fn stale_plan_check(p: &crate::harness::Project) {
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

// -- answer_checkpoint and confirm_setup (D-r41-4, D-r41-5, D-r41-6) -----------------------------------------------

pub(super) const CHECKPOINT: &str = "v1.0/01/01-02/C1";
const SECOND_PLAN: &str = "v1.0/01/01-02";
pub(super) const SETUP: &str = "v1.0/02/02-01/US1";

/// Plays the plan checker on the second plan of the `execute` project: its four plan-check verdicts are fresh.
fn checked(p: &crate::harness::Project) {
    for code in ["W311", "W313", "W315", "W316"] {
        let run = p.record(SECOND_PLAN, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
}

#[test]
fn answer_checkpoint_writes_option_text_by_and_at() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(p.inputs(SECOND_PLAN, "W311")["verdict"]["state"], "fresh");
    let ledger_before = p.read("specforge-harness");
    let run = gsd(
        &p,
        "answer-checkpoint",
        &[
            CHECKPOINT,
            "--option",
            "timestamp",
            "--text",
            "keep the time",
            "--by",
            "ada",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = &p.fields_of(CHECKPOINT)["answer"];
    assert_eq!(answer["option"], "timestamp");
    assert_eq!(answer["text"], "keep the time");
    assert_eq!(answer["by"], "ada");
    let at = answer["at"].as_str().expect("at");
    assert!(at.len() == 20 && at.ends_with('Z'), "{at}");
    // The ledger holds the value (no E083), and the check adds no error of its own.
    assert_ne!(p.read("specforge-harness"), ledger_before);
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    // Named staleness (D-r41-6): the answer is a plan-check input, so the plan's verdicts are stale.
    assert_eq!(p.inputs(SECOND_PLAN, "W311")["verdict"]["state"], "stale");
}

#[test]
fn an_option_outside_the_checkpoints_options_is_refused() {
    let p = super::harness::project("execute");
    let before = bytes(&p);
    let run = gsd(
        &p,
        "answer-checkpoint",
        &[CHECKPOINT, "--option", "maybe", "--by", "ada"],
    );
    refused(&run, "invalid_input", &["maybe", "flag, timestamp"]);
    let run = gsd(&p, "answer-checkpoint", &[CHECKPOINT, "--by", "ada"]);
    refused(&run, "invalid_input", &["--option or --text"]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn an_answered_checkpoint_is_refused() {
    let p = super::harness::project("execute");
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    let before = bytes(&p);
    let run = gsd(
        &p,
        "answer-checkpoint",
        &[CHECKPOINT, "--option", "timestamp", "--by", "ada"],
    );
    refused(&run, "invalid_input", &["already answered"]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn a_control_character_in_text_is_refused() {
    let p = super::harness::project("execute");
    let before = bytes(&p);
    let run = gsd(
        &p,
        "answer-checkpoint",
        &[CHECKPOINT, "--text", "a\u{1b}[31mred", "--by", "ada"],
    );
    refused(
        &run,
        "invalid_input",
        &["'text' holds a control character (U+001B)"],
    );
    assert_eq!(bytes(&p), before);
}

#[test]
fn confirm_setup_writes_done() {
    let p = super::harness::project("execute");
    let run = gsd(&p, "confirm-setup", &[SETUP, "--by", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let done = &p.fields_of(SETUP)["done"];
    assert_eq!(done["by"], "ada");
    assert!(
        done["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{done}"
    );
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    let again = gsd(&p, "confirm-setup", &[SETUP, "--by", "ada"]);
    refused(&again, "invalid_input", &["already confirmed"]);
}

const SECOND_SUMMARY: &str = "summary 01-02-SUMMARY \"Completion: summary\" {\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Completion\"\n  subsystem              \"tasks\"\n  requirements_completed [TASK-02]\n}\n";

/// An answered checkpoint stales the plan check; once the plan is executed (its summary is `completed`) that is
/// history, and a gate lets it by.
#[test]
fn an_executed_plans_stale_plan_check_does_not_refuse_a_gate() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    plus(&p);
    let run = gsd(&p, "start-plan", &[SECOND_PLAN]);
    refused(&run, "gate_failed", &["E516", SECOND_PLAN]);
    plant(&p, "spec/plans.spec", SECOND_SUMMARY);
    let summary = format!("{SECOND_PLAN}/01-02-SUMMARY");
    let run = gsd(&p, "complete-plan", &[&summary, "--started", STARTED]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        p.inputs(SECOND_PLAN, "W311")["verdict"]["state"],
        "stale",
        "still stale in check"
    );
    let run = gsd(&p, "start-plan", &[SECOND_PLAN]);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// A summary created before the run is not an execution: the entry gate still refuses (U33).
#[test]
fn starting_a_plan_with_an_uncompleted_summary_is_still_refused() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    plant(&p, "spec/plans.spec", SECOND_SUMMARY);
    plus(&p);
    let before = bytes(&p);
    let run = gsd(&p, "start-plan", &[SECOND_PLAN]);
    refused(&run, "gate_failed", &["E516", SECOND_PLAN]);
    assert_eq!(bytes(&p), before);
    strict(&p);
    refused(
        &gsd(&p, "start-plan", &[SECOND_PLAN]),
        "gate_failed",
        &["W318"],
    );
}

#[test]
fn an_unexecuted_plans_stale_plan_check_still_refuses() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    strict(&p);
    let run = gsd(&p, "start-plan", &[SECOND_PLAN]);
    refused(&run, "gate_failed", &["W318", SECOND_PLAN]);
}

/// The executor body's `specforge new` examples are accepted by the host (the body test checks their words).
#[test]
fn the_executors_new_examples_run() {
    let p = super::harness::project("execute");
    let summary = p.run(&[
        "new",
        "summary",
        "01-02-SUMMARY",
        "Completion",
        "--in",
        "spec/plans.spec",
        "--plan",
        "v1.0/01/01-02",
        "--status",
        "complete",
        "--one_liner",
        "Done",
        "--subsystem",
        "tasks",
        "--requirements_completed",
        "TASK-02",
        "--dry-run",
    ]);
    assert_eq!(summary.code, 0, "{}", summary.all());
    let decision = p.run(&[
        "new",
        "decision",
        "D-09",
        "Keep it small",
        "--in",
        "spec/plans.spec",
        "--phase",
        "v1.0/01",
        "--text",
        "Keep it small",
        "--origin",
        "summary",
        "--dry-run",
    ]);
    assert_eq!(decision.code, 0, "{}", decision.all());
}

#[test]
fn a_started_time_older_than_a_week_is_refused() {
    let p = twin();
    plant(&p, "spec/plans.spec", PLANTED);
    let before = bytes(&p);
    let run = gsd(
        &p,
        "complete-plan",
        &[SUMMARY, "--started", "1970-01-01T00:00:00Z"],
    );
    refused(&run, "invalid_input", &["over a week before now"]);
    unchanged(&p, &before);
}

/// Pins today (M2 of the r41 review): at a command gate the host keeps only the diagnostics about entities within
/// the subject's scope, so E516 is subject-scoped there, not project-wide as `rules.md` reads. A stale verdict of
/// another plan does not refuse `start-plan`.
#[test]
fn a_stale_verdict_outside_the_subject_does_not_refuse_start_plan_yet() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    assert_eq!(p.inputs(SECOND_PLAN, "W311")["verdict"]["state"], "stale");
    plus(&p);
    let run = gsd(&p, "start-plan", &["v1.0/01/01-01"]);
    assert_eq!(run.code, 0, "{}", run.all());
}

/// The desired behaviour (K19): E516 names the gate's subject, so a stale verdict anywhere refuses the step.
#[test]
#[ignore = "K19: project-wide E516 at a command gate"]
fn a_stale_verdict_anywhere_refuses_start_plan_under_plus() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    plus(&p);
    refused(
        &gsd(&p, "start-plan", &["v1.0/01/01-01"]),
        "gate_failed",
        &["E516"],
    );
}
