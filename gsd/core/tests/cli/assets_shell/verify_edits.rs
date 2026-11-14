//! The verify family's steps (the verifier, its spawn, UAT, validation), run as the assets hold them on a taskboard
//! copy with only `specforge` on the `PATH`, and the entities and answers they produce.

use super::{Shell, asset, typed};
use crate::harness::taskboard;

/// The code of the first bash fence of `text` holding `mark`.
pub(super) fn fence_holding(text: &str, mark: &str) -> String {
    let fence = gsd_kit::markdown::fences(text)
        .into_iter()
        .find(|f| f.language == "bash" && f.text.contains(mark))
        .unwrap_or_else(|| panic!("no bash fence holds {mark}"));
    super::code(&fence.text).to_string()
}

/// The verifier's Step 6 traces the phase's requirements along their threads: each requirement of `v1.0/01` with its
/// plan and its verified truth.
#[test]
fn the_verifier_traces_its_phases_requirements() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let code = fence_holding(
        &asset("agents/gsd-verifier.md"),
        "trace --route gsd/requirement-thread",
    );
    let (out, err, status) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{code}"));
    assert_eq!(status, 0, "{err}");
    for want in ["TASK-01", "TASK-02", "plan 01-01", "TR1 verified"] {
        assert!(out.contains(want), "{want}: {out}");
    }
}

/// A verdict cites a test fact by the id the thread's test step prints: the verification template's record fence,
/// filled with that id, the basis the truth's inputs give and no finding, is accepted, and the truth's inputs then
/// list the cited test.
#[test]
fn the_verifier_cites_a_test_by_fact_id() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let run = p.collect_node_test_golden();
    assert_eq!(run.code, 0, "{}", run.all());
    let trace = p.run(&[
        "trace",
        "--route",
        "gsd/requirement-thread",
        "--in",
        "v1.0/01",
        "--format",
        "json",
    ]);
    let trace: serde_json::Value = serde_json::from_str(&trace.stdout).expect("the threads");
    let test = trace["traces"]
        .as_array()
        .expect("traces")
        .iter()
        .find(|t| t["entity_id"] == "TASK-01")
        .and_then(|t| t["steps"].as_array()?.iter().find(|s| s["name"] == "test"))
        .and_then(|s| s["matches"][0]["entity_id"].as_str())
        .expect("TASK-01's test")
        .to_string();
    let truth = "v1.0/01/01-01/TR1";
    let inputs = p.run(&[
        "query",
        truth,
        "--inputs",
        "gsd/E501",
        "--cite-file",
        "src/tasks.js",
        "--cite-test",
        &test,
        "--format",
        "json",
    ]);
    let inputs: serde_json::Value = serde_json::from_str(&inputs.stdout).expect("the inputs");
    let cited = serde_json::json!([{"file": "src/tasks.js"}, {"test": test}]).to_string();
    let record = typed::filled(
        &fence_holding(
            &asset("gsd-core/templates/verification-report.md"),
            "record-verdict",
        ),
        &[
            ("TRUTH", truth),
            ("OUTCOME", "verified"),
            ("BASIS", &inputs["basis"].to_string()),
            ("CITED", &cited),
            ("FINDINGS", "[]"),
        ],
    );
    let done = typed::apply(&sh, &p, &record);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let inputs = p.run(&["query", truth, "--inputs", "gsd/E501", "--format", "json"]);
    assert!(inputs.stdout.contains(&test), "{}", inputs.stdout);
}

/// The verifier spawn's reading list opens with the `gsd/verifier` pack line and reads no context window; run with
/// `PHASE_ID=v1.0/01`, the line serves the phase's truths.
#[test]
fn the_verifier_spawn_reads_one_context_line() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/execute-phase/steps/verify-phase-goal.md");
    assert!(!text.contains("CONTEXT_WINDOW"));
    let reading = super::block(&text, "<required_reading>", "</required_reading>");
    let first = reading.lines().nth(1).expect("a first line");
    assert!(
        first.starts_with("specforge context gsd/verifier"),
        "{reading}"
    );
    let p = taskboard();
    let line = first.replace("{PHASE_ID}", "$PHASE_ID");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{line}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("v1.0/01/01-01/TR1"), "{out}");
}

/// The UAT template's fences, filled, create a session on `v1.0/02` with one test, then a gap on that test.
#[test]
fn the_uat_template_records_a_session_a_test_and_a_gap() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let template = asset("gsd-core/templates/UAT.md");
    let session = typed::filled(
        &fence_holding(&template, "new uat_session"),
        &[
            ("PHASE_ID", "v1.0/02"),
            ("SOURCE", "v1.0/01/01-02/01-02-SUMMARY"),
            ("TEST_ID", "U1"),
            ("NAME", "Undo the last add"),
            (
                "EXPECTED",
                "`node src/cli.js undo` removes the task added last",
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &session);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(p.fields_of("v1.0/02/02-UAT")["phase"], "v1.0/02");
    let gap = typed::filled(
        &fence_holding(&template, "new uat_gap"),
        &[
            ("TEST", "v1.0/02/02-UAT/U1"),
            ("GAP_ID", "G1"),
            ("SEVERITY", "major"),
            ("TRUTH", "Undo removes the task added last"),
            ("REASON", "User reported: nothing happens"),
        ],
    );
    let done = typed::apply(&sh, &p, &gap);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert_eq!(
        p.fields_of("v1.0/02/02-UAT/U1/G1")["reason"],
        "User reported: nothing happens"
    );
}

/// diagnose-issues' fence, filled, records the diagnosis on `01-UAT`'s gap `G1`: its root cause, its artifacts and
/// what is missing, with no new finding.
#[test]
fn a_uat_gap_diagnosis_is_a_typed_edit() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let gap = "v1.0/01/01-UAT/U2/G1";
    let diagnosis = typed::filled(
        &fence_holding(
            &asset("gsd-core/workflows/diagnose-issues.md"),
            "root_cause",
        ),
        &[
            ("GAP", gap),
            (
                "ROOT_CAUSE",
                "done() rethrows the lookup error before printing",
            ),
            (
                "ARTIFACTS",
                r#"[{"path": "src/cli.js", "issue": "the catch rethrows"}]"#,
            ),
            (
                "MISSING",
                "Print the message without the stack\nTest the one-line output",
            ),
        ],
    );
    let done = typed::apply(&sh, &p, &diagnosis);
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    assert!(done.added.is_empty(), "{:?}", done.added);
    let fields = p.fields_of(gap);
    assert_eq!(
        fields["root_cause"],
        "done() rethrows the lookup error before printing"
    );
    assert_eq!(fields["artifacts"][0]["path"], "src/cli.js");
    assert_eq!(
        fields["missing"].as_array().map(Vec::len),
        Some(2),
        "{fields}"
    );
}

