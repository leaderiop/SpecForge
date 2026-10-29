//! `pause`, `resume`, `raise_blocker` and `resolve_blocker`: STATE.md's blockers and the handoff.

use super::commands_support::*;
use super::harness::project;

#[test]
fn pause_then_resume_round_trips_the_twin() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(
        &p,
        "pause",
        &[
            "Waiting on the storage decision",
            "--phase",
            "02",
            "--by",
            "ada",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/02/PS1");
    assert_eq!(fields["reason"], "Waiting on the storage decision");
    assert_eq!(fields["by"], "ada");
    assert!(
        fields["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{fields}"
    );
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    assert_ne!(bytes(&p), before);
    let run = gsd(&p, "resume", &["v1.0/02/PS1"]);
    assert_eq!(run.code, 0, "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn a_second_pause_is_numbered_after_the_first() {
    let p = twin();
    for _ in 0..2 {
        assert_eq!(gsd(&p, "pause", &["again", "--phase", "02"]).code, 0);
    }
    assert_eq!(p.fields_of("v1.0/02/PS2")["reason"], "again");
}

#[test]
fn pause_without_a_reason_is_refused_by_clap() {
    let p = twin();
    let before = bytes(&p);
    let run = gsd(&p, "pause", &[]);
    assert_eq!(run.code, 2, "{}", run.all());
    assert!(run.all().contains("REASON"), "{}", run.all());
    assert_eq!(bytes(&p), before);
}

#[test]
fn resume_keeps_a_handoff_another_pause_names() {
    let p = project("execute");
    plant(
        &p,
        "spec/ops.spec",
        "pause PS2 \"Same handoff\" {\n  phase   v1.0/01\n  reason  \"A second pause\"\n  handoff v1.0/01/HO1\n}\n",
    );
    assert_eq!(gsd(&p, "resume", &["v1.0/01/PS1"]).code, 0);
    assert!(
        p.nodes_of("handoff")
            .iter()
            .any(|h| h["id"] == "v1.0/01/HO1")
    );
    assert!(p.nodes_of("pause").iter().all(|n| n["id"] != "v1.0/01/PS1"));
    // The last pause takes the handoff with it.
    assert_eq!(gsd(&p, "resume", &["v1.0/01/PS2"]).code, 0);
    assert!(p.nodes_of("handoff").is_empty());
    assert!(
        !p.codes().iter().any(|c| c.starts_with('E')),
        "{:?}",
        p.codes()
    );
}

#[test]
fn raise_then_resolve_writes_raised_and_resolved() {
    let p = twin();
    let run = gsd(
        &p,
        "raise-blocker",
        &["The file format is undecided", "--phase", "02"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("BL1");
    assert_eq!(fields["text"], "The file format is undecided");
    assert!(
        fields["raised"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{fields}"
    );
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    let run = gsd(&p, "resolve-blocker", &["BL1", "Decided: one JSON file"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let resolved = &p.fields_of("BL1")["resolved"];
    assert_eq!(resolved["how"], "Decided: one JSON file");
    assert!(
        resolved["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{resolved}"
    );
    assert!(!p.codes().contains(&"E107".to_string()), "{:?}", p.codes());
}

#[test]
fn a_resolved_blocker_is_refused() {
    let p = project("execute");
    let before = bytes(&p);
    let run = gsd(&p, "resolve-blocker", &["BL1", "again"]);
    refused(&run, "invalid_input", &["already resolved"]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn a_control_character_in_how_is_refused() {
    let p = twin();
    assert_eq!(gsd(&p, "raise-blocker", &["Open question"]).code, 0);
    let before = bytes(&p);
    let run = gsd(&p, "resolve-blocker", &["BL1", "done\u{1b}[0m"]);
    refused(
        &run,
        "invalid_input",
        &["'how' holds a control character (U+001B)"],
    );
    let run = gsd(&p, "raise-blocker", &["bad\u{85}next"]);
    assert_ne!(run.code, 0);
    assert_eq!(bytes(&p), before);
}
