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

/// C1 of the r39 review, closed for `override` by marking its fields human (`.u()`): the host checks a field's writer on
/// `set`, `unset` and `link` of an existing entity. (An override has no list field, so `link` has nothing to add to.)
#[test]
fn an_agent_cannot_edit_an_existing_override() {
    let p = verify();
    let before = p.read("spec/verify.spec");
    for args in [
        &["set", "v1.0/01/01-VERIFICATION/OV1", "by", "eve"][..],
        &["set", "v1.0/01/01-VERIFICATION/OV1", "reason", "forged"][..],
        &["set", "v1.0/01/01-VERIFICATION/OV1", "at", "2026-01-01"][..],
        &[
            "set",
            "v1.0/01/01-VERIFICATION/OV1",
            "truth",
            "v1.0/01/01-02/TR1",
        ][..],
        &["unset", "v1.0/01/01-VERIFICATION/OV1", "by"][..],
        &["set", "v1.0/01/01-VERIFICATION/OV1", "accepts", "anything"][..],
    ] {
        let run = p.run(args);
        assert_ne!(run.code, 0, "{args:?}: {}", run.all());
        assert!(run.all().contains("human_only"), "{args:?}: {}", run.all());
    }
    assert_eq!(p.read("spec/verify.spec"), before);
}

