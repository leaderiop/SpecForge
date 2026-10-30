//! `smart_entry` (D-r43-12): `/gsd-next`'s router, read black-box. The signals come from the graph a command receives
//! (ADR 0299); the classification is upstream's, so the twin's situation is derived here through the port from
//! `progress-report`'s phases.

use gsd_core::commands::smart_entry::{Signals, classify};
use serde_json::Value;

use super::commands_support::{bytes, gsd, json, twin, unchanged};
use super::harness::{Project, project};

fn entry(p: &Project, args: &[&str]) -> Value {
    let mut all = vec!["--format", "json"];
    all.extend_from_slice(args);
    let run = gsd(p, "smart-entry", &all);
    assert_eq!(run.code, 0, "{}", run.all());
    json(&run)
}

fn recommended(answer: &Value) -> (&str, &str) {
    let id = answer["recommended"].as_str().expect("an id");
    let action = answer["actions"]
        .as_array()
        .expect("actions")
        .iter()
        .find(|a| a["recommended"] == true)
        .expect("one recommended action");
    assert_eq!(action["id"], id);
    (id, action["command"].as_str().expect("a command"))
}

#[test]
fn smart_entry_on_an_empty_project_is_no_project() {
    let p = project("empty");
    let answer = entry(&p, &[]);
    assert_eq!(answer["situation"], "no-project");
    assert_eq!(recommended(&answer), ("new-project", "/gsd-new-project"));
    assert_eq!(answer["signals"]["has_planning"], false);
}

/// P1: the signals are the twin's computed values, and the situation is the port's `classify` of them.
#[test]
fn smart_entry_classifies_the_twin() {
    let p = twin();
    let answer = entry(&p, &[]);
    let report = json(&gsd(&p, "progress-report", &["--format", "json"]));
    let phases = report["phases"].as_array().expect("phases");
    assert!(phases.len() >= 2, "{report}");
    let current = phases
        .iter()
        .find(|ph| ph["status"] != "Complete")
        .expect("an open phase");
    let status = match current["status"].as_str().expect("a label") {
        "Pending" => "planning",
        "Planned" | "In Progress" => "executing",
        "Executed" | "Needs Review" => "verifying",
        other => panic!("{other}"),
    };
    let done = phases
        .iter()
        .filter(|ph| ph["status"] == "Complete")
        .count() as i64;
    let signals = Signals {
        current_phase: current["number"].as_str().map(String::from),
        total_phases: Some(phases.len() as i64),
        status: status.into(),
        progress: report["percent"].as_i64(),
        has_planning: true,
        has_roadmap: true,
        roadmap_total_phases: Some(phases.len() as i64),
        roadmap_completed_phases: Some(done),
        milestone_phases: phases
            .iter()
            .filter_map(|ph| ph["number"].as_str().map(String::from))
            .collect(),
        ..Signals::default()
    };
    assert_eq!(
        answer["signals"],
        signals.to_json(),
        "the signals are the twin's computed fields"
    );
    assert_eq!(answer["situation"], classify(&signals));
    assert!(matches!(
        answer["situation"].as_str(),
        Some("planning" | "executing" | "verify-pending")
    ));
    // upstream's recommended action for forward motion is `/gsd-progress --next`
    assert_eq!(recommended(&answer).1, "/gsd-progress --next");
    assert_eq!(
        answer["signals"]["git_dirty"], false,
        "a command runs no git: idle-stranded is never derived"
    );
}

#[test]
fn smart_entry_is_paused_after_a_pause() {
    let p = twin();
    assert_eq!(entry(&p, &[])["signals"]["paused"], false);
    let run = gsd(&p, "pause", &["a handoff", "--by", "t"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = entry(&p, &[]);
    assert_eq!(answer["situation"], "paused");
    assert_eq!(recommended(&answer), ("resume-work", "/gsd-resume-work"));
}

#[test]
fn smart_entry_is_blocked_by_an_open_blocker() {
    let p = twin();
    let run = gsd(&p, "raise-blocker", &["Need an API key"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let answer = entry(&p, &[]);
    assert_eq!(answer["situation"], "blocked");
    assert_eq!(answer["signals"]["blockers"][0], "Need an API key");
    assert_eq!(recommended(&answer).0, "debug");
    assert!(
        answer["summary"]
            .as_str()
            .is_some_and(|s| s.starts_with("Blocked · 1 blocker(s)"))
    );
    // resolving it lifts the situation
    let id = p.nodes_of("blocker")[0]["id"]
        .as_str()
        .expect("an id")
        .to_string();
    let run = gsd(&p, "resolve-blocker", &[&id, "got the key"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_ne!(entry(&p, &[])["situation"], "blocked");
}

#[test]
fn smart_entry_names_commands_for_the_runtime() {
    let p = twin();
    let codex = entry(&p, &["--runtime", "codex"]);
    for action in codex["actions"].as_array().expect("actions") {
        assert!(
            action["command"]
                .as_str()
                .is_some_and(|c| c.starts_with("$gsd-")),
            "{action}"
        );
    }
    let cursor = entry(&p, &["--runtime", "cursor"]);
    assert!(
        cursor["actions"][0]["command"]
            .as_str()
            .is_some_and(|c| c.starts_with("/gsd-"))
    );
    // a runtime the setting can hold but upstream has no slash form for falls through to `/gsd-x`
    let other = entry(&p, &["--runtime", "vim"]);
    assert!(
        other["actions"][0]["command"]
            .as_str()
            .is_some_and(|c| c.starts_with("/gsd-"))
    );
}

#[test]
fn smart_entry_writes_nothing() {
    let p = twin();
    let before = bytes(&p);
    entry(&p, &[]);
    entry(&p, &["--runtime", "codex"]);
    unchanged(&p, &before);
}

/// REVIEW-r43 Minor 23: a backlog phase (`999.x`) is in no count and is never the current phase. Mutation: deleting
/// the `999.` filter in `signals` makes the total one higher and changes the current phase.
#[test]
fn a_backlog_phase_is_in_no_count() {
    let p = twin();
    let before = entry(&p, &[]);
    let mut text = p.read("spec/v1.0.spec");
    text.push_str("\nphase 999.1 \"Idea\" {\n  milestone v1.0\n  name \"Idea\"\n  slug \"idea\"\n  goal \"later\"\n  success_criteria [\"a\"]\n}\n");
    p.write("spec/v1.0.spec", &text);
    let after = entry(&p, &[]);
    for key in ["total_phases", "current_phase", "progress"] {
        assert_eq!(
            before["signals"][key], after["signals"][key],
            "{key}: {after}"
        );
    }
    assert_eq!(before["situation"], after["situation"]);
}

/// REVIEW-r43 Minor 23: `verify_failed` is read from the computed `verification.status == gaps_found` of the current
/// phase. Mutation: deleting that read makes the situation `executing`, not `verify-failed`.
#[test]
fn a_gaps_found_verification_of_the_current_phase_is_verify_failed() {
    let p = project("verify");
    assert_ne!(entry(&p, &[])["situation"], "verify-failed");
    std::fs::remove_file(p.root().join("src/complete.js")).expect("remove the artifact");
    let answer = entry(&p, &[]);
    assert_eq!(answer["situation"], "verify-failed", "{answer}");
    assert_eq!(answer["signals"]["verify_failed"], true);
}
