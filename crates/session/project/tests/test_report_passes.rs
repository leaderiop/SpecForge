//! K12: a check, compute or guard pass that declares `reads_tests` receives the recorded test report, and none
//! recorded is I205 once (E045 when it cannot be used). A pass that does not declare it is sent what 1.10 sent.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_project::testing::{
    GSD_JUDGED, REPORT, TestProject, gsd_facts_project, gsd_facts_runtime, gsd_facts_serving,
    gsd_judged_builder, gsd_judged_project, pass_input,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_wasm::testing::InProcessRuntime;

fn count(diagnostics: &[Diagnostic], code: &str) -> usize {
    diagnostics.iter().filter(|d| d.code == code).count()
}

/// A one-test report for the plan `pl1`, written as `collect` writes it.
const REPORT_OF_PL1: &str =
    r#"{"runner":"fixture","results":{"pl1":{"tests":[{"name":"it works","status":"passed"}]}}}"#;

/// `passes` of [`FACTS_EXTENSION`] over the facts project with `report` (if any) recorded.
fn compile(
    passes: Value,
    report: Option<&str>,
) -> (CompiledProject, Arc<InProcessRuntime>, TestProject) {
    let mut project = gsd_facts_project();
    if let Some(report) = report {
        project = project.file("specforge-report.json", report);
    }
    let runtime = Arc::new(gsd_facts_serving(passes));
    (project.compile_in(runtime.clone()), runtime, project)
}

fn reading(phase: &str) -> Value {
    let computes = if phase == "compute" {
        json!(["plan.status"])
    } else {
        json!([])
    };
    json!([{"name": "p", "phase": phase, "reads_tests": true, "computes": computes}])
}

#[test]
fn a_check_pass_sees_the_recorded_report() {
    let project = gsd_facts_project().file("specforge-report.json", REPORT);
    let runtime = Arc::new(gsd_facts_runtime());
    let compiled = project.compile_in(runtime.clone());

    let input = pass_input(&runtime, "tests");
    assert!(input["test_results"].is_object(), "{input}");
    // A pass that does not declare `reads_tests` is sent what 1.10 sent.
    assert!(pass_input(&runtime, "lists").get("test_results").is_none());
    assert_eq!(count(&compiled.diagnostics(), "I205"), 0);
}

#[test]
fn a_check_pass_sees_the_tests_of_its_entities() {
    let (_, runtime, _project) = compile(reading("check"), Some(REPORT_OF_PL1));
    let input = pass_input(&runtime, "p");
    assert_eq!(input["test_results"]["runner"], "fixture");
    assert_eq!(
        input["test_results"]["results"]["pl1"]["tests"][0]["name"], "it works",
        "{input}"
    );
}

#[test]
fn a_compute_pass_sees_the_recorded_report() {
    let (compiled, runtime, _project) = compile(reading("compute"), Some(REPORT));
    let input = pass_input(&runtime, "p");
    assert!(input["test_results"].is_object(), "{input}");
    assert_eq!(count(&compiled.diagnostics(), "I205"), 0);
}

#[test]
fn a_guard_pass_sees_the_recorded_report() {
    let (compiled, runtime, _project) = compile(reading("guard"), Some(REPORT));
    // A compile never runs a guard pass: a write is judged against the project before it.
    assert!(runtime.calls().iter().all(|c| c.export != "__pass_p"));
    let judged = compiled.clone().judged_against(&compiled);
    let input = pass_input(&runtime, "p");
    assert!(input["test_results"].is_object(), "{input}");
    assert_eq!(count(&judged.diagnostics(), "I205"), 0);
}

#[test]
fn a_guard_pass_never_reports_i205_on_a_write() {
    let (compiled, runtime, _project) = compile(reading("guard"), None);
    // The compile of the project says it once.
    assert_eq!(count(&compiled.diagnostics(), "I205"), 1);
    let judged = compiled.clone().judged_against(&compiled);
    let input = pass_input(&runtime, "p");
    assert!(input.get("test_results").is_none(), "{input}");
    assert_eq!(count(&judged.diagnostics(), "I205"), 1);
}

#[test]
fn no_report_is_i205_once() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let compiled = project.compile_in(runtime);

    let diagnostics = compiled.diagnostics();
    assert_eq!(count(&diagnostics, "I205"), 1, "{diagnostics:?}");
}

