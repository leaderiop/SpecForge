//! K12: a check, compute or guard pass that declares `reads_tests` receives the recorded test report, and none
//! recorded is I205 once. Pins for the ticket that delivers it; each is ignored until then.

use std::sync::Arc;

use specforge_project::testing::{REPORT, gsd_facts_project, gsd_facts_runtime, pass_input};

fn count(diagnostics: &[specforge_common::Diagnostic], code: &str) -> usize {
    diagnostics.iter().filter(|d| d.code == code).count()
}

#[test]
#[ignore = "pins K12; T6 un-ignores"]
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
#[ignore = "pins K12; T6 un-ignores"]
fn no_report_is_i205_once() {
    let project = gsd_facts_project();
    let runtime = Arc::new(gsd_facts_runtime());
    let compiled = project.compile_in(runtime);

    let diagnostics = compiled.diagnostics();
    assert_eq!(count(&diagnostics, "I205"), 1, "{diagnostics:?}");
}
