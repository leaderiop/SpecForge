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
