//! The commands that record a person's answer (C1 of the r41 review): `answer_checkpoint`, `confirm_setup`,
//! `set_uat_result`, `approve_validation`, `waive_window` and `dispose_refactor` are human-attested (ADR 0301): a person
//! at a terminal runs them, and an agent's call is refused `human_only`. `pause` stays an agent's: no gate reads a
//! pause, and `/gsd-pause-work` is the agent's own act.

use super::cmd_execute::{CHECKPOINT, SETUP};
use super::commands_support::*;
use super::harness::{Project, Run, project};

struct Case {
    command: &'static str,
    project: fn() -> Project,
    args: fn(&str) -> Vec<String>,
    /// The entity and field that hold the attestation, when the kind has a place for it.
    holds: Option<(&'static str, &'static str)>,
}

fn strs(args: &[&str]) -> Vec<String> {
    args.iter().map(|a| a.to_string()).collect()
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            command: "answer-checkpoint",
            project: || project("execute"),
            args: |by| strs(&[CHECKPOINT, "--option", "flag", "--by", by]),
            holds: Some((CHECKPOINT, "answer")),
        },
        Case {
            command: "confirm-setup",
            project: || project("execute"),
            args: |by| strs(&[SETUP, "--by", by]),
            holds: Some((SETUP, "done")),
        },
        Case {
            command: "set-uat-result",
            project: super::cmd_verify::unanswered,
            args: |by| strs(&["v1.0/01/01-UAT/U1", "pass", "--by", by]),
            // A uat_test has no `by`: the attestation is echoed, not stored.
            holds: None,
        },
        Case {
            command: "approve-validation",
            project: super::commands::unapproved,
            args: |by| strs(&["v1.0/01/01-VALIDATION", by]),
            holds: Some(("v1.0/01/01-VALIDATION", "approval")),
        },
        Case {
            command: "waive-window",
            project: || project("verify"),
            args: |by| strs(&["BW1", "tracked elsewhere", by]),
            holds: Some(("BW1", "waiver")),
        },
        Case {
            command: "dispose-refactor",
            project: || project("verify"),
            args: |by| strs(&["RP1", "accepted", by]),
            holds: Some(("RP1", "disposition")),
        },
    ]
}

fn run(p: &Project, case: &Case, by: &str) -> Run {
    let args = (case.args)(by);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    gsd(p, case.command, &args)
}

fn run_as_person(p: &Project, case: &Case, by: &str) -> Run {
    let args = (case.args)(by);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    gsd_as_person(p, case.command, &args)
}

/// A person at a terminal types the name, and whatever they type is recorded.
#[test]
fn a_person_records_whatever_by_names() {
    for case in cases() {
        let p = (case.project)();
        let result = run_as_person(&p, &case, "Grace Hopper");
        assert_eq!(result.code, 0, "{}: {}", case.command, result.all());
        if let Some((entity, field)) = case.holds {
            let held = p.fields_of(entity)[field].to_string();
            assert!(
                held.contains("Grace Hopper"),
                "{}: {entity}.{field} = {held}",
                case.command
            );
        }
    }
}

/// `--by` is required and not empty: a record names who answered.
#[test]
fn an_empty_by_is_refused() {
    for case in cases() {
        let p = (case.project)();
        let before = bytes(&p);
        for by in ["", "   "] {
            let result = run_as_person(&p, &case, by);
            refused(&result, "invalid_input", &["'by' is required"]);
            unchanged(&p, &before);
        }
    }
}

/// A human-attested command refuses an agent-facing call, whatever `--by`.
#[test]
fn a_human_attested_command_refuses_an_agent_call() {
    for case in cases() {
        let p = (case.project)();
        let before = bytes(&p);
        let result = run(&p, &case, "Grace Hopper");
        assert_ne!(result.code, 0, "{}: {}", case.command, result.all());
        assert!(
            result.all().contains("human_only"),
            "{}: {}",
            case.command,
            result.all()
        );
        unchanged(&p, &before);
    }
}

/// A gate that a person's answer satisfies (W404's `tdd_review`) is not opened by an agent's call.
#[test]
fn an_agents_answer_does_not_satisfy_the_tdd_review_gate() {
    let p = project("execute");
    p.settings(serde_json::json!({ "profile": "gsd-strict", "workflow.tdd_mode": true }));
    let result = gsd(
        &p,
        "answer-checkpoint",
        &[CHECKPOINT, "--option", "flag", "--by", "anyone"],
    );
    assert_ne!(result.code, 0, "{}", result.all());
}

/// `pause` records the agent's own act: any `--by` from an agent's call.
#[test]
fn pause_records_any_by_from_an_agent() {
    let p = twin();
    let result = gsd(
        &p,
        "pause",
        &["wait", "--phase", "02", "--by", "Grace Hopper"],
    );
    assert_eq!(result.code, 0, "{}", result.all());
    assert!(
        p.fields_of("v1.0/02/PS1")["by"]
            .to_string()
            .contains("Grace Hopper")
    );
}
