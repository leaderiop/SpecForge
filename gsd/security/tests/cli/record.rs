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

/// Both plans of phase 01 keep a `T-01-SC` (by design), and plan 01-02 claims `T-01-02` too (a real E101 duplicate).
fn two_plans() -> Project {
    let p = recording();
    let spec = p.read("spec/security.spec");
    let extra = r#"
threat T-01-SC "Supply chain" {
  plan        v1.0/01/01-01
  category    tampering
  component   "package.json"
  severity    high
  disposition mitigate
  mitigation  "Pin dependencies"
}

threat T-01-SC "Supply chain" {
  plan        v1.0/01/01-02
  category    tampering
  component   "package.json"
  severity    high
  disposition mitigate
  mitigation  "Pin dependencies"
}

threat T-01-02 "Again" {
  plan        v1.0/01/01-02
  category    tampering
  component   "src/complete.js"
  severity    medium
  disposition mitigate
  mitigation  "Checked"
}
"#;
    p.write("spec/security.spec", &format!("{spec}{extra}"));
    p
}

#[test]
fn a_verdict_on_the_supply_chain_row_sets_every_plans_sc_threat() {
    let p = two_plans();
    let v = json!({ "threats": [
        { "id": "T-01-01", "status": "closed" },
        { "id": "T-01-SC", "status": "closed", "evidence": "package-lock.json" },
    ] });
    answered(&record(&p, &v));
    for plan in ["01-01", "01-02"] {
        let sc = p.fields_of(&format!("v1.0/01/{plan}/T-01-SC"));
        assert_eq!(sc["status"], "closed", "{plan}: {sc}");
    }
}

#[test]
fn a_suffixed_id_names_one_threat() {
    let p = two_plans();
    // the bare digit-suffixed id is a real duplicate: refused, nothing written
    let v = json!({ "threats": [{ "id": "T-01-02", "status": "closed" }] });
    let run = record(&p, &v);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(
        run.all().contains("claimed by more than one plan"),
        "{}",
        run.all()
    );
    // the plan's name before it picks one
    let v = json!({ "threats": [{ "id": "01-02/T-01-02", "status": "closed" }] });
    answered(&record(&p, &v));
    assert_eq!(p.fields_of("v1.0/01/01-02/T-01-02")["status"], "closed");
    assert!(p.fields_of(T2).get("status").is_none());
}

/// REVIEW-security-part2 m2: a phase's audit is written beside that phase's plans, not in the file of whichever audit
/// came first.
#[test]
fn each_phases_audit_lands_beside_its_own_plans() {
    let p = recording();
    answered(&record(&p, &first()));
    assert!(
        p.read("spec/security.spec")
            .contains("security_audit 01-SECURITY")
    );
    let milestone = p.read("spec/v1.0.spec");
    p.write(
        "spec/v1.0.spec",
        &format!(
            "{milestone}\nphase 03 \"Later\" {{\n  milestone v1.0\n  name \"Later\"\n  slug \"later\"\n  goal \"Later\"\n}}\n"
        ),
    );
    p.write(
        "spec/phases/03/plans.spec",
        r#"plan 03-01 "Later plan" {
  phase          v1.0/03
  type           execute
  files_modified ["src/later.js"]
  objective      "Later"
}

threat T-03-01 "Later threat" {
  plan        v1.0/03/03-01
  category    tampering
  component   "src/later.js"
  severity    high
  disposition mitigate
  mitigation  "Checked"
}
"#,
    );
    let run = p.run(&[
        "gsd",
        "record-security-audit",
        "03",
        "--verdict",
        &json!({ "threats": [{ "id": "T-03-01", "status": "closed" }] }).to_string(),
        "--format",
        "json",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let later = p.read("spec/phases/03/plans.spec");
    assert!(later.contains("security_audit 03-SECURITY"), "{later}");
    assert!(later.contains("audit_run AU1"), "{later}");
    assert!(!p.read("spec/security.spec").contains("03-SECURITY"));
}
