//! `gsd.security/W301`: a completed phase whose audit is missing, still a draft, or has threats open.

use super::harness::Project;
use super::model::model;

const PHASE: &str = "v1.0/01";

/// The model project with phase 01 complete: its verification passed (every truth, artifact and link verified).
fn complete() -> Project {
    let p = model();
    p.write(
        "spec/verify.spec",
        "verification 01-VERIFICATION \"Phase 01 verification\" {\n  phase v1.0/01\n}\n",
    );
    p.verify_phase(PHASE);
    assert_eq!(p.fields_of(PHASE)["status"], "complete");
    p
}

fn w301(p: &Project) -> Vec<String> {
    p.report()
        .into_iter()
        .filter(|(code, _)| code == "gsd.security/W301")
        .map(|(_, message)| message)
        .collect()
}

fn drop_audit(p: &Project) {
    let text = p.read("spec/security.spec");
    let start = text.find("security_audit").expect("the audit");
    p.write("spec/security.spec", &text[..start]);
}

#[test]
fn a_complete_phase_without_an_audit_says_missing() {
    let p = complete();
    drop_audit(&p);
    let found = w301(&p);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("01-SECURITY is missing") && found[0].contains("/gsd-secure-phase 01"),
        "{found:?}"
    );
}

#[test]
fn an_audit_without_a_run_is_still_draft() {
    let p = complete();
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_block_on": "none" }),
    );
    let text = p.read("spec/security.spec");
    let (start, end) = (
        text.find("audit_run AU1").expect("run"),
        text.find("accepted_risk AR1").expect("risk"),
    );
    p.write(
        "spec/security.spec",
        &format!("{}{}", &text[..start], &text[end..]),
    );
    let found = w301(&p);
    assert!(
        found.len() == 1 && found[0].contains("is still draft"),
        "{found:?}"
    );
}

#[test]
fn threats_open_are_counted() {
    let p = complete();
    let found = w301(&p);
    assert!(
        found.len() == 1 && found[0].contains("1 threat(s) open"),
        "{found:?}"
    );
}

#[test]
fn a_verified_audit_is_silent() {
    let p = complete();
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_block_on": "critical" }),
    );
    assert!(w301(&p).is_empty(), "{:?}", w301(&p));
}

#[test]
fn enforcement_off_is_silent() {
    let p = complete();
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_enforcement": false }),
    );
    assert!(w301(&p).is_empty(), "{:?}", w301(&p));
}

#[test]
fn a_phase_that_is_not_complete_is_silent() {
    let p = model();
    assert_ne!(p.fields_of(PHASE)["status"], "complete");
    assert!(w301(&p).is_empty());
}
