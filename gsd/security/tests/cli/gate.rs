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
