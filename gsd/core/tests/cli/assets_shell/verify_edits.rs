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
