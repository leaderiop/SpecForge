//! `@code/core`'s check passes over a project (ADR 0329 D5, D6): the builtin's component loads beside the in-process
//! `Collected` fixture peering it (`@t/node` collects, `@t/spec`'s `truth` cites `test`), and `@t/owner` scopes a
//! citing `step` by a `plan` complete on `status ∈ [complete]`.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::prelude::{
    ContributionsBuilder, ExtensionMeta, FieldType, Vocabulary,
};
use specforge_project::CompiledProject;
use specforge_project::testing::{Collected, NODE, SPEC, TestProject, component_runtime, stored};
use specforge_wasm::testing::InProcessRuntime;

const CODE_CORE: &str = "@code/core";
const OWNER: &str = "@t/owner";
const KEY: &str = "0123456789abcdef0123456789abcdef01234567";
const ADDS: &str = "js:app/test/a.test.js#board > adds";

/// `node`'s store: the test `ADDS`, passed, and its run.
fn adds() -> Value {
    json!([
        {"kind": "test", "id": ADDS, "title": "board > adds",
         "fields": {"package": "app", "file": "test/a.test.js", "suite": ["board"], "name": "adds", "status": "passed"}},
        {"kind": "test_run", "id": format!("run:{ADDS}"), "fields": {"test": ADDS, "status": "passed"}},
    ])
}

/// `@t/owner`: a `plan` complete on `status ∈ [complete]`, and a `step` it scopes that cites tests in `proof`.
fn owner() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(OWNER, "1.0.0"));
    c.peer(CODE_CORE, "^1.0");
    c.kind("plan", |k| {
        k.complete_when("status", &["complete"])
            .field("status", |f| {
                f.vocabulary(&Vocabulary::new(&["open", "complete"]));
            });
    });
    c.kind("step", |k| {
        k.scoped_by("plan")
            .field("plan", |f| {
                f.field_type(FieldType::Reference).target_kind("plan");
            })
            .field("proof", |f| {
                f.field_type(FieldType::ReferenceList).target_kind("test");
            });
    });
    c
}

/// A project enabling `@code/core`, the fixture peering it and `@t/owner`, with `files`.
fn project(files: &[(&str, &str)]) -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&[CODE_CORE, NODE, SPEC, OWNER])
        .installing_enabled()
        .files(files)
}

/// `p` compiled with `@code/core`'s component beside the fixture.
fn compiled(p: &TestProject) -> CompiledProject {
    let beside = InProcessRuntime::new()
        .with(owner)
        .beside(component_runtime());
    p.compile_in(Arc::new(Collected::new().peering(CODE_CORE).over(beside)))
}

/// `@code/core`'s findings: `(code, line, suggestion)`.
fn found(p: &CompiledProject) -> Vec<(String, usize, Option<String>)> {
    p.diagnostics()
        .iter()
        .filter(|d| d.code.as_str().starts_with("code/"))
        .map(|d| {
            (
                d.code.to_string(),
                d.span.as_ref().map_or(0, |s| s.start_line),
                d.suggestion.clone(),
            )
        })
        .collect()
}

/// A truth citing `cited` in `proof`, its reference on line 2.
fn citing(cited: &str) -> String {
    format!("truth T1 \"One\" {{\n  proof [\"{cited}\"]\n}}\n")
}

#[specforge_test_macros::test(
    behavior = "cc_check_fact_refs",
    verify = "a fact ref to a missing test is code/E101"
)]
fn a_fact_ref_to_a_missing_test_is_code_e101() {
    let p = project(&[("truths.spec", &citing("js:app/test/a.test.js#board > add"))]);
    stored(p.root(), "node", KEY, adds());

    let compiled = compiled(&p);

    assert_eq!(
        found(&compiled),
        [(
            "code/E101".to_string(),
            2,
            Some(format!("did you mean `{ADDS}`?"))
        )],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(
    behavior = "cc_check_fact_refs",
    verify = "a fact ref with no facts collected is code/W102"
)]
fn a_fact_ref_with_no_facts_collected_is_code_w102() {
    let p = project(&[("truths.spec", &citing(ADDS))]);

    let compiled = compiled(&p);

    assert_eq!(
        found(&compiled),
        [("code/W102".to_string(), 2, None)],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(
    behavior = "cc_check_fact_refs",
    verify = "a fact ref of a pending owner is unchecked"
)]
fn a_fact_ref_of_a_pending_owner_is_unchecked() {
    let plans = format!(
        "plan open \"Open\" {{\n  status open\n}}\n\nplan done \"Done\" {{\n  status complete\n}}\n\n\
         step later \"Later\" {{\n  plan open\n  proof [\"{ADDS}\"]\n}}\n\n\
         step now \"Now\" {{\n  plan done\n  proof [\"{ADDS}\"]\n}}\n"
    );
    let p = project(&[("plans.spec", &plans)]);

    let compiled = compiled(&p);

    // Only the complete plan's step is checked: its reference is on line 16.
    assert_eq!(
        found(&compiled),
        [("code/W102".to_string(), 16, None)],
        "{:?}",
        compiled.diagnostics()
    );
}

#[specforge_test_macros::test(
    behavior = "cc_check_fact_refs",
    verify = "a held fact ref links and reports nothing"
)]
fn a_held_fact_ref_links_and_reports_nothing() {
    let p = project(&[("truths.spec", &citing(ADDS))]);
    stored(p.root(), "node", KEY, adds());

    let compiled = compiled(&p);

    assert_eq!(found(&compiled), [], "{:?}", compiled.diagnostics());
    assert!(
        compiled
            .graph()
            .edges_from("T1")
            .iter()
            .any(|e| e.target.as_str() == ADDS),
        "T1 links the test"
    );
}
