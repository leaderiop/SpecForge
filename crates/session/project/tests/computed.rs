//! A compute pass gives a field its value, held beside the graph and read by the rules; one written by hand is
//! E082 (ADR 0287, behavior `compute_field_values`).

use std::sync::Arc;

use serde_json::json;
use specforge_common::Diagnostic;
use specforge_project::testing::{GSD_EXTENSION, TestProject, gsd_history_runtime};
use specforge_test::prelude::*;

pub(crate) const BASE: &str =
    "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\n";

/// A project over `@test/gsd` whose spec is `BASE` and `spec`.
pub(crate) fn project(spec: &str) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[GSD_EXTENSION])
        .installing_enabled()
        .file("a.spec", format!("{BASE}{spec}"))
}

pub(crate) fn with_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| d.code == code).collect()
}

const VERIFICATION: &str = "verification V \"V\" {\n  phase 03\n}\n";

#[specforge_test(
    behavior = "compute_field_values",
    verify = "a compute pass's value joins the snapshot beside the graph, which keeps the authored values"
)]
fn a_compute_pass_gives_a_value_the_rules_read() {
    let runtime = Arc::new(gsd_history_runtime(
        json!([{"entity": "v1.0/03/V", "field": "status", "value": "passed"}]),
    ));
    let compiled = project(VERIFICATION).compile_in(runtime);
    let diagnostics = compiled.diagnostics();
    assert!(
        with_code(&diagnostics, "W165").is_empty(),
        "{diagnostics:?}"
    );
    let (record, _) = compiled.entities().get("v1.0/03/V").unwrap();
    let status = record
        .fields
        .iter()
        .find(|f| f.key == "status")
        .expect("the snapshot holds status");
    assert_eq!(status.text, "passed");
    let node = compiled.graph().node("v1.0/03/V").unwrap();
    assert!(
        node.fields.get("status").is_none(),
        "the graph holds authored values only"
    );
}

#[specforge_test(
    behavior = "refuse_written_computed",
    verify = "a computed field written in a spec file is E082 at the value, and the computed value wins"
)]
fn a_computed_field_written_by_hand_is_e082() {
    let runtime = Arc::new(gsd_history_runtime(
        json!([{"entity": "v1.0/03/V", "field": "status", "value": "passed"}]),
    ));
    let spec = "verification V \"V\" {\n  phase 03\n  status failed\n}\n";
    let compiled = project(spec).compile_in(runtime);
    let diagnostics = compiled.diagnostics();
    let e082 = with_code(&diagnostics, "E082");
    assert_eq!(e082.len(), 1, "{diagnostics:?}");
    assert!(e082[0].message.contains("status"), "{e082:?}");
    let (record, _) = compiled.entities().get("v1.0/03/V").unwrap();
    let status = record.fields.iter().find(|f| f.key == "status").unwrap();
    assert_eq!(status.text, "passed");

    // A computed member of a block list item.
    let spec = "deliverable D \"D\" {\n  checks [ { kind unit  ref \"t\"  status pass } ]\n}\n";
    let diagnostics = project(spec)
        .compile_in(Arc::new(gsd_history_runtime(json!([]))))
        .diagnostics();
    let e082 = with_code(&diagnostics, "E082");
    assert_eq!(e082.len(), 1, "{diagnostics:?}");
    assert!(e082[0].message.contains("checks[0].status"), "{e082:?}");
}

#[specforge_test(
    behavior = "compute_field_values",
    verify = "a value the pass may not give is W165 and dropped"
)]
fn a_value_the_pass_may_not_give_is_w165() {
    let values = json!([
        {"entity": "v1.0/03/V", "field": "phase", "value": "03"},
        {"entity": "ghost", "field": "status", "value": "passed"},
        {"entity": "v1.0/03/V", "field": "re_verification", "value": ["a", "b"]},
    ]);
    let compiled = project(VERIFICATION).compile_in(Arc::new(gsd_history_runtime(values)));
    let diagnostics = compiled.diagnostics();
    assert_eq!(with_code(&diagnostics, "W165").len(), 3, "{diagnostics:?}");
    let (record, _) = compiled.entities().get("v1.0/03/V").unwrap();
    assert!(!record.fields.iter().any(|f| f.key == "re_verification"));
}

#[specforge_test(
    behavior = "compute_field_values",
    verify = "a computed field or member is never required and never scaffolded"
)]
fn no_computed_field_is_required_or_scaffolded() {
    let spec = "deliverable D \"D\" {\n  checks [ { kind unit  ref \"t\" } ]\n}\n";
    let runtime = gsd_history_runtime(json!([
        {"entity": "D", "field": "checks[0].status", "value": "pass"},
    ]));
    let compiled = project(&format!("{VERIFICATION}\n{spec}")).compile_in(Arc::new(runtime));
    let diagnostics = compiled.diagnostics();
    assert!(
        with_code(&diagnostics, "E006").is_empty(),
        "{diagnostics:?}"
    );
    assert!(
        with_code(&diagnostics, "E082").is_empty(),
        "{diagnostics:?}"
    );
    assert!(
        with_code(&diagnostics, "W165").is_empty(),
        "{diagnostics:?}"
    );
    let (record, _) = compiled.entities().get("D").unwrap();
    let checks = record.fields.iter().find(|f| f.key == "checks").unwrap();
    let status = checks.members.iter().find(|m| m.key == "checks[0].status");
    assert_eq!(status.map(|m| m.text.as_str()), Some("pass"));
}
