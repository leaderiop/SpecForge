//! `record_context` (ADR 0356 D11): a context route's pack recorded on an entity by the host, over the GSD-shaped
//! executor route that records the plan's `context_pack`: the value and its ledger line, the refusals, and a verdict
//! the record leaves fresh.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::authoring::apply::{
    HostJudgedRules, JudgedRules, Origin, SystemCalendar, apply_plan_with,
};
use specforge_ops::read::context::{ContextRequest, context};
use specforge_ops::read::export::Format;
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    CODE_CORE, CONTEXT_PLAN, GSD_CONTEXT_SPEC, GSD_JUDGED, TestProject, gsd_context_builder,
    gsd_context_project, gsd_routed_project_over, gsd_routed_serving,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_protocol_types::EditPlan;
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

/// `@gsd/core` with the context routes, and `requirement-pack`, which reads a requirement and records it in a plan's
/// `context_pack` through the plan's requirements: a plan with none or with two has no one subject.
fn runtime() -> InProcessRuntime {
    gsd_routed_serving(|| {
        let mut c = gsd_context_builder();
        c.context("requirement-pack", |r| {
            r.subject("requirement")
                .budget(2000)
                .core("requirement", "self")
                .record("plan.context_pack", "self.requirements")
                .stale("W524");
        });
        c
    })
}

fn compile(project: &TestProject) -> CompiledProject {
    project.compile_in(Arc::new(runtime()) as SharedRuntime)
}

/// Apply `ops` asked by `origin`: `applied`, or the refusal's reason (its code when it names none).
fn apply(project: &TestProject, origin: &Origin, ops: Value) -> String {
    let compiled = compile(project);
    let view = ProjectView::of(&compiled);
    let rules: Arc<dyn JudgedRules> = Arc::new(HostJudgedRules::of(&view));
    let plan: EditPlan = serde_json::from_value(json!({ "ops": ops })).unwrap();
    match apply_plan_with(
        &view,
        &plan,
        origin,
        false,
        specforge_ops::testing::caller(),
        &rules,
        &SystemCalendar,
    ) {
        Ok(_) => "applied".to_string(),
        Err(refusal) => {
            let error = refusal.into_error();
            let data = error.data.map_or(Value::Null, |data| *data);
            data["reason"]
                .as_str()
                .map_or_else(|| error.code.to_string(), str::to_string)
        }
    }
}

fn gsd() -> Origin {
    Origin::Command {
        extension: GSD_JUDGED.to_string(),
    }
}

fn record(entity: &str, route: &str, max_tokens: Option<u32>) -> Value {
    let mut op = json!({"op": "record_context", "entity": entity, "route": route});
    if let Some(max_tokens) = max_tokens {
        op["max_tokens"] = json!(max_tokens);
    }
    json!([op])
}

/// The digest the executor's pack of the plan has at `max_tokens`.
fn digest(project: &TestProject, max_tokens: Option<usize>) -> String {
    let compiled = compile(project);
    let request = ContextRequest {
        route: "gsd/executor",
        subject: CONTEXT_PLAN,
        max_tokens,
        format: Format::Context,
    };
    context(&ProjectView::of(&compiled), &request)
        .unwrap_or_else(|refusal| panic!("context refused: {:?}", refusal.error))
        .digest
}

