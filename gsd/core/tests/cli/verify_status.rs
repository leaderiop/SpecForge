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
    let run = p.run(&["set", VALIDATION, "approval.by", "ada"]);
    assert_ne!(run.code, 0, "{}", run.all());
    p.edit(
        "spec/validation.spec",
        "  max_latency_s 30\n",
        "  max_latency_s 30\n  approval {\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
    );
    assert!(p.codes().contains(&"E083".to_string()), "{:?}", p.report());
}
