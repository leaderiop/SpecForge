//! The compute pass: a threat's `blocking`, the audit's `threats_open`, `status` and `register_planned`, over the
//! `model` project (one open high threat, one accepted low supply-chain threat, a run).

use super::model::model;

const AUDIT: &str = "v1.0/01/01-SECURITY";
const HIGH: &str = "v1.0/01/01-01/T-01-01";
const SC: &str = "v1.0/01/01-01/T-01-SC";

fn audit(p: &super::harness::Project) -> serde_json::Value {
    p.fields_of(AUDIT)
}

#[test]
fn an_open_high_threat_blocks_at_the_default_threshold() {
    let p = model();
    let fields = audit(&p);
    assert_eq!(fields["threats_open"], 1, "{fields}");
    assert_eq!(fields["status"], "draft", "{fields}");
    assert_eq!(fields["register_planned"], true, "{fields}");
    assert_eq!(p.fields_of(HIGH)["blocking"], true);
    // accepted (and low): never blocking
    assert_eq!(p.fields_of(SC)["blocking"], false);
}

#[test]
fn a_higher_threshold_clears_it_and_the_run_makes_the_audit_verified() {
    let p = model();
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_block_on": "critical" }),
    );
    let fields = audit(&p);
    assert_eq!(fields["threats_open"], 0, "{fields}");
    assert_eq!(fields["status"], "verified", "{fields}");
    assert_eq!(p.fields_of(HIGH)["blocking"], false);
}

#[test]
fn none_blocks_nothing() {
    let p = model();
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_block_on": "none" }),
    );
    assert_eq!(audit(&p)["threats_open"], 0);
}

#[test]
fn an_accepted_risk_stops_a_threat_blocking() {
    let p = model();
    p.edit(
        "spec/security.spec",
        "threat      v1.0/01/01-01/T-01-SC",
        "threat      v1.0/01/01-01/T-01-01",
    );
    assert_eq!(p.fields_of(HIGH)["blocking"], false);
    assert_eq!(audit(&p)["threats_open"], 0);
}

#[test]
fn a_closed_threat_does_not_block() {
    let p = model();
    p.record_harness(HIGH, "status", "\"closed\"");
    p.edit(
        "spec/security.spec",
        "  files       [\"src/tasks.js\"]",
        "  files       [\"src/tasks.js\"]\n  status      closed",
    );
    assert_eq!(p.fields_of(HIGH)["blocking"], false);
}

#[test]
fn an_audit_without_a_run_is_a_draft_and_retroactive_threats_are_not_a_planned_register() {
    let p = model();
    // drop the run
    let text = p.read("spec/security.spec");
    let start = text.find("audit_run AU1").expect("the run");
    let end = text.find("accepted_risk AR1").expect("the risk");
    p.write(
        "spec/security.spec",
        &format!("{}{}", &text[..start], &text[end..]),
    );
    p.settings_of(
        "@gsd/security",
        serde_json::json!({ "workflow.security_block_on": "none" }),
    );
    assert_eq!(audit(&p)["status"], "draft", "no run, nothing open");
    // every threat retroactive
    for threat in [HIGH, SC] {
        p.record_harness(threat, "retroactive", "true");
    }
    for name in ["T-01-01", "T-01-SC"] {
        let header = format!("threat {name} ");
        let text = p.read("spec/security.spec");
        let at = text.find(&header).expect("the threat");
        let brace = at + text[at..].find('{').expect("a block") + 1;
        p.write(
            "spec/security.spec",
            &format!("{}\n  retroactive true{}", &text[..brace], &text[brace..]),
        );
    }
    assert_eq!(audit(&p)["register_planned"], false);
}