/// The spec file, its runs of spaces one (the formatter aligns a block's values).
fn spec(project: &TestProject) -> String {
    let text = std::fs::read_to_string(project.root().join("a.spec")).unwrap();
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[specforge_test(
    behavior = "record_context_pack",
    verify = "a recorded pack holds the route, the digest the pack read gives and its budget, with its ledger line"
)]
fn a_command_of_the_routes_extension_records_its_pack() {
    let project = gsd_context_project();
    let expected = digest(&project, Some(5000));
    assert_eq!(
        apply(
            &project,
            &gsd(),
            record("02-01", "gsd/executor", Some(5000))
        ),
        "applied"
    );
    let written = spec(&project);
    assert!(written.contains("context_pack {"), "{written}");
    assert!(written.contains("route \"gsd/executor\""), "{written}");
    assert!(
        written.contains(&format!("digest \"{expected}\"")),
        "{written}"
    );
    assert!(written.contains("max_tokens 5000"), "{written}");
    let ledger = std::fs::read_to_string(project.root().join("specforge-harness")).unwrap();
    assert!(
        ledger.contains(&format!("{CONTEXT_PLAN} context_pack ")),
        "{ledger}"
    );
    // The record moves no digest of the pack it records, and the project holds it without E083.
    assert_eq!(digest(&project, Some(5000)), expected);
    let reported = compile(&project).diagnostics().to_vec();
    assert!(!reported.iter().any(|d| d.code == "E083"), "{reported:?}");
    // With no budget, the route's own; a core surface records too.
    assert_eq!(
        apply(
            &project,
            &Origin::cli(false),
            record(CONTEXT_PLAN, "gsd/executor", None)
        ),
        "applied"
    );
    assert!(
        spec(&project).contains("max_tokens 6000"),
        "{}",
        spec(&project)
    );
    assert_eq!(spec(&project).matches("context_pack {").count(), 1);
}

#[specforge_test(
    behavior = "record_context_pack",
    verify = "a record of an unknown route, a kind it does not record, from another extension or with no subject is refused record_refused"
)]
fn a_record_is_refused_for_an_unknown_route_an_unrecorded_kind_another_origin_or_no_subject() {
    // 02-02 names no requirement; 02-01 names two.
    let extra = "\nplan 02-02 \"Nothing required\" {\n  phase 02\n}\n";
    let project = gsd_routed_project_over(
        &format!("{GSD_CONTEXT_SPEC}{extra}"),
        &[GSD_JUDGED, CODE_CORE],
    )
    .file("CLAUDE.md", "")
    .file("src/store.js", "")
    .file("src/cli.js", "")
    .file("test/store.test.js", "");
    let before = spec(&project);
    let other = Origin::Command {
        extension: "@acme/other".to_string(),
    };
    for (origin, ops, reason) in [
        (
            gsd(),
            record(CONTEXT_PLAN, "gsd/nothing", None),
            "unknown_route",
        ),
        (
            gsd(),
            record("v1.0/02", "gsd/executor", None),
            "not_recorded",
        ),
        (
            gsd(),
            record(CONTEXT_PLAN, "gsd/plan-checker", None),
            "not_recorded",
        ),
        (other, record(CONTEXT_PLAN, "gsd/executor", None), "origin"),
        (
            gsd(),
            record("v1.0/02/02-02", "gsd/requirement-pack", None),
            "no_subject",
        ),
        (
            gsd(),
            record(CONTEXT_PLAN, "gsd/requirement-pack", None),
            "several_subjects",
        ),
        (
            gsd(),
            record(CONTEXT_PLAN, "gsd/executor", Some(1)),
            "budget",
        ),
        (
            gsd(),
            record("v1.0/02/09-09", "gsd/executor", None),
            "invalid_plan",
        ),
    ] {
        assert_eq!(apply(&project, &origin, ops), reason);
    }
    assert_eq!(spec(&project), before, "nothing is written");
}

#[specforge_test(
    behavior = "record_context_pack",
    verify = "recording a pack stales no verdict"
)]
fn recording_a_pack_leaves_a_fresh_verdict_fresh() {
    let project = gsd_context_project();
    let compiled = compile(&project);
    let view = ProjectView::of(&compiled);
    let basis = HostJudgedRules::of(&view)
        .digests(&view, CONTEXT_PLAN, "gsd/W311", &[])
        .unwrap();
    let verdict = json!([{"op": "set_verdict", "entity": CONTEXT_PLAN, "rule": "gsd/W311", "outcome": "passed",
        "judge": "gsd-plan-checker", "model": "m", "findings": [], "cited": [], "basis": basis}]);
    assert_eq!(apply(&project, &gsd(), verdict), "applied");
    let state = |project: &TestProject| {
        compile(project)
            .judgements()
            .get(CONTEXT_PLAN, "gsd/W311")
            .map(|judgement| judgement.state.clone())
    };
    assert_eq!(state(&project).as_deref(), Some("fresh"));
    assert_eq!(
        apply(&project, &gsd(), record(CONTEXT_PLAN, "gsd/executor", None)),
        "applied"
    );
    assert!(spec(&project).contains("context_pack {"));
    assert_eq!(state(&project).as_deref(), Some("fresh"));
}
