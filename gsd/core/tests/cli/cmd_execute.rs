//! `start_plan` and `complete_plan` (D-r41-3): the times of a plan's summary, and the gates that fire at their entry.

use serde_json::json;

use super::commands_support::*;

pub(super) const PLAN: &str = "v1.0/02/02-01";
pub(super) const SUMMARY: &str = "v1.0/02/02-01/02-01-SUMMARY";
pub(super) const PLANTED: &str = "summary 02-01-SUMMARY \"Save and load: summary\" {\n  plan                   v1.0/02/02-01\n  status                 complete\n  one_liner              \"Tasks are saved after every change.\"\n  subsystem              \"store\"\n  requirements_completed [STORE-01, STORE-02]\n}\n";

/// Opens `plan`, as a run does before its summary exists; the time it recorded.
pub(super) fn opened(p: &crate::harness::Project, plan: &str) -> String {
    let run = gsd(p, "start-plan", &[plan, "--pick", ".started"]);
    assert_eq!(run.code, 0, "{}", run.all());
    run.stdout.trim_end().to_string()
}

#[test]
fn complete_plan_writes_both_times_and_their_ledger_lines() {
    let p = twin();
    let started = opened(&p, PLAN);
    let started = started.as_str();
    plant(&p, "spec/plans.spec", PLANTED);
    let status = p.fields_of("v1.0/02")["status"].clone();
    let before = p.codes();
    let run = gsd(&p, "complete-plan", &[SUMMARY]);
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of(SUMMARY);
    assert_eq!(fields["started"], started);
    let completed = fields["completed"].as_str().expect("completed").to_string();
    assert!(
        completed.len() == 20 && completed.ends_with('Z') && completed.as_str() >= started,
        "{completed}"
    );
    assert!(ledgered(&p, SUMMARY, "started", &format!("\"{started}\"")));
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
fn start_plan_records_the_hosts_now_once() {
    let p = twin();
    let started = opened(&p, PLAN);
    assert!(started.len() == 20 && started.ends_with('Z'), "{started}");
    let start = format!("{PLAN}/START");
    assert_eq!(p.fields_of(&start)["at"], started.as_str());
    assert!(ledgered(&p, &start, "at", &format!("\"{started}\"")));
    // A second call, a second later and by a short form, answers the first value and leaves it.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let before = p.fields_of(&start);
    let run = gsd(&p, "start-plan", &["02-01", "--pick", ".started"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(run.stdout.trim_end(), started);
    assert_eq!(p.fields_of(&start), before);
}

/// The executor pack `specforge context` prints of [`PLAN`], at `budget` when one is given.
fn executor_pack(p: &crate::harness::Project, budget: Option<&str>) -> serde_json::Value {
    let mut args = vec!["context", "gsd/executor", PLAN];
    if let Some(budget) = budget {
        args.extend(["--max-tokens", budget]);
    }
    let run = p.run(&args);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).expect("a pack")
}

/// Every call records the plan's executor pack, at the route's budget or at `--max-tokens`; a re-run leaves
/// `plan_start` as the first call wrote it.
#[test]
fn start_plan_records_the_executor_pack_on_every_call() {
    let p = twin();
    let run = gsd(&p, "start-plan", &[PLAN, "--pick", ".recorded"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(run.stdout.trim_end(), "gsd/executor");
    let record = p.fields_of(PLAN)["context_pack"].clone();
    assert_eq!(record["route"], "gsd/executor", "{record}");
    assert_eq!(
        record["digest"],
        executor_pack(&p, None)["digest"],
        "{record}"
    );
    assert_eq!(record["max_tokens"], 6000, "{record}");
    let start = p.fields_of(&format!("{PLAN}/START"));
    let run = gsd(&p, "start-plan", &[PLAN, "--max-tokens", "4000"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let record = p.fields_of(PLAN)["context_pack"].clone();
    assert_eq!(record["max_tokens"], 4000, "{record}");
    assert_eq!(
        record["digest"],
        executor_pack(&p, Some("4000"))["digest"],
        "{record}"
    );
    assert_eq!(p.fields_of(&format!("{PLAN}/START")), start);
    assert!(
        !p.codes().iter().any(|c| c == "E083" || c == "gsd/W524"),
        "{:?}",
        p.codes()
    );
}

/// `--head` is a commit id, recorded once beside the time; empty is an unborn branch, which has none.
#[test]
fn start_plan_records_the_head_once_and_refuses_what_is_no_sha() {
    let p = twin();
    let sha = "a".repeat(40);
    let start = format!("{PLAN}/START");
    for bad in [
        "HEAD",
        "abc",
        "$(touch x)",
        &"A".repeat(40),
        &"a".repeat(41),
    ] {
        let before = bytes(&p);
        let run = gsd(&p, "start-plan", &[PLAN, "--head", bad]);
        refused(&run, "invalid_input", &["head"]);
        assert_eq!(bytes(&p), before, "{bad}");
    }
    let run = gsd(&p, "start-plan", &[PLAN, "--head", ""]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(p.fields_of(&start).get("head").is_none());
    let run = gsd(&p, "start-plan", &[PLAN, "--head", &sha, "--pick", ".head"]);
    assert_eq!(run.stdout.trim_end(), sha, "{}", run.all());
    assert_eq!(p.fields_of(&start)["head"], sha.as_str());
    let other = "b".repeat(64);
    let run = gsd(
        &p,
        "start-plan",
        &[PLAN, "--head", &other, "--pick", ".head"],
    );
    assert_eq!(run.stdout.trim_end(), sha, "{}", run.all());
    assert_eq!(p.fields_of(&start)["head"], sha.as_str());
}

#[test]
fn complete_plan_refuses_a_plan_never_started_and_names_the_remedy() {
    let p = twin();
    plant(&p, "spec/plans.spec", PLANTED);
    let before = bytes(&p);
    let run = gsd(&p, "complete-plan", &[SUMMARY]);
    refused(
        &run,
        "invalid_input",
        &[PLAN, "no start", "specforge gsd start-plan"],
    );
    assert_eq!(bytes(&p), before);
    assert!(
        gsd(
            &p,
            "complete-plan",
            &[SUMMARY, "--started", "2026-10-29T09:00:00Z"]
        )
        .code
            != 0
    );
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
    opened(&p, PLAN);
    plant(&p, "spec/plans.spec", PLANTED);
    p.edit("spec/plans.spec", "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             execute", "plan 02-01 \"Save and load tasks.json\" {\n  phase            v1.0/02\n  type             tdd");
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": true }));
    let before = bytes(&p);
    let run = gsd(&p, "complete-plan", &[SUMMARY]);
    refused(&run, "gate_failed", &["W404", PLAN]);
    assert_eq!(bytes(&p), before);
    // Allowed by default (W404 is a warning there) and with tdd_mode off.
    p.settings(json!({ "profile": "gsd-strict", "workflow.tdd_mode": false }));
    assert_eq!(gsd(&p, "complete-plan", &[SUMMARY, "--dry-run"]).code, 0);
}

#[test]
fn complete_plan_refuses() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(&p, "complete-plan", &["v1.0/02/02-01/02-01-SUMMARY"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(run.all().contains("not found"), "{}", run.all());
    plant(&p, "spec/plans.spec", PLANTED);
    let planted = bytes(&p);
    assert_ne!(planted, before);
    opened(&p, PLAN);
    assert_eq!(gsd(&p, "complete-plan", &[SUMMARY]).code, 0);
    let done = bytes(&p);
    let again = gsd(&p, "complete-plan", &[SUMMARY]);
    refused(&again, "invalid_input", &["already completed"]);
    assert_eq!(bytes(&p), done);
}

// -- answer_checkpoint and confirm_setup (D-r41-4, D-r41-5, D-r41-6) -----------------------------------------------

pub(super) const CHECKPOINT: &str = "v1.0/01/01-02/C1";
const SECOND_PLAN: &str = "v1.0/01/01-02";

/// Each person's step and the plan it belongs to: answering a checkpoint, confirming a user setup.
fn steps() -> [(&'static str, &'static str, Vec<&'static str>); 2] {
    [
        (
            SECOND_PLAN,
            "answer-checkpoint",
            vec![CHECKPOINT, "--option", "timestamp", "--by", "ada"],
        ),
        ("v1.0/02/02-01", "confirm-setup", vec![SETUP, "--by", "ada"]),
    ]
}

/// Opens `plan` (its plan checks played first), then takes the person's `step`; whether `gsd/W524` is raised after.
fn stepped(plan: &str, command: &str, args: &[&str], restart: bool) -> bool {
    let p = super::harness::project("execute");
    for code in ["W311", "W313", "W315", "W316"] {
        let run = p.record(plan, code, "passed", &[]);
        assert_eq!(run.code, 0, "{code}: {}", run.all());
    }
    opened(&p, plan);
    assert!(
        !p.codes().contains(&"gsd/W524".to_string()),
        "{:?}",
        p.codes()
    );
    let run = gsd_as_person(&p, command, args);
    assert_eq!(run.code, 0, "{command}: {}", run.all());
    if restart {
        opened(&p, plan);
    }
    p.codes().contains(&"gsd/W524".to_string())
}

#[test]
fn answering_a_checkpoint_or_confirming_setup_then_start_plan_raises_no_w524() {
    for (plan, command, args) in steps() {
        assert!(!stepped(plan, command, &args, true), "{command}");
    }
}

#[test]
fn answering_a_checkpoint_or_confirming_setup_without_start_plan_raises_w524() {
    for (plan, command, args) in steps() {
        assert!(stepped(plan, command, &args, false), "{command}");
    }
}
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
    let run = gsd_as_person(
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
    let run = gsd_as_person(
        &p,
        "answer-checkpoint",
        &[CHECKPOINT, "--option", "maybe", "--by", "ada"],
    );
    refused(&run, "invalid_input", &["maybe", "flag, timestamp"]);
    let run = gsd_as_person(&p, "answer-checkpoint", &[CHECKPOINT, "--by", "ada"]);
    refused(&run, "invalid_input", &["--option or --text"]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn an_answered_checkpoint_is_refused() {
    let p = super::harness::project("execute");
    assert_eq!(
        gsd_as_person(
            &p,
            "answer-checkpoint",
            &[CHECKPOINT, "--option", "flag", "--by", "ada"]
        )
        .code,
        0
    );
    let before = bytes(&p);
    let run = gsd_as_person(
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
    let run = gsd_as_person(
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
    let run = gsd_as_person(&p, "confirm-setup", &[SETUP, "--by", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let done = &p.fields_of(SETUP)["done"];
    assert_eq!(done["by"], "ada");
    assert!(
        done["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{done}"
    );
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    let again = gsd_as_person(&p, "confirm-setup", &[SETUP, "--by", "ada"]);
    refused(&again, "invalid_input", &["already confirmed"]);
}

const SECOND_SUMMARY: &str = "summary 01-02-SUMMARY \"Completion: summary\" {\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Completion\"\n  subsystem              \"tasks\"\n  requirements_completed [TASK-02]\n}\n";

/// An answered checkpoint stales the plan check; once the plan is executed (its summary is `completed`) that is
/// history, and a gate lets it by.
#[test]
fn an_executed_plans_stale_plan_check_does_not_refuse_a_gate() {
    let p = super::harness::project("execute");
    checked(&p);
    // The start is recorded beside the plan, so its checked verdicts stay fresh, and a re-run passes the gate.
    opened(&p, SECOND_PLAN);
    assert_ne!(p.inputs(SECOND_PLAN, "W311")["verdict"]["state"], "stale");
    assert_eq!(
        gsd_as_person(
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
    let run = gsd(&p, "complete-plan", &[&summary]);
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
        gsd_as_person(
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
        gsd_as_person(
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
    // The executor writes the plan's file before its summary says complete (E016 otherwise, ADR 0350).
    p.write("src/complete.js", "// complete\n");
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

/// E516 names the gate's subject, so a stale verdict anywhere refuses the step.
#[test]
fn a_stale_verdict_anywhere_refuses_start_plan_under_plus() {
    let p = super::harness::project("execute");
    checked(&p);
    assert_eq!(
        gsd_as_person(
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
