//! K17 (ADR 0297): a judged rule's finding is never an error a write adds. A failing verdict, a new judged entity
//! and an edit that stales a passing verdict are written; the finding is reported. The fixture is `@gsd/core`'s
//! judged rules ([`specforge_project::testing::gsd_judged_project`]).

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::Diagnostic;
use specforge_ops::authoring::apply::{HostJudgedRules, JudgedRules};
use specforge_ops::authoring::edit::{self as typed_edit, Edit, Input};
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{TestProject, gsd_judged_project, gsd_judged_runtime};
use specforge_test_macros::test as specforge_test;

use crate::plans::revision;
use crate::verdicts::gsd;

const TRUTH: &str = "v1.0/03/03-02/TR1";

fn compile(p: &TestProject) -> specforge_project::CompiledProject {
    p.compile_in(Arc::new(gsd_judged_runtime()) as SharedRuntime)
}

/// The `set_verdict` of `rule` on `entity`, on the basis the host digests now.
pub(crate) fn verdict_on(
    p: &TestProject,
    entity: &str,
    rule: &str,
    outcome: &str,
    judge: &str,
) -> Value {
    let compiled = compile(p);
    let view = ProjectView::of(&compiled);
    let basis = HostJudgedRules::of(&view)
        .digests(&view, entity, rule, &[])
        .unwrap();
    json!([{"op": "set_verdict", "entity": entity, "rule": rule, "outcome": outcome,
        "judge": judge, "model": "m", "findings": [], "cited": [], "basis": basis}])
}

/// Apply typed `edits` as the CLI without a terminal: the refusal's code and message, or nothing.
fn typed(p: &TestProject, edits: &[Edit]) -> Result<(), (String, String)> {
    let compiled = compile(p);
    typed_edit::run(
        &ProjectView::of(&compiled),
        edits,
        false,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .map(|_| ())
    .map_err(|r| (r.error.code.to_string(), r.error.message))
}

fn create(kind: &str, id: &str, fields: &[(&str, &str)]) -> Edit {
    Edit::Create {
        kind: kind.into(),
        id: id.into(),
        title: "New".into(),
        file: "a.spec".into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), Input::One(v.to_string())))
            .collect(),
        verify: Vec::new(),
    }
}

/// What the project reports under `code` whose message names `entity`.
fn findings(p: &TestProject, code: &str, entity: &str) -> Vec<Diagnostic> {
    compile(p)
        .diagnostics()
        .into_iter()
        .filter(|d| d.code == code && d.message.contains(entity))
        .collect()
}