#[test]
fn several_passes_reading_tests_report_i205_once() {
    let passes = json!([
        {"name": "a", "phase": "check", "reads_tests": true},
        {"name": "b", "phase": "check", "reads_tests": true},
    ]);
    let (compiled, ..) = compile(passes, None);
    let diagnostics = compiled.diagnostics();
    let i205: Vec<_> = diagnostics.iter().filter(|d| d.code == "I205").collect();
    assert_eq!(i205.len(), 1, "{diagnostics:?}");
    assert!(i205[0].message.contains("2 passes"), "{}", i205[0].message);
}

#[test]
fn i205_is_not_repeated_when_a_judged_rule_already_reported_it() {
    let passes = json!([
        {"name": "plan_status", "phase": "compute", "computes": ["plan.status"]},
        {"name": "tests", "phase": "check", "reads_tests": true},
    ]);
    let runtime = Arc::new(
        InProcessRuntime::new()
            .with(move || {
                let mut c = gsd_judged_builder();
                c.raw_category(
                    specforge_extension_sdk::DeclaredCategory::Passes,
                    passes.clone(),
                );
                c
            })
            .answer_raw(
                GSD_JUDGED,
                "__pass_plan_status",
                specforge_wasm::WasmCallResult::Ok(
                    json!({"diagnostics": [], "summary": {}, "values": []})
                        .to_string()
                        .into_bytes(),
                ),
            )
            .answer_raw(
                GSD_JUDGED,
                "__pass_tests",
                specforge_wasm::WasmCallResult::Ok(
                    json!({"diagnostics": [], "summary": {}, "values": []})
                        .to_string()
                        .into_bytes(),
                ),
            ),
    );
    let project = gsd_judged_project("");
    crate::judged::record(&project, "v1.0/03/03-02/TR1", "gsd/E501", "passed");
    let compiled = project.compile_in(runtime as SharedRuntime);
    let diagnostics = compiled.diagnostics();
    assert_eq!(count(&diagnostics, "I205"), 1, "{diagnostics:?}");
}

#[test]
fn an_unusable_report_is_e045_once() {
    let passes = json!([
        {"name": "a", "phase": "check", "reads_tests": true},
        {"name": "b", "phase": "check", "reads_tests": true},
    ]);
    let (compiled, ..) = compile(passes, Some("not json"));
    let diagnostics = compiled.diagnostics();
    assert_eq!(count(&diagnostics, "E045"), 1, "{diagnostics:?}");
    assert_eq!(count(&diagnostics, "I205"), 0);
}

#[test]
fn a_pass_that_does_not_declare_reads_tests_sees_no_report() {
    let passes = json!([{"name": "p", "phase": "check"}]);
    let (compiled, runtime, _project) = compile(passes, Some(REPORT_OF_PL1));
    assert!(pass_input(&runtime, "p").get("test_results").is_none());
    // And a project whose passes never ask reads no report: no I205 without one, no E045 with a bad one.
    let (without, ..) = compile(json!([{"name": "p", "phase": "check"}]), None);
    let (broken, ..) = compile(json!([{"name": "p", "phase": "check"}]), Some("not json"));
    for diagnostics in [
        compiled.diagnostics(),
        without.diagnostics(),
        broken.diagnostics(),
    ] {
        assert_eq!(
            count(&diagnostics, "I205") + count(&diagnostics, "E045"),
            0,
            "{diagnostics:?}"
        );
    }
}

#[test]
fn editing_the_report_reruns_the_check_passes() {
    use specforge_project::UpdateKind;
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let mut session = project.session_in(runtime.clone() as SharedRuntime);
    assert_eq!(count(&session.project().diagnostics(), "I205"), 1);
    runtime.clear_calls();

    // Absent, the report is still an input: creating it re-runs the checks.
    let report = project.root().join("specforge-report.json");
    std::fs::write(&report, REPORT).unwrap();
    let update = session
        .apply(&session.changes([report.as_path()]))
        .expect("the creation of the report is a check input");
    assert_eq!(update.kind, UpdateKind::Checks);
    assert!(pass_input(&runtime, "tests")["test_results"].is_object());
    assert_eq!(count(&session.project().diagnostics(), "I205"), 0);
}
