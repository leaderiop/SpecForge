//! The commands that record a person's answer (C1 of the r41 review): `answer_checkpoint`, `confirm_setup`,
//! `set_uat_result`, `approve_validation`, `waive_window`, `dispose_refactor` and `pause`.
//!
//! The model: the caller RECORDS what the person said, as upstream's agent does. `--by` is the caller's attestation,
//! never a proof of a person, so any non-empty name records. These tests pin that (live), and carry the desired
//! behaviour as ignored twins until the kernel has a human-attested command kind (K18).

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
        Case {
            command: "pause",
            project: twin,
            args: |by| strs(&["wait", "--phase", "02", "--by", by]),
            holds: Some(("v1.0/02/PS1", "by")),
        },
    ]
}

fn run(p: &Project, case: &Case, by: &str) -> Run {
    let args = (case.args)(by);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    gsd(p, case.command, &args)
}

/// Pins today: whatever name the caller gives is recorded, a person's or not.
#[test]
fn a_call_records_whatever_by_names() {
    for case in cases() {
        let p = (case.project)();
        let result = run(&p, &case, "Grace Hopper");
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

/// `--by` is required and not empty: a record names who attests it.
#[test]
fn an_empty_by_is_refused() {
    for case in cases() {
        let p = (case.project)();
        let before = bytes(&p);
        for by in ["", "   "] {
            let result = run(&p, &case, by);
            refused(&result, "invalid_input", &["'by' is required"]);
            unchanged(&p, &before);
        }
    }
}

/// The desired behaviour (K18): a command of a human-attested kind refuses an agent-facing call, whatever `--by`.
#[test]
#[ignore = "K18: human-attested"]
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

/// The desired behaviour (K18): a gate that a person's answer satisfies (W404's `tdd_review`) is not opened by an
/// agent's call.
#[test]
#[ignore = "K18: human-attested"]
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