/// The line verify-work shows the person for a deferred follow-up, run as the person runs it on `01-UAT`'s `U3`, adds
/// one `follow_ups` entry to the session, and the step's count reads it.
#[cfg(unix)]
#[test]
fn a_deferred_follow_up_lands_on_the_session() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let text = asset("gsd-core/workflows/verify-work.md");
    assert!(text.contains(
        "`specforge gsd set-uat-result <uat_test> skipped --by <who> --follow-up \"<their words>\"`"
    ));
    let p = taskboard();
    let run = p.run_as_person(&[
        "gsd",
        "set-uat-result",
        "v1.0/01/01-UAT/U3",
        "skipped",
        "--by",
        "Sam",
        "--follow-up",
        "print the version beside the usage line",
    ]);
    assert_eq!(run.code, 0, "{}", run.all());
    let follow_ups = p.fields_of("v1.0/01/01-UAT")["follow_ups"].clone();
    assert_eq!(follow_ups.as_array().map(Vec::len), Some(1), "{follow_ups}");
    let count = fence_holding(&text, "totals.follow_ups");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/01\n{count}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("deferred follow-ups: 1"), "{out}");
}

/// verify-work's [P] fence for one follow-up adds the backlog phase `v1.0/999.1`.
#[test]
fn uat_follow_ups_promote_to_backlog_phases() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    let promote = typed::filled(
        &fence_holding(&asset("gsd-core/workflows/verify-work.md"), "--backlog"),
        &[
            ("TEST", "3"),
            ("IDEA", "print the version beside the usage line"),
        ],
    );
    let done = typed::apply(&sh, &p, &format!("PHASE_ID=v1.0/01\n{promote}"));
    assert_eq!(done.code, 0, "{}{}", done.out, done.err);
    let phase = p.fields_of("v1.0/999.1");
    assert_eq!(
        phase["goal"], "print the version beside the usage line",
        "{phase}"
    );
}

/// validate-phase §6's fence, filled and run once per row on `v1.0/02` (no validation yet; its tasks given an
/// automated verify, as `gsd/E314` asks of a validated phase), creates one validation and two rows with no new finding;
/// the auditor spawn's context line, run with that phase, names the validation.
#[test]
fn the_validation_audit_writes_rows() {
    let Some(sh) = Shell::only(&[]) else {
        return;
    };
    let p = taskboard();
    for task in ["v1.0/02/02-01/T1", "v1.0/02/02-01/T2"] {
        let run = p.run(&[
            "set",
            task,
            "verify",
            r#"[{"automated": "npm test", "fails_when": "a task test fails"}]"#,
        ]);
        assert_eq!(run.code, 0, "{}", run.all());
    }
    let text = asset("gsd-core/workflows/validate-phase.md");
    let fence = fence_holding(&text, "new validation_row");
    for (row, task) in [("R1", "v1.0/02/02-01/T1"), ("R2", "v1.0/02/02-01/T2")] {
        let filled = typed::filled(
            &fence,
            &[
                ("PHASE_ID", "v1.0/02"),
                ("FRAMEWORK", "node:test"),
                ("QUICK_CMD", "node --test"),
                ("FULL_CMD", "npm test"),
                ("ROW_ID", row),
                ("TASK", task),
                ("TEST_TYPE", "unit"),
                ("REQUIREMENT", "TASK-03"),
                ("TEST_FILE", "test/tasks.test.js"),
                ("COMMAND", "node --test test/tasks.test.js"),
            ],
        );
        let done = typed::apply(&sh, &p, &filled);
        assert_eq!(done.code, 0, "{}{}", done.out, done.err);
        assert!(done.added.is_empty(), "{:?}", done.added);
    }
    assert_eq!(p.nodes_of("validation").len(), 1);
    assert_eq!(p.nodes_of("validation_row").len(), 2);
    let line = text
        .lines()
        .find(|l| l.contains("specforge context gsd/nyquist-auditor"))
        .expect("the spawn's context line");
    let start = line.find("specforge context").expect("the command");
    let end = line[start..].find("</required_reading>").expect("its end") + start;
    let command = line[start..end]
        .replace("\\\"", "\"")
        .replace("{PHASE_ID}", "$PHASE_ID");
    let (out, err, code) = sh.run_in(p.root(), &format!("PHASE_ID=v1.0/02\n{command}"));
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("02-VALIDATION"), "{out}");
}