/// A person edits the file by hand (or from an interactive command, which a test without a terminal cannot be): the
/// change is read back and the project still checks clean.
#[test]
fn a_person_edits_an_override() {
    let p = verify();
    p.edit(
        "spec/verify.spec",
        "Checked by hand against the running app",
        "Checked again by hand",
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(
        p.fields_of("v1.0/01/01-VERIFICATION/OV1")["reason"],
        "Checked again by hand"
    );
}

#[test]
fn a_hand_written_override_checks_clean() {
    let p = verify();
    assert!(p.read("spec/verify.spec").contains("override OV1"));
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
}

#[test]
fn the_verify_project_checks_clean() {
    verify().check_exact(&[]);
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

/// ADR 0298: an agent's delete of an entity holding a person's value is refused; the same entity without it is deleted.
#[test]
fn an_agent_cannot_delete_an_audit_carrying_debt_accepted() {
    let p = verify();
    p.edit(
        "spec/audit.spec",
        "  milestone v1.0\n",
        "  milestone v1.0\n  debt_accepted {\n    by \"ada\"\n    at 2026-10-29\n  }\n",
    );
    let before = p.read("spec/audit.spec");
    let run = p.run(&["delete", AUDIT]);
    assert_ne!(run.code, 0, "{}", run.all());
    assert!(run.all().contains("human_only"), "{}", run.all());
    assert_eq!(p.read("spec/audit.spec"), before);
    let without = verify();
    let run = without.run(&["delete", AUDIT]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert!(!without.read("spec/audit.spec").contains("milestone_audit"));
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

fn verification(p: &Project) -> serde_json::Value {
    p.fields_of(VERIFICATION)
}

/// The fixture's override and human item removed: no truth is accepted and nothing is left for a person to check.
fn without_override_and_human_item(p: &Project) {
    let text = p.read("spec/verify.spec");
    let at = text.find("override OV1").expect("the override");
    p.write("spec/verify.spec", &format!("{}\n", text[..at].trim_end()));
}

#[test]
fn a_missing_artifact_makes_gaps_found() {
    let p = verify();
    std::fs::remove_file(p.root().join("src/complete.js")).expect("remove the artifact");
    assert_eq!(verification(&p)["status"], "gaps_found");
}

#[test]
fn an_open_human_item_makes_human_needed() {
    let p = verify();
    assert_eq!(verification(&p)["status"], "human_needed");
}

#[test]
fn without_verdicts_status_is_pending_and_score_0_of_m() {
    let p = verify();
    without_override_and_human_item(&p);
    let found = verification(&p);
    assert_eq!(
        (&found["status"], &found["score"]),
        (&json_str("pending"), &json_str("0/2")),
        "{found}"
    );
    assert_eq!(p.codes(), Vec::<String>::new(), "{:?}", p.report());
}

fn json_str(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.into())
}

#[test]
fn re_verification_keeps_what_still_holds() {
    let p = verify();
    p.edit(
        "spec/plans.spec",
        "  depends_on     [01-01]\n  files_modified [\"src/complete.js\"]\n",
        &format!("  depends_on     [01-01]\n  gap_closure    [{VERIFICATION}/VG1]\n  files_modified [\"src/complete.js\"]\n"),
    );
    p.commit();
    // Nothing changed since the baseline: the gap that was open still is.
    assert_eq!(
        p.fields_of(&format!("{VERIFICATION}/VG1"))["status"],
        "open"
    );
    assert_eq!(
        verification(&p)["re_verification"],
        serde_json::json!({
            "previous_status": "human_needed",
            "previous_score": "1/2",
            "gaps_remaining": [format!("{VERIFICATION}/VG1")]
        }),
        "{}",
        verification(&p)
    );
    // Plan 01-02 completes: the gap it closes was open at the baseline and is closed now.
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{text}\nsummary 01-02-SUMMARY \"Completion summary\" {{\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks can be completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}}\n"),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    assert_eq!(
        p.fields_of(&format!("{VERIFICATION}/VG1"))["status"],
        "closed"
    );
    assert_eq!(
        verification(&p)["re_verification"],
        serde_json::json!({
            "previous_status": "human_needed",
            "previous_score": "1/2",
            "gaps_closed": [format!("{VERIFICATION}/VG1")]
        }),
        "{}",
        verification(&p)
    );
}

#[test]
fn re_verification_is_absent_without_a_baseline_verification() {
    let p = verify();
    assert!(verification(&p)["re_verification"].is_null());
}

const SESSION: &str = "v1.0/01/01-UAT";

/// Records a person's answer to UAT test `name` (criterion `criterion`): the value in the file, blessed in the ledger.
fn answer(p: &Project, name: &str, criterion: u8, result: &str) {
    p.edit(
        "spec/uat.spec",
        &format!("  criterion {criterion}\n}}"),
        &format!(
            "  criterion {criterion}\n  result {result}\n{}}}",
            if result == "issue" {
                "  reported \"The list ignores it\"\n"
            } else {
                ""
            }
        ),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    p.record_harness(
        &format!("{SESSION}/{name}"),
        "result",
        &format!("\"{result}\""),
    );
}

#[test]
fn uat_totals_match_audit_uat_golden() {
    let p = verify();
    let totals = |p: &Project| p.fields_of(SESSION)["totals"].clone();
    // The Summary of `templates/UAT.md`: total, passed, issues, pending, skipped, blocked.
    assert_eq!(
        totals(&p),
        serde_json::json!({ "total": 2, "passed": 0, "issues": 0, "pending": 2, "skipped": 0, "blocked": 0 })
    );
    assert_eq!(p.fields_of(SESSION)["status"], "testing");
    answer(&p, "U1", 1, "pass");
    answer(&p, "U2", 2, "issue");
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(
        totals(&p),
        serde_json::json!({ "total": 2, "passed": 1, "issues": 1, "pending": 0, "skipped": 0, "blocked": 0 })
    );
    // The issue has an open gap with a root cause.
    assert_eq!(p.fields_of(SESSION)["status"], "diagnosed");
}

#[test]
fn a_completed_gap_closure_plan_resolves_its_gap() {
    let p = verify();
    answer(&p, "U1", 1, "pass");
    answer(&p, "U2", 2, "issue");
    let gap = format!("{SESSION}/U2/G1");
    assert_eq!(p.fields_of(&gap)["status"], "open");
    p.edit(
        "spec/plans.spec",
        "  depends_on     [01-01]\n  files_modified [\"src/complete.js\"]\n",
        &format!("  depends_on     [01-01]\n  gap_closure    [{gap}]\n  files_modified [\"src/complete.js\"]\n"),
    );
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{text}\nsummary 01-02-SUMMARY \"Completion summary\" {{\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks can be completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}}\n"),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    let resolved = p.fields_of(&gap);
    assert_eq!(resolved["status"], "resolved", "{resolved}");
    assert_eq!(resolved["resolved_by"], "v1.0/01/01-02");
    // The issue's gap is resolved, so the session is complete.
    assert_eq!(p.fields_of(SESSION)["status"], "complete");
}

#[test]
fn a_waived_window_is_waived() {
    let p = verify();
    assert_eq!(p.fields_of(WINDOW)["status"], "open");
    waive(&p, "Tracked as issue 42");
    let window = p.fields_of(WINDOW);
    assert_eq!(window["status"], "waived", "{window}");
    assert_eq!(window["milestone"], "v1.0");
}

#[test]
fn a_disposed_proposal_fixes_its_window() {
    let p = verify();
    p.edit(
        "spec/ops/windows.spec",
        "  description \"A TODO remains in the store\"\n",
        "  description \"A TODO remains in the store\"\n  refactor    v1.0/01/RP1\n",
    );
    assert_eq!(p.fields_of("RP1")["status"], "proposed");
    assert_eq!(p.fields_of(WINDOW)["status"], "open");
    p.edit(
        "spec/ops/windows.spec",
        "  target_function \"add\"\n",
        "  target_function \"add\"\n  disposition {\n    outcome accepted\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
    );
    p.record_harness(
        "v1.0/01/RP1",
        "disposition",
        r#"{"at":"2026-10-29T11:00:00Z","by":"ada","outcome":"accepted"}"#,
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(p.fields_of("v1.0/01/RP1")["status"], "accepted");
    assert_eq!(p.fields_of(WINDOW)["status"], "fixed");
}

#[test]
fn an_unverified_phase_makes_the_audit_gaps_found() {
    let p = verify();
    let audit = p.fields_of(AUDIT);
    assert_eq!(audit["status"], "gaps_found", "{audit}");
    assert_eq!(audit["nyquist"]["missing"], 1, "{audit}");
}

#[test]
fn an_override_written_with_relative_references_still_counts() {
    let p = verify();
    p.edit(
        "spec/verify.spec",
        "  verification v1.0/01/01-VERIFICATION\n  truth        v1.0/01/01-01/TR1\n  reason       \"Checked",
        "  verification 01-VERIFICATION\n  truth        01-01/TR1\n  reason       \"Checked",
    );
    p.edit("spec/verify.spec", "  phase v1.0/01\n}", "  phase 01\n}");
    let text = p.read("spec/verify.spec");
    let at = text.find("human_item HI1").expect("the human item");
    p.write("spec/verify.spec", &format!("{}\n", text[..at].trim_end()));
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(verification(&p)["score"], "1/2", "{}", verification(&p));
}

#[test]
fn uat_tests_and_gaps_written_with_relative_references_still_count() {
    let p = verify();
    p.edit(
        "spec/uat.spec",
        "  session   v1.0/01/01-UAT\n  expected  \"A completed",
        "  session   01-UAT\n  expected  \"A completed",
    );
    p.edit(
        "spec/uat.spec",
        "  test       v1.0/01/01-UAT/U2\n",
        "  test       01-UAT/U2\n",
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    let session = p.fields_of(SESSION);
    assert_eq!(session["totals"]["total"], 2, "{session}");
    assert_eq!(session["totals"]["pending"], 2, "{session}");
}

#[test]
fn validation_rows_written_with_relative_references_still_count() {
    let p = verify();
    p.edit(
        "spec/validation.spec",
        "  validation  v1.0/01/01-VALIDATION\n  task        v1.0/01/01-01/T2\n",
        "  validation  01-VALIDATION\n  task        01-01/T2\n",
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    let row = p.fields_of("v1.0/01/01-VALIDATION/VR1");
    assert!(row.get("wave").is_some(), "{row}");
}

#[test]
fn a_declined_proposal_waives_its_window() {
    let p = verify();
    p.edit(
        "spec/ops/windows.spec",
        "  description \"A TODO remains in the store\"\n",
        "  description \"A TODO remains in the store\"\n  refactor    v1.0/01/RP1\n",
    );
    p.edit(
        "spec/ops/windows.spec",
        "  target_function \"add\"\n",
        "  target_function \"add\"\n  disposition {\n    outcome declined\n    by \"ada\"\n    at \"2026-10-29T11:00:00Z\"\n  }\n",
    );
    p.record_harness(
        "v1.0/01/RP1",
        "disposition",
        r#"{"at":"2026-10-29T11:00:00Z","by":"ada","outcome":"declined"}"#,
    );
    assert_eq!(p.report(), Vec::new(), "{:?}", p.report());
    assert_eq!(p.fields_of("v1.0/01/RP1")["status"], "declined");
    assert_eq!(p.fields_of(WINDOW)["status"], "waived");
}

#[test]
fn an_issue_with_one_gap_resolved_and_one_open_is_not_complete() {
    let p = verify();
    answer(&p, "U1", 1, "pass");
    answer(&p, "U2", 2, "issue");
    let text = p.read("spec/uat.spec");
    p.write(
        "spec/uat.spec",
        &format!("{text}\nuat_gap G2 \"Completed tasks keep their position\" {{\n  test       v1.0/01/01-UAT/U2\n  truth      \"A completed task moves down\"\n  reason     \"The list is not reordered\"\n  severity   minor\n  root_cause \"the sort ignores the flag\"\n}}\n"),
    );
    let gap = format!("{SESSION}/U2/G1");
    p.edit(
        "spec/plans.spec",
        "  depends_on     [01-01]\n  files_modified [\"src/complete.js\"]\n",
        &format!("  depends_on     [01-01]\n  gap_closure    [{gap}]\n  files_modified [\"src/complete.js\"]\n"),
    );
    let text = p.read("spec/plans.spec");
    p.write(
        "spec/plans.spec",
        &format!("{text}\nsummary 01-02-SUMMARY \"Completion summary\" {{\n  plan                   v1.0/01/01-02\n  status                 complete\n  one_liner              \"Tasks can be completed\"\n  subsystem              \"store\"\n  requirements_completed [TASK-02]\n}}\n"),
    );
    assert_eq!(p.run(&["format", "spec"]).code, 0);
    assert_eq!(p.fields_of(&gap)["status"], "resolved");
    assert_eq!(p.fields_of(&format!("{SESSION}/U2/G2"))["status"], "open");
    assert_eq!(p.fields_of(SESSION)["status"], "diagnosed");
}

#[test]
fn present_files_alone_never_pass_a_verification() {
    // Both truths overridden, the human item gone, the key links literal and wired: the artifacts are present, yet
    // that is no verdict (a file that is there is not shown substantive and wired), so the phase is not `passed`.
    let p = verify();
    p.edit("spec/verify.spec", "human_item HI1", "human_item HI1");
    let text = p.read("spec/verify.spec");
    let at = text.find("human_item HI1").expect("the human item");
    let second = "\noverride OV2 \"Accept the completion\" {\n  verification v1.0/01/01-VERIFICATION\n  truth        v1.0/01/01-02/TR1\n  reason       \"Checked by hand\"\n  by           \"ada\"\n  at           2026-10-29\n}\n";
    p.write(
        "spec/verify.spec",
        &format!("{}\n{second}", text[..at].trim_end()),
    );
    p.edit(
        "spec/plans.spec",
        "via  \"require\"",
        "pattern \"require\"\n  via  \"require\"",
    );
    p.edit(
        "spec/plans.spec",
        "via  \"import\"\n}\n\nplan 02-01",
        "pattern \"import\"\n  via  \"import\"\n}\n\nplan 02-01",
    );
    let found = verification(&p);
    assert_ne!(found["status"], "passed", "{found}");
    assert_eq!(found["score"], "2/2", "{found}");
}
