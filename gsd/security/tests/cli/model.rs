//! The threat model kinds over a project that loads both extensions: a plan's threats, a phase's audit with its
//! run and accepted risk, and the threat reference a validation row gains.

use super::harness::{self, Project};

pub const AUDIT: &str = "v1.0/01/01-SECURITY";

/// The `model` project with the audit run's harness values recorded as the command that writes them would.
pub fn model() -> Project {
    let p = harness::project("model").with_extension("@gsd/security", "GSD_SECURITY_WASM");
    let run = format!("{AUDIT}/AU1");
    p.record_harness(&run, "at", "\"2026-10-30T09:00:00Z\"");
    p.record_harness(&run, "total", "2");
    p.record_harness(&run, "closed", "1");
    p.record_harness(&run, "open", "1");
    p.record_harness(&run, "run_by", "\"gsd-security-auditor\"");
    p
}

#[test]
fn a_complete_model_checks_clean_of_errors_from_security() {
    let p = model();
    let report = p.report();
    let bad: Vec<_> = report
        .iter()
        .filter(|(code, _)| {
            code.starts_with("gsd.security/") || code.starts_with('E') || code == "W021"
        })
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
    for id in [
        "v1.0/01/01-01/TB1",
        "v1.0/01/01-01/T-01-01",
        "v1.0/01/01-01/T-01-SC",
        AUDIT,
        &format!("{AUDIT}/AU1"),
        &format!("{AUDIT}/AR1"),
    ] {
        assert_eq!(p.run(&["query", id, "--depth", "0"]).code, 0, "{id}");
    }
    // the validation row's reference resolves to the threat
    let row = p.fields_of("v1.0/01/01-VALIDATION/VR1");
    assert!(row["threat"].to_string().contains("T-01-01"), "{row}");
}

#[test]
fn a_threat_without_its_plan_is_a_missing_required_field() {
    let p = model();
    p.edit(
        "spec/security.spec",
        "  plan        v1.0/01/01-01\n  category    tampering\n  component   \"src/tasks.js\"",
        "  category    tampering\n  component   \"src/tasks.js\"",
    );
    let found = p.report();
    assert!(
        found
            .iter()
            .any(|(code, message)| code.starts_with('E') && message.contains("plan")),
        "{found:#?}"
    );
}

#[test]
fn an_agent_may_not_write_an_accepted_risk() {
    let p = model();
    let run = p.run(&[
        "new",
        "accepted_risk",
        "AR2",
        "--in",
        "spec/security.spec",
        "Another",
        "--audit",
        AUDIT,
        "--threat",
        "v1.0/01/01-01/T-01-01",
        "--rationale",
        "r",
        "--accepted_by",
        "bob",
        "--at",
        "2026-10-30",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
}
