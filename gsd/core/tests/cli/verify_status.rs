//! The verification kinds end to end (`spec/verify.md`), on the `verify` project: who may write them, how they are
//! named, and the values computed for them.

use crate::harness::{Project, project};

const VERIFICATION: &str = "v1.0/01/01-VERIFICATION";

fn verify() -> Project {
    project("verify")
}

#[test]
fn an_agent_written_override_is_refused() {
    let p = verify();
    let run = p.run(&[
        "new",
        "override",
        "OV2",
        "Another",
        "--in",
        "spec/verify.spec",
        "--verification",
        VERIFICATION,
        "--truth",
        "v1.0/01/01-01/TR1",
        "--reason",
        "x",
    ]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
    assert!(!p.read("spec/verify.spec").contains("OV2"), "{}", run.all());
}

#[test]
fn a_hand_written_override_checks_clean() {
    let p = verify();
    assert!(p.read("spec/verify.spec").contains("override OV1"));
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
}

#[test]
fn the_verify_project_checks_clean() {
    assert_eq!(verify().findings(), Vec::new());
}

#[test]
fn a_verify_gap_is_named_by_its_verification() {
    let p = verify();
    let query = p.run(&[
        "query",
        &format!("{VERIFICATION}/VG1"),
        "--depth",
        "0",
        "--format",
        "brief",
    ]);
    assert_eq!(query.code, 0, "{}", query.all());
    assert!(query.all().contains("VG1"), "{}", query.all());
}

#[test]
fn verification_fields_are_computed() {
    let p = verify();
    p.edit(
        "spec/verify.spec",
        "  phase v1.0/01\n",
        "  phase v1.0/01\n  status passed\n",
    );
    assert!(p.codes().contains(&"E082".to_string()), "{:?}", p.report());
}

#[test]
fn a_uat_gap_is_named_03_03_uat_u1_g1() {
    let p = verify();
    let query = p.run(&[
        "query",
        "v1.0/01/01-UAT/U2/G1",
        "--depth",
        "0",
        "--format",
        "brief",
    ]);
    assert_eq!(query.code, 0, "{}", query.all());
    assert!(query.all().contains("G1"), "{}", query.all());
}

#[test]
fn a_uat_result_is_harness_written() {
    let p = verify();
    let run = p.run(&["set", "v1.0/01/01-UAT/U1", "result", "pass"]);
    assert_eq!(run.code, 1, "{}", run.all());
    assert!(run.all().contains("harness_field"), "{}", run.all());
    // The value is in the file and blessed in the ledger, as the command that records an answer leaves it.
    p.edit(
        "spec/uat.spec",
        "  criterion 1\n}",
        "  criterion 1\n  result pass\n}",
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    p.record_harness("v1.0/01/01-UAT/U1", "result", "\"pass\"");
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(p.fields_of("v1.0/01/01-UAT/U1")["result"], "pass");
}

#[test]
fn gap_closure_names_a_verify_or_uat_gap() {
    let p = verify();
    let anchor = "  type           execute\n  files_modified [";
    p.edit(
        "spec/plans.spec",
        anchor,
        &format!(
            "  type           execute\n  gap_closure    [{VERIFICATION}/VG1, v1.0/01/01-UAT/U2/G1]\n  files_modified ["
        ),
    );
    assert_eq!(p.codes(), Vec::<String>::new());
    p.edit(
        "spec/plans.spec",
        &format!("{VERIFICATION}/VG1, "),
        "v1.0/01/01-01/TR1, ",
    );
    assert!(!p.codes().is_empty(), "a truth is not a gap");
}

const VALIDATION: &str = "v1.0/01/01-VALIDATION";

#[test]
fn a_validation_row_names_its_task() {
    let p = verify();
    assert_eq!(p.codes(), Vec::<String>::new());
    let row = p.fields_of(&format!("{VALIDATION}/VR1"));
    assert_eq!(row["task"], "v1.0/01/01-01/T2", "{row}");
    p.edit(
        "spec/validation.spec",
        "  task        v1.0/01/01-01/T2\n",
        "  task        v1.0/01/01-01/T9\n",
    );
    assert!(!p.codes().is_empty(), "a row names a task that exists");
}

#[test]
fn approval_is_harness_written() {
    let p = verify();
    // The fixture's approval is blessed in the ledger.
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    let run = p.run(&["set", VALIDATION, "approval.by", "eve"]);
    assert_ne!(run.code, 0, "{}", run.all());
    // A hand edit of the value no ledger line covers.
    p.edit("spec/validation.spec", "by \"ada\"", "by \"eve\"");
    assert!(p.codes().contains(&"E083".to_string()), "{:?}", p.report());
}

const AUDIT: &str = "v1.0/A1";

#[test]
fn an_agent_cannot_accept_debt() {
    let p = verify();
    let run = p.run(&["set", AUDIT, "debt_accepted.by", "ada"]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
    assert!(!p.read("spec/audit.spec").contains("debt_accepted"));
}

#[test]
fn a_human_accepts_debt() {
    let p = verify();
    p.edit(
        "spec/audit.spec",
        "  milestone v1.0\n",
        "  milestone v1.0\n  debt_accepted {\n    by \"ada\"\n    at 2026-10-29\n  }\n",
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(p.fields_of(AUDIT)["debt_accepted"]["by"], "ada");
}

#[test]
fn audit_status_is_computed() {
    let p = verify();
    p.edit(
        "spec/audit.spec",
        "  milestone v1.0\n",
        "  milestone v1.0\n  status passed\n",
    );
    assert!(p.codes().contains(&"E082".to_string()), "{:?}", p.report());
}

const WINDOW: &str = "BW1";

fn waiver_json(reason: &str) -> String {
    format!(r#"{{"at":"2026-10-29T11:00:00Z","by":"ada","reason":"{reason}"}}"#)
}

/// Writes the waiver into `ops/windows.spec` and blesses it in the ledger, as the command that waives a window does.
fn waive(p: &Project, reason: &str) {
    p.edit(
        "spec/ops/windows.spec",
        "  description \"A TODO remains in the store\"\n",
        &format!(
            "  description \"A TODO remains in the store\"\n  waiver {{\n    reason \"{reason}\"\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }}\n"
        ),
    );
    p.record_harness(WINDOW, "waiver", &waiver_json(reason));
}

#[test]
fn a_waiver_is_immutable_once_recorded() {
    let p = verify();
    waive(&p, "Tracked as issue 42");
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    p.commit();
    // A different waiver, blessed in the ledger as well: the history rule is what refuses it.
    p.edit(
        "spec/ops/windows.spec",
        "reason \"Tracked as issue 42\"",
        "reason \"Waived without a ticket\"",
    );
    p.record_harness(WINDOW, "waiver", &waiver_json("Waived without a ticket"));
    let codes = p.codes();
    assert!(codes.contains(&"E084".to_string()), "{:?}", p.report());
}
