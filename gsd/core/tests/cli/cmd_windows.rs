//! `record_window`, `waive_window` and `fix_window`: the broken-windows ledger.

use super::commands_support::*;
use super::harness::project;

#[test]
fn waive_requires_a_reason() {
    let p = project("verify");
    let before = bytes(&p);
    let run = gsd(&p, "waive-window", &["BW1"]);
    assert_eq!(run.code, 2, "{}", run.all());
    assert!(run.all().contains("REASON"), "{}", run.all());
    unchanged(&p, &before);
}

#[test]
fn record_then_waive_writes_recorded_at_and_waiver() {
    let p = project("verify");
    let run = gsd(
        &p,
        "record-window",
        &[
            "todo",
            "01",
            "A TODO remains in the cli",
            "--file",
            "src/cli.js",
            "--line",
            "7",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let window = p.fields_of("BW2");
    assert_eq!(window["status"], "open");
    assert!(
        window["recorded_at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{window}"
    );
    let run = gsd(&p, "waive-window", &["BW2", "Tracked as issue 42", "ada"]);
    assert_eq!(run.code, 0, "{}", run.all());
    let window = p.fields_of("BW2");
    assert_eq!(window["status"], "waived");
    assert_eq!(window["waiver"]["reason"], "Tracked as issue 42");
    assert_eq!(window["waiver"]["by"], "ada");
    assert!(
        window["waiver"]["at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z')),
        "{window}"
    );
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
}

#[test]
fn fix_then_waive_is_refused() {
    let p = project("verify");
    assert_eq!(gsd(&p, "fix-window", &["BW1"]).code, 0);
    assert_eq!(p.fields_of("BW1")["status"], "fixed");
    let before = bytes(&p);
    let run = gsd(&p, "waive-window", &["BW1", "no longer matters", "ada"]);
    refused(&run, "invalid_input", &["already fixed"]);
    let run = gsd(&p, "fix-window", &["BW1"]);
    refused(&run, "invalid_input", &["already fixed"]);
    unchanged(&p, &before);
}

#[test]
fn record_window_line_is_an_integer() {
    let p = project("verify");
    let run = gsd(
        &p,
        "record-window",
        &["fixme", "v1.0/01", "Broken", "--line", "42"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(p.fields_of("BW2")["line"], 42);
    assert!(
        !p.codes().iter().any(|c| c.starts_with('E')),
        "{:?}",
        p.codes()
    );
}

#[test]
fn a_control_character_in_the_reason_is_refused() {
    let p = project("verify");
    let before = bytes(&p);
    let run = gsd(&p, "waive-window", &["BW1", "bad\u{1b}reason", "ada"]);
    refused(
        &run,
        "invalid_input",
        &["'reason' holds a control character (U+001B)"],
    );
    let run = gsd(&p, "record-window", &["todo", "01", "x\u{9b}y"]);
    refused(
        &run,
        "invalid_input",
        &["'description' holds a control character (U+009B)"],
    );
    unchanged(&p, &before);
}

#[test]
fn a_window_whose_proposal_was_accepted_cannot_be_waived() {
    let p = project("verify");
    p.edit(
        "spec/ops/windows.spec",
        "  phase       v1.0/01\n  file ",
        "  phase       v1.0/01\n  refactor    v1.0/01/RP1\n  file ",
    );
    assert_eq!(
        gsd(&p, "dispose-refactor", &["RP1", "accepted", "ada"]).code,
        0
    );
    let before = bytes(&p);
    let run = gsd(&p, "waive-window", &["BW1", "why", "ada"]);
    refused(&run, "invalid_input", &["already fixed"]);
    unchanged(&p, &before);
}
