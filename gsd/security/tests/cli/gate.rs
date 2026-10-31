//! `specforge gsd security-gate <phase>`: the ship gate's verdict, fail closed.

use serde_json::{Value, json};

use super::harness::Project;
use super::model::model;

fn gate(p: &Project, phase: &str) -> Value {
    let run = p.run(&["gsd", "security-gate", phase, "--format", "json"]);
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

#[test]
fn no_audit_is_the_no_review_block() {
    // phase 02 has no audit
    let out = gate(&model(), "02");
    assert_eq!(out["block"], true);
    assert!(
        out["message"].as_str().is_some_and(|m| m.starts_with(
            "Security enforcement is enabled but no SECURITY.md exists for this phase."
        ) && m.contains("/gsd-secure-phase 02")),
        "{out}"
    );
    assert_eq!(out["details"]["code"], "SECURITY_SHIP_GATE_NO_REVIEW");
    assert_eq!(out["details"]["exists"], false);
}

#[test]
fn an_open_threat_is_the_open_threats_block() {
    let out = gate(&model(), "01");
    assert_eq!(out["block"], true, "{out}");
    assert_eq!(out["details"]["code"], "SECURITY_SHIP_GATE_OPEN_THREATS");
    assert_eq!(out["details"]["exists"], true);
    assert_eq!(out["details"]["threats_open"], 1);
    assert_eq!(out["details"]["status"], "draft");
    assert!(
        out["message"]
            .as_str()
            .is_some_and(|m| m.contains("(found: 1)")),
        "{out}"
    );
}

#[test]
fn zero_open_threats_passes() {
    let p = model();
    p.settings_of(
        "@gsd/security",
        json!({ "workflow.security_block_on": "critical" }),
    );
    let out = gate(&p, "01");
    assert_eq!(out["block"], false, "{out}");
    assert_eq!(out["details"]["threats_open"], 0);
    assert_eq!(out["details"]["status"], "verified");
}

#[test]
fn enforcement_off_does_not_block() {
    let p = model();
    p.settings_of(
        "@gsd/security",
        json!({ "workflow.security_enforcement": false }),
    );
    assert_eq!(gate(&p, "01")["block"], false);
    assert_eq!(gate(&p, "02")["block"], false);
}

#[test]
fn an_unknown_phase_is_refused() {
    let run = model().run(&["gsd", "security-gate", "99", "--format", "json"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("invalid_input"), "{}", run.all());
}

/// Fail closed (REVIEW-security-part2 m3 and the brief's contract 2): the gate never answers `block: false` unless the
/// phase's audit ran and the security entities load and check clean.
fn at_critical() -> Project {
    let p = model();
    p.settings_of(
        "@gsd/security",
        json!({ "workflow.security_block_on": "critical" }),
    );
    assert_eq!(gate(&p, "01")["block"], false, "the base passes");
    p
}

#[test]
fn an_audit_that_never_ran_blocks() {
    let p = model();
    let made = p.run(&[
        "new",
        "security_audit",
        "02-SECURITY",
        "Phase 02 security",
        "--phase",
        "v1.0/02",
        "--in",
        "spec/security.spec",
    ]);
    assert_eq!(made.code, 0, "{}", made.all());
    let out = gate(&p, "02");
    assert_eq!(out["block"], true, "{out}");
    assert_eq!(out["details"]["code"], "SECURITY_SHIP_GATE_OPEN_THREATS");
    assert!(
        out["message"]
            .as_str()
            .is_some_and(|m| m.contains("(found: draft)")),
        "{out}"
    );
}

#[test]
fn a_threat_that_does_not_load_blocks_with_a_reason() {
    for (from, to) in [
        // an enum the kind does not have: it would block nothing
        ("severity    high", "severity    bogus"),
        // a plan that does not resolve: the threat would belong to no phase
        (
            "plan        v1.0/01/01-01\n  category    tampering\n  component   \"src/tasks.js\"",
            "plan        v1.0/01/09-09\n  category    tampering\n  component   \"src/tasks.js\"",
        ),
    ] {
        let p = at_critical();
        p.edit("spec/security.spec", from, to);
        let out = gate(&p, "01");
        assert_eq!(out["block"], true, "{to}: {out}");
        assert_eq!(
            out["details"]["code"], "SECURITY_SHIP_GATE_UNREADABLE",
            "{out}"
        );
        assert!(
            out["details"]["problems"]
                .as_array()
                .is_some_and(|p| !p.is_empty()),
            "{out}"
        );
    }
}

#[test]
fn an_accepted_risk_that_does_not_resolve_blocks() {
    let p = at_critical();
    p.edit(
        "spec/security.spec",
        "threat      v1.0/01/01-01/T-01-SC",
        "threat      v1.0/01/01-01/T-01-99",
    );
    let out = gate(&p, "01");
    assert_eq!(out["block"], true, "{out}");
    assert_eq!(
        out["details"]["code"], "SECURITY_SHIP_GATE_UNREADABLE",
        "{out}"
    );
}

#[test]
fn a_register_that_lost_threats_since_the_last_run_blocks() {
    let p = at_critical();
    // the run counted 2 threats; one is gone (a block the file no longer parses reads the same)
    let text = p.read("spec/security.spec");
    let start = text.find("threat T-01-01").expect("the threat");
    let end = start + text[start..].find("}\n").expect("its end") + 2;
    p.write(
        "spec/security.spec",
        &format!("{}{}", &text[..start], &text[end..]),
    );
    let out = gate(&p, "01");
    assert_eq!(out["block"], true, "{out}");
    assert_eq!(
        out["details"]["code"], "SECURITY_SHIP_GATE_UNREADABLE",
        "{out}"
    );
}
