//! `set_uat_result` and `approve_validation`: a person's answers, written by commands.

use super::commands_support::*;
use super::harness::project;

const SESSION: &str = "v1.0/01/01-UAT";
const REPORTED: &str = "it prints a stack trace before the message, which is noisy";

/// The twin's U1-U3 as if nobody had answered: no `result` line in the spec, none in the ledger.
pub(super) fn unanswered() -> crate::harness::Project {
    let p = twin();
    let text = p.read("spec/verify.spec");
    let stripped: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("result "))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(stripped, text);
    p.write("spec/verify.spec", &stripped);
    for test in ["U1", "U2", "U3"] {
        unledger(&p, &format!("{SESSION}/{test}"), "result");
    }
    p
}

#[test]
fn set_uat_result_reproduces_the_twins_ledger() {
    let golden = twin();
    let p = unanswered();
    assert_ne!(
        p.read("specforge-harness"),
        golden.read("specforge-harness")
    );
    for (test, args) in [
        ("U1", vec!["pass"]),
        (
            "U2",
            vec!["issue", "--reported", REPORTED, "--severity", "cosmetic"],
        ),
        ("U3", vec!["pass"]),
    ] {
        let id = format!("{SESSION}/{test}");
        let mut all = vec![id.as_str()];
        all.extend(&args);
        let run = gsd(&p, "set-uat-result", &all);
        assert_eq!(run.code, 0, "{test}: {}", run.all());
    }
    assert_eq!(
        p.read("specforge-harness"),
        golden.read("specforge-harness")
    );
    for test in ["U1", "U2", "U3"] {
        let id = format!("{SESSION}/{test}");
        assert_eq!(
            p.fields_of(&id)["result"],
            golden.fields_of(&id)["result"],
            "{test}"
        );
    }
    assert_eq!(p.fields_of("v1.0/01/01-UAT/U2")["reported"], REPORTED);
    assert_eq!(p.codes(), golden.codes());
}

#[test]
fn an_issue_without_reported_is_refused() {
    let p = unanswered();
    let before = bytes(&p);
    // U1 has no `reported` of its own.
    let run = gsd(&p, "set-uat-result", &["v1.0/01/01-UAT/U1", "issue"]);
    refused(&run, "invalid_input", &["--reported"]);
    let run = gsd(&p, "set-uat-result", &["v1.0/01/01-UAT/U1", "skipped"]);
    refused(&run, "invalid_input", &["--reason"]);
    let run = gsd(&p, "set-uat-result", &["v1.0/01/01-UAT/U1", "blocked"]);
    refused(&run, "invalid_input", &["--blocked-by"]);
    let run = gsd(
        &p,
        "set-uat-result",
        &[
            "v1.0/01/01-UAT/U1",
            "skipped",
            "--reason",
            "later",
            "--source",
            "automated",
        ],
    );
    refused(&run, "invalid_input", &["--source automated"]);
    assert_eq!(bytes(&p), before);
}

#[test]
fn a_control_character_in_reported_is_refused() {
    let p = unanswered();
    let before = bytes(&p);
    let run = gsd(
        &p,
        "set-uat-result",
        &["v1.0/01/01-UAT/U1", "issue", "--reported", "bad\u{7}bell"],
    );
    refused(
        &run,
        "invalid_input",
        &["'reported' holds a control character (U+0007)"],
    );
    assert_eq!(bytes(&p), before);
}

#[test]
fn the_sessions_totals_follow() {
    let p = unanswered();
    let totals = || p.fields_of(SESSION)["totals"].clone();
    assert_eq!(totals()["pending"], 3, "{}", totals());
    assert_eq!(
        gsd(&p, "set-uat-result", &["v1.0/01/01-UAT/U1", "pass"]).code,
        0
    );
    assert_eq!(totals()["pending"], 2);
    assert_eq!(totals()["passed"], 1);
    let run = gsd(
        &p,
        "set-uat-result",
        &[
            "v1.0/01/01-UAT/U3",
            "skipped",
            "--reason",
            "Deferred follow-up to phase 02",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(totals()["skipped"], 1);
}

#[test]
fn approve_validation_writes_by_and_at() {
    let p = project("verify");
    p.edit(
        "spec/validation.spec",
        "  approval {\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
        "",
    );
    unledger(&p, "v1.0/01/01-VALIDATION", "approval");
    assert_eq!(p.fields_of("v1.0/01/01-VALIDATION")["status"], "draft");
    let run = gsd(
        &p,
        "approve-validation",
        &["v1.0/01/01-VALIDATION", "grace"],
    );
    assert_eq!(run.code, 0, "{}", run.all());
    let fields = p.fields_of("v1.0/01/01-VALIDATION");
    assert_eq!(fields["approval"]["by"], "grace");
    assert!(
        fields["approval"]["at"]
            .as_str()
            .is_some_and(|at| at.ends_with('Z'))
    );
    assert_eq!(fields["status"], "validated");
    assert!(!p.codes().contains(&"E083".to_string()), "{:?}", p.codes());
    let before = bytes(&p);
    let again = gsd(&p, "approve-validation", &["v1.0/01/01-VALIDATION", "ada"]);
    refused(&again, "invalid_input", &["already approved"]);
    assert_eq!(bytes(&p), before);
}
