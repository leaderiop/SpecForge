//! `specforge gsd record-security-audit`: the audit, the threats' verdicts and the trail.

use serde_json::{Value, json};

use super::harness::{self, Project};

const AUDIT: &str = "v1.0/01/01-SECURITY";
const T1: &str = "v1.0/01/01-01/T-01-01";
const T2: &str = "v1.0/01/01-01/T-01-02";

/// Phase 01 with two planned threats (high, medium) and no audit.
fn recording() -> Project {
    harness::project("recording").with_extension("@gsd/security", "GSD_SECURITY_WASM")
}

fn record(p: &Project, verdict: &Value) -> harness::Run {
    p.run(&[
        "gsd",
        "record-security-audit",
        "01",
        "--verdict",
        &verdict.to_string(),
        "--run-by",
        "gsd-security-auditor",
        "--format",
        "json",
    ])
}

fn answered(run: &harness::Run) -> Value {
    assert_eq!(run.code, 0, "{}", run.all());
    let wrapped: Value =
        serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()));
    wrapped["output"].clone()
}

fn first() -> Value {
    json!({ "threats": [
        { "id": "T-01-01", "status": "closed", "evidence": "src/a.js:12" },
        { "id": "T-01-02", "status": "open" }
    ] })
}

#[test]
fn the_first_verdict_creates_the_audit_writes_the_statuses_and_appends_one_run() {
    let p = recording();
    let out = answered(&record(&p, &first()));
    assert_eq!(out, json!({ "appended": true, "threats_open": 0 }));
    let audit = p.fields_of(AUDIT);
    assert_eq!(audit["asvs_level"], 1, "{audit}");
    assert!(
        audit["created"].as_str().is_some_and(|c| c.ends_with('Z')),
        "{audit}"
    );
    let closed = p.fields_of(T1);
    assert_eq!(
        (closed["status"].clone(), closed["evidence"].clone()),
        (json!("closed"), json!("src/a.js:12"))
    );
    assert_eq!(p.fields_of(T2)["status"], "open");
    let run = p.fields_of(&format!("{AUDIT}/AU1"));
    assert_eq!(
        (
            run["total"].clone(),
            run["closed"].clone(),
            run["open"].clone()
        ),
        (json!(2), json!(1), json!(1))
    );
    assert_eq!(run["run_by"], "gsd-security-auditor");
    // the model checks clean of errors
    let bad: Vec<_> = p
        .report()
        .into_iter()
        .filter(|(c, _)| c.starts_with('E'))
        .collect();
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn the_same_verdict_again_appends_nothing() {
    let p = recording();
    answered(&record(&p, &first()));
    let again = answered(&record(&p, &first()));
    assert_eq!(again["appended"], false);
    assert_eq!(
        p.run(&["query", &format!("{AUDIT}/AU2"), "--depth", "0"])
            .code,
        1
    );
}

#[test]
fn changed_counts_append_a_second_run() {
    let p = recording();
    answered(&record(&p, &first()));
    let second = json!({ "threats": [{ "id": "T-01-02", "status": "closed" }] });
    assert_eq!(answered(&record(&p, &second))["appended"], true);
    let run = p.fields_of(&format!("{AUDIT}/AU2"));
    assert_eq!(
        (run["closed"].clone(), run["open"].clone()),
        (json!(2), json!(0))
    );
}

#[test]
fn a_retroactive_threat_is_created_in_the_named_plan() {
    let p = recording();
    let verdict = json!({ "threats": [{
        "id": "T-01-09", "status": "open", "retroactive": true, "plan": "01-02",
        "category": "tampering", "component": "src/complete.js", "severity": "high",
        "disposition": "mitigate", "mitigation": "validate ids"
    }] });
    let out = answered(&record(&p, &verdict));
    assert_eq!(out["threats_open"], 2, "high T-01-01 stays open too: {out}");
    let made = p.fields_of("v1.0/01/01-02/T-01-09");
    assert_eq!(made["retroactive"], true, "{made}");
    assert_eq!(made["component"], "src/complete.js");
}

#[test]
fn an_unknown_threat_is_refused_and_nothing_is_written() {
    let p = recording();
    let run = record(
        &p,
        &json!({ "threats": [{ "id": "T-01-77", "status": "open" }] }),
    );
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(
        run.all().contains("invalid_input") && run.all().contains("T-01-77"),
        "{}",
        run.all()
    );
    assert_eq!(
        p.run(&["query", AUDIT, "--depth", "0"]).code,
        1,
        "no audit was created"
    );
}

#[test]
fn an_accepted_threat_is_left_as_it_is() {
    let p = recording();
    answered(&record(&p, &first()));
    let text = p.read("spec/security.spec");
    p.write(
        "spec/security.spec",
        &format!(
            "{text}\naccepted_risk AR1 \"Accepted\" {{\n  audit v1.0/01/01-SECURITY\n  threat v1.0/01/01-01/T-01-02\n  \
             rationale \"fine\"\n  accepted_by \"ada\"\n  at 2026-10-30\n}}\n"
        ),
    );
    let verdict = json!({ "threats": [{ "id": "T-01-02", "status": "closed", "evidence": "x" }] });
    answered(&record(&p, &verdict));
    assert_eq!(
        p.fields_of(T2)["status"],
        "open",
        "an accepted threat does not resurface"
    );
}