#[specforge_test(
    behavior = "judge_rule_by_fresh_verdict",
    verify = "a failing verdict on an error rule is recorded"
)]
fn a_failing_verdict_on_an_error_rule_records() {
    let p = gsd_judged_project("");
    let ops = verdict_on(&p, TRUTH, "gsd/E501", "failed", "gsd-verifier");
    assert_eq!(gsd(&p, ops, false), "applied");
    let reported = findings(&p, "gsd/E501", "TR1");
    assert!(
        reported
            .iter()
            .any(|d| d.message.ends_with("is 'failed' (0 findings)")),
        "{reported:?}"
    );
    let compiled = compile(&p);
    let judgement = compiled.judgements().get(TRUTH, "gsd/E501").unwrap();
    assert_eq!(judgement.state, "fresh");
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a judged rules finding is no error a write adds"
)]
fn a_typed_edit_creating_a_judged_target_is_written() {
    let p = gsd_judged_project("");
    let truth = create("truth", "TR2", &[("plan", "03-02")]);
    let spec = create("phase_spec", "SPEC", &[("phase", "03")]);
    typed(&p, &[truth, spec]).unwrap_or_else(|(code, message)| panic!("{code}: {message}"));
    let on_truth = findings(&p, "gsd/E501", "TR2");
    assert!(
        on_truth.iter().any(|d| d.message.ends_with("no verdict")),
        "{on_truth:?}"
    );
    let on_spec = findings(&p, "gsd/E202", "SPEC");
    assert!(
        on_spec.iter().any(|d| d.message.ends_with("no verdict")),
        "{on_spec:?}"
    );
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a judged rules finding is no error a write adds"
)]
fn a_command_plan_creating_a_judged_target_is_applied() {
    let p = gsd_judged_project("");
    let ops = json!([{"op": "create", "kind": "truth", "id": "TR2", "title": "New", "file": "a.spec",
        "revision": revision(&p, "a.spec"), "fields": {"plan": "03-02"}}]);
    assert_eq!(gsd(&p, ops, false), "applied");
    let reported = findings(&p, "gsd/E501", "TR2");
    assert!(
        reported.iter().any(|d| d.message.ends_with("no verdict")),
        "{reported:?}"
    );
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a judged rules finding is no error a write adds"
)]
fn an_edit_that_stales_a_passing_verdict_is_written() {
    let p = gsd_judged_project("").file("src/auth/other.ts", "export const other = 1;\n");
    let ops = verdict_on(&p, TRUTH, "gsd/E501", "passed", "gsd-verifier");
    assert_eq!(gsd(&p, ops, false), "applied");
    assert!(findings(&p, "gsd/E501", "TR1").is_empty());
    let restage = crate::edit::set(
        "v1.0/03/03-02",
        "files_modified",
        Input::Many(vec!["src/auth/other.ts".into()]),
    );
    typed(&p, &[restage]).unwrap_or_else(|(code, message)| panic!("{code}: {message}"));
    let reported = findings(&p, "gsd/E501", "TR1");
    assert!(
        reported.iter().any(|d| d.message.contains("stale")),
        "{reported:?}"
    );
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a judged rules finding is no error a write adds"
)]
fn a_non_judged_error_beside_a_judged_one_is_still_e080() {
    // A truth (judged: no verdict) and a plan naming no file: only the second refuses, and the message names it alone.
    let p = gsd_judged_project("");
    let truth = create("truth", "TR2", &[("plan", "03-02")]);
    let dangling = create("truth", "TR3", &[("plan", "03-99")]);
    let (code, message) = typed(&p, &[truth, dangling]).unwrap_err();
    assert_eq!(code, "E080");
    assert!(message.contains("E003"), "{message}");
    assert!(!message.contains("gsd/E501"), "{message}");
    assert!(!message.contains("no verdict"), "{message}");
}

#[specforge_test(
    behavior = "validate_typed_edit",
    verify = "a judged rules finding is no error a write adds"
)]
fn a_judged_finding_already_reported_is_not_reported_again() {
    let p = gsd_judged_project("");
    let compiled = compile(&p);
    let view = ProjectView::of(&compiled);
    let edit = Edit::Retitle {
        entity_id: "v1.0/03/03-03".into(),
        title: "Other".into(),
    };
    let edited = typed_edit::run(
        &view,
        &[edit],
        true,
        &specforge_ops::authoring::apply::testing::origin(),
        specforge_ops::testing::caller(),
    )
    .unwrap();
    let added = serde_json::to_string(&edited.report.added).unwrap();
    assert!(!added.contains("no verdict"), "{added}");
}

#[specforge_test(
    behavior = "validate_post_migration_integrity",
    verify = "a migration counts no judged finding"
)]
fn a_migration_counts_no_judged_finding() {
    use specforge_ops::authoring::migrate::{MigrationInput, judge};
    let p = gsd_judged_project("");
    let pre = specforge_project::testing::compile_at(
        p.root(),
        Some(Arc::new(gsd_judged_runtime()) as SharedRuntime),
    );
    let text = format!(
        "{}truth TR2 \"New\" {{\n  plan 03-02\n}}\n",
        std::fs::read_to_string(p.root().join("a.spec")).unwrap()
    );
    let planned = std::collections::BTreeMap::from([("a.spec".to_string(), text)]);
    let input = MigrationInput {
        from: "0.9".into(),
        to: "1.0".into(),
        files: vec![],
    };
    let verdict = judge(&pre, &planned, &input);
    assert!(verdict.compared);
    assert_eq!(verdict.added_errors, 0, "{:?}", verdict.post_diagnostics);
}
