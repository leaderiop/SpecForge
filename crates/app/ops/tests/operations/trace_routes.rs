//! `trace` along the route an extension declares (ADR 0330): the requirement thread of the GSD-shaped
//! `@gsd/core`, its one-line and tree renderings, complete and partial threads, the reach kept for every other kind,
//! a named route, and the step an unloaded optional peer skips.

use std::sync::Arc;

use serde_json::json;
use specforge_extension_sdk::DeclaredCategory;
use specforge_ops::authoring::apply::{HostJudgedRules, JudgedRules};
use specforge_ops::read::trace::{
    Annotated, Follow, Partial, ROUTE_OF_ANOTHER_KIND, RouteTrace, Target, Trace, TraceField,
    TraceOutcome, UNKNOWN_ROUTE, chain, trace,
};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    CODE_CORE, GSD_JUDGED, GSD_JUDGED_SPEC, GSD_ROUTED_SPEC, ROUTED_TEST, TestProject,
    gsd_judged_project, gsd_judged_runtime, gsd_routed_builder, gsd_routed_project,
    gsd_routed_project_over, gsd_routed_runtime, gsd_routed_serving, stored,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::testing::InProcessRuntime;

use crate::view_support::plain;

/// The thread every test starts from: one plan, task, commit, test (passing) and truth (no verdict).
const WHOLE: &str = "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing";

/// `project` compiled, `runtime` serving `@gsd/core` with the route and `@code/core`.
fn compiled_in(project: TestProject, runtime: InProcessRuntime) -> (TestProject, CompiledProject) {
    let compiled = project.compile_in(Arc::new(runtime) as SharedRuntime);
    (project, compiled)
}

/// `spec` compiled over `@gsd/core` with the route and `@code/core`, the test it cites collected.
fn compiled(spec: &str) -> (TestProject, CompiledProject) {
    compiled_in(gsd_routed_project(spec), gsd_routed_runtime())
}

/// The trace of `id` as `follow` asks, with its notices' codes.
fn traced(
    compiled: &CompiledProject,
    target: Target,
    follow: Follow,
) -> (TraceOutcome, Vec<String>) {
    let (outcome, notices) = trace(&ProjectView::of(compiled), target, follow)
        .unwrap_or_else(|refusal| panic!("trace refused: {:?}", refusal.error))
        .into_value_and_notices();
    (outcome, notices.into_iter().map(|d| d.code).collect())
}

/// The route trace of `id` along its kind's route.
fn thread_of(compiled: &CompiledProject, id: &str) -> RouteTrace {
    match traced(compiled, Target::Entity(id), Follow::Declared).0 {
        TraceOutcome::One(document) => match document.trace {
            Trace::Route(route) => *route,
            Trace::Chain(chain) => panic!("traced by its reach: {chain:?}"),
        },
        TraceOutcome::Every(_) => unreachable!("one entity"),
    }
}

/// The thread of `spec`'s requirement AUTH-07.
fn thread(spec: &str) -> RouteTrace {
    let (_project, compiled) = compiled(spec);
    thread_of(&compiled, "AUTH-07")
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "trace follows the route its kind declares, step by step, each match under the match it came from"
)]
fn a_requirement_is_traced_along_its_route_in_one_line() {
    let route = thread(GSD_ROUTED_SPEC);
    assert_eq!(route.route, "gsd/requirement-thread");
    assert_eq!(route.text, WHOLE);
    assert_eq!(route.thread(Partial::No).as_deref(), Some(WHOLE));
    assert!(route.complete && route.missing.is_empty(), "{route:?}");
    let parents: Vec<(&str, &str, &str)> = route
        .steps
        .iter()
        .flat_map(|step| {
            step.matches
                .iter()
                .map(|m| (step.name.as_str(), m.entity_id.as_str(), m.parent.as_str()))
        })
        .collect();
    assert_eq!(
        parents,
        [
            ("plan", "v1.0/03/03-02", "AUTH-07"),
            ("task", "v1.0/03/03-02/T1", "v1.0/03/03-02"),
            ("commit", "5e2a91c0d3b4", "v1.0/03/03-02/T1"),
            ("test", ROUTED_TEST, "AUTH-07"),
            ("truth", "v1.0/03/03-02/TR1", "AUTH-07"),
        ]
    );
    let document = serde_json::to_value(&route).unwrap();
    assert_eq!(document["steps"][3]["from"], "start");
    assert_eq!(document["steps"][1]["from"], "plan");
    assert_eq!(
        document["steps"][2]["matches"][0]["annotations"][0],
        json!({"source": "field", "name": "linked_by", "values": ["Gsd-Task"], "text": "(Gsd-Task)"})
    );
    assert!(document.get("skipped").is_none(), "{document}");
}

/// The basis a `set_verdict` of E501 on the truth TR1 is judged over now.
fn verdict_on_the_truth(project: &TestProject) -> serde_json::Value {
    let compiled = project.compile_in(Arc::new(gsd_routed_runtime()) as SharedRuntime);
    let view = ProjectView::of(&compiled);
    let truth = "v1.0/03/03-02/TR1";
    let basis = HostJudgedRules::of(&view)
        .digests(&view, truth, "gsd/E501", &[])
        .unwrap();
    json!([{"op": "set_verdict", "entity": truth, "rule": "gsd/E501", "outcome": "passed",
        "judge": "gsd-verifier", "model": "m", "findings": [], "cited": [], "basis": basis}])
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "an annotation prints a field, a value reached through a selector, or a judgement's outcome and freshness"
)]
fn a_fresh_verdict_reads_outcome_and_freshness() {
    let project = gsd_routed_project(GSD_ROUTED_SPEC);
    let ops = verdict_on_the_truth(&project);
    assert_eq!(
        crate::verdicts::gsd_in(&project, gsd_routed_runtime(), ops, false),
        "applied"
    );
    let compiled = project.compile_in(Arc::new(gsd_routed_runtime()) as SharedRuntime);
    let route = thread_of(&compiled, "AUTH-07");
    assert!(
        route.text.ends_with(" → TR1 passed, fresh"),
        "{}",
        route.text
    );
    let Annotated::Judgement { state, outcome, .. } = &route.steps[4].matches[0].annotations[0]
    else {
        panic!("the truth step prints a judgement");
    };
    assert_eq!(
        (state.as_deref(), outcome.as_deref()),
        (Some("fresh"), Some("passed"))
    );
    // The code the truth's plan modifies changes: the verdict is stale.
    project.write("src/auth/session.ts", "export const session = 2;\n");
    let compiled = project.compile_in(Arc::new(gsd_routed_runtime()) as SharedRuntime);
    let route = thread_of(&compiled, "AUTH-07");
    assert!(
        route.text.ends_with(" → TR1 passed, stale"),
        "{}",
        route.text
    );
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "an annotation prints a field, a value reached through a selector, or a judgement's outcome and freshness"
)]
fn an_annotation_reads_a_field_of_the_entities_a_selector_reaches() {
    // The route as declared, its plan step also printing the `linked_by` of its tasks' commits.
    let runtime = gsd_routed_serving(|| {
        let mut c = gsd_routed_builder();
        let mut routes = serde_json::to_value(gsd_routed_builder().declaration().routes).unwrap();
        routes[0]["steps"][0]["annotations"] =
            json!([{"field": "linked_by", "of": "<task.plan<commit.task", "absent": "no commit"}]);
        c.raw_category(DeclaredCategory::Routes, routes);
        c
    });
    let (_project, compiled) = compiled_in(gsd_routed_project(GSD_ROUTED_SPEC), runtime);
    let route = thread_of(&compiled, "AUTH-07");
    assert!(
        route
            .text
            .starts_with("AUTH-07 → plan 03-02 (Gsd-Task) → T1 → "),
        "{}",
        route.text
    );
}

#[specforge_test(
    behavior = "render_trace_thread",
    verify = "several matches of a step print as branches of a tree"
)]
fn several_matches_render_as_branches() {
    let spec = format!(
        "{GSD_ROUTED_SPEC}\ncommit 7b3c11aa09fe \"Another\" {{\n  task v1.0/03/03-02/T1\n  linked_by \"Gsd-Task\"\n}}\n"
    );
    let route = thread(&spec);
    let tree = "AUTH-07
  → plan 03-02
    → T1
      → 5e2a91c0d3b4 (Gsd-Task)
      → 7b3c11aa09fe (Gsd-Task)
  → js:app/test/auth.test.js#keeps a session (passed)
  → TR1 missing";
    assert_eq!(route.text, tree);
    assert_eq!(route.thread(Partial::No).as_deref(), Some(tree));
}

/// [`GSD_ROUTED_SPEC`] without the commit.
fn no_commit() -> String {
    GSD_ROUTED_SPEC.replace(
        "commit 5e2a91c0d3b4 \"Keep the session\" {\n  task v1.0/03/03-02/T1\n  linked_by \"Gsd-Task\"\n}\n",
        "",
    )
}

#[specforge_test(
    behavior = "render_trace_thread",
    verify = "a thread with at most one match per step prints on one line, a step with none as no <step>"
)]
fn a_step_with_no_match_prints_no_and_its_descendants_nothing() {
    assert_eq!(
        thread(&no_commit()).text,
        "AUTH-07 → plan 03-02 → T1 → no commit → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
    );
    let no_plan = GSD_ROUTED_SPEC.replace(
        "  requirements [AUTH-07]\n  files_modified",
        "  files_modified",
    );
    let route = thread(&no_plan);
    assert_eq!(
        route.text,
        "AUTH-07 → no plan → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
    );
    assert_eq!(route.missing, ["plan"]);
}

#[specforge_test(behavior = "follow_trace_route")]
fn an_absent_fact_prints_the_absent_text() {
    // The cited test is collected, with no status recorded.
    let project = gsd_routed_project_over(GSD_ROUTED_SPEC, &[GSD_JUDGED, CODE_CORE]);
    let unrun = json!([{"kind": "test", "id": ROUTED_TEST, "fields": {"name": "keeps a session"}}]);
    stored(
        project.root(),
        "node",
        "fedcba9876543210fedcba9876543210fedcba98",
        unrun,
    );
    let (_project, compiled) = compiled_in(project, gsd_routed_runtime());
    let route = thread_of(&compiled, "AUTH-07");
    assert!(
        route
            .text
            .contains(&format!(" → {ROUTED_TEST} (not run) → ")),
        "{}",
        route.text
    );
    // An absent text is not a value.
    assert!(matches!(
        &route.steps[3].matches[0].annotations[0],
        Annotated::Field { values, .. } if values.is_empty()
    ));
}

#[specforge_test(
    behavior = "render_trace_thread",
    verify = "only a complete thread prints by default, its dead-end branches left out, and a partial one with --partial"
)]
fn an_incomplete_thread_prints_one_line_by_default_and_whole_with_partial() {
    let route = thread(&no_commit());
    assert_eq!(route.thread(Partial::No), None);
    assert_eq!(
        route.incomplete_line(),
        "AUTH-07: incomplete thread (no commit); print it with: specforge trace AUTH-07 --partial"
    );
    assert!(!route.complete);
    assert_eq!(route.missing, ["commit"]);
    assert!(
        route
            .thread(Partial::Yes)
            .unwrap()
            .contains("→ T1 → no commit →")
    );
}

#[specforge_test(behavior = "render_trace_thread")]
fn a_complete_thread_leaves_dead_end_branches_out() {
    let spec = format!("{GSD_ROUTED_SPEC}\ntask T2 \"Other\" {{\n  plan 03-02\n}}\n");
    let route = thread(&spec);
    assert!(route.complete);
    assert_eq!(route.thread(Partial::No).as_deref(), Some(WHOLE));
    assert_eq!(
        route.thread(Partial::Yes).unwrap(),
        "AUTH-07
  → plan 03-02
    → T1
      → 5e2a91c0d3b4 (Gsd-Task)
    → T2
      → no commit
  → js:app/test/auth.test.js#keeps a session (passed)
  → TR1 missing"
    );
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a kind with no route is traced by the generic reach, unchanged"
)]
fn a_kind_without_a_route_keeps_the_generic_reach_byte_for_byte() {
    let (_project, compiled) = compiled(GSD_ROUTED_SPEC);
    let view = ProjectView::of(&compiled);
    let declared = plain(trace(&view, Target::Entity("v1.0/03/03-02"), Follow::Declared).unwrap());
    let reach = plain(chain(&view, "v1.0/03/03-02").unwrap());
    assert_eq!(
        serde_json::to_string(&declared).unwrap(),
        serde_json::to_string(&reach).unwrap()
    );
    // A project whose extensions declare no route traces every entity as before.
    let project = gsd_judged_project("");
    let compiled = project.compile_in(Arc::new(gsd_judged_runtime()) as SharedRuntime);
    let view = ProjectView::of(&compiled);
    let declared = plain(trace(&view, Target::Every, Follow::Declared).unwrap());
    assert_eq!(
        serde_json::to_string(&declared).unwrap(),
        serde_json::to_string(&plain(trace(&view, Target::Every, Follow::Reach).unwrap())).unwrap()
    );
    assert!(GSD_JUDGED_SPEC.contains("requirement AUTH-07"));
}

#[specforge_test(behavior = "follow_trace_route")]
fn reach_traces_a_routed_kind_generically() {
    let (_project, compiled) = compiled(GSD_ROUTED_SPEC);
    let view = ProjectView::of(&compiled);
    let reach = plain(trace(&view, Target::Entity("AUTH-07"), Follow::Reach).unwrap());
    assert!(matches!(reach.traces(), [Trace::Chain(_)]));
    assert_eq!(
        serde_json::to_string(&reach).unwrap(),
        serde_json::to_string(&plain(chain(&view, "AUTH-07").unwrap())).unwrap()
    );
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a named route, the reach, and every entity are traced as asked, and a wrong route is refused"
)]
fn a_named_route_of_another_kind_or_none_is_refused() {
    let (_project, compiled) = compiled(GSD_ROUTED_SPEC);
    let view = ProjectView::of(&compiled);
    let named = traced(
        &compiled,
        Target::Entity("AUTH-07"),
        Follow::Route("gsd/requirement-thread"),
    );
    assert!(matches!(named.0.traces(), [Trace::Route(_)]));

    let unknown = trace(&view, Target::Entity("AUTH-07"), Follow::Route("gsd/nope")).unwrap_err();
    assert_eq!(unknown.error.code, UNKNOWN_ROUTE);
    assert_eq!(
        unknown.error.message,
        "Unknown route 'gsd/nope' (available: gsd/requirement-thread)"
    );
    assert_eq!(unknown.subject, Some(TraceField::Route));

    let another = trace(
        &view,
        Target::Entity("v1.0/03/03-02"),
        Follow::Route("gsd/requirement-thread"),
    )
    .unwrap_err();
    assert_eq!(another.error.code, ROUTE_OF_ANOTHER_KIND);
    assert_eq!(
        another.error.message,
        "route 'gsd/requirement-thread' starts at kind 'requirement', and 'v1.0/03/03-02' is a 'plan'"
    );
    assert_eq!(another.subject, Some(TraceField::Route));

    let missing = trace(&view, Target::Entity("AUTH-7"), Follow::Declared).unwrap_err();
    assert_eq!(missing.error.code, "E003");
    assert_eq!(missing.subject, Some(TraceField::EntityId));
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a named route, the reach, and every entity are traced as asked, and a wrong route is refused"
)]
fn every_entity_is_traced_as_its_own_trace_and_a_route_traces_its_start_kind() {
    let spec = format!("{GSD_ROUTED_SPEC}\nrequirement AUTH-08 \"Logout\" {{}}\n");
    let (_project, compiled) = compiled(&spec);
    let ids = |outcome: &TraceOutcome| -> Vec<(String, bool)> {
        outcome
            .traces()
            .iter()
            .map(|trace| match trace {
                Trace::Chain(chain) => (chain.entity_id.clone(), false),
                Trace::Route(route) => (route.entity_id.clone(), true),
            })
            .collect()
    };
    let every = traced(&compiled, Target::Every, Follow::Declared).0;
    let routed: Vec<(String, bool)> = ids(&every).into_iter().filter(|(_, r)| *r).collect();
    assert_eq!(
        routed,
        [("AUTH-07".to_string(), true), ("AUTH-08".to_string(), true)]
    );
    let all = ids(&every);
    let mut sorted = all.clone();
    sorted.sort();
    assert_eq!(all, sorted, "in id order");
    assert!(all.len() > 2);

    let named = traced(
        &compiled,
        Target::Every,
        Follow::Route("gsd/requirement-thread"),
    )
    .0;
    assert_eq!(ids(&named), routed);

    let reach = traced(&compiled, Target::Every, Follow::Reach).0;
    assert!(ids(&reach).iter().all(|(_, routed)| !routed));
    assert_eq!(ids(&reach).len(), all.len());
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a step skipped for an unloaded optional peer is an I210 notice"
)]
fn a_skipped_step_is_an_i210_notice() {
    // `@code/core`, the optional peer that declares `test`, is not loaded.
    let spec = GSD_ROUTED_SPEC.replace(&format!("  proof [\"{ROUTED_TEST}\"]\n"), "");
    let (_project, compiled) = compiled_in(
        gsd_routed_project_over(&spec, &[GSD_JUDGED]),
        gsd_routed_runtime(),
    );
    let view = ProjectView::of(&compiled);
    let traced = trace(&view, Target::Entity("AUTH-07"), Follow::Declared).unwrap();
    let skipped: Vec<_> = traced
        .notices()
        .iter()
        .filter(|d| d.code == "I210")
        .collect();
    assert_eq!(skipped.len(), 1, "{:?}", traced.notices());
    assert_eq!(
        skipped[0].message,
        "route 'gsd/requirement-thread' skips step 'test': kind 'test' is not loaded"
    );
    let route = match &traced.into_value_and_notices().0 {
        TraceOutcome::One(document) => match &document.trace {
            Trace::Route(route) => (**route).clone(),
            Trace::Chain(_) => panic!("the route is registered"),
        },
        TraceOutcome::Every(_) => unreachable!(),
    };
    assert_eq!(route.skipped, ["step 'test'"]);
    assert_eq!(
        route.text,
        "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → TR1 missing"
    );
}

#[specforge_test(
    behavior = "render_trace_thread",
    verify = "the summary counts a step's matches"
)]
fn the_summary_counts_branches() {
    assert_eq!(
        thread(GSD_ROUTED_SPEC).summary(),
        "thread: plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
    );
    let spec = format!(
        "{GSD_ROUTED_SPEC}\ncommit 7b3c11aa09fe \"Another\" {{\n  task v1.0/03/03-02/T1\n  linked_by \"Gsd-Task\"\n}}\n"
    );
    assert_eq!(
        thread(&spec).summary(),
        "thread: plan 03-02 → T1 → 2 commits → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
    );
    let no_plan = GSD_ROUTED_SPEC.replace(
        "  requirements [AUTH-07]\n  files_modified",
        "  files_modified",
    );
    assert_eq!(
        thread(&no_plan).summary(),
        "thread: no plan → js:app/test/auth.test.js#keeps a session (passed) → TR1 missing"
    );
}

/// [`GSD_ROUTED_SPEC`] over `@gsd/core` and `@code/core`, the test the truth cites collected when `collected`.
fn with_facts(collected: bool) -> RouteTrace {
    let project = match collected {
        true => gsd_routed_project(GSD_ROUTED_SPEC),
        false => gsd_routed_project_over(GSD_ROUTED_SPEC, &[GSD_JUDGED, CODE_CORE]),
    };
    let (_project, compiled) = compiled_in(project, gsd_routed_runtime());
    let reported: Vec<_> = compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == "E003" || d.code == "E115" || d.code == "E116")
        .map(|d| d.message.clone())
        .collect();
    assert!(reported.is_empty(), "{reported:?}");
    thread_of(&compiled, "AUTH-07")
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a route step reaches collector-written facts through a scheme reference, and an unheld one is no match"
)]
fn a_route_step_follows_a_scheme_reference_into_a_fact_entity() {
    let route = with_facts(true);
    assert_eq!(
        route.text,
        format!(
            "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → {ROUTED_TEST} (passed) → TR1 missing"
        )
    );
    assert_eq!(route.steps[3].matches[0].entity_id, ROUTED_TEST);
}

#[specforge_test(
    behavior = "follow_trace_route",
    verify = "a route step reaches collector-written facts through a scheme reference, and an unheld one is no match"
)]
fn an_unheld_scheme_reference_is_no_match() {
    let route = with_facts(false);
    assert_eq!(
        route.text,
        "AUTH-07 → plan 03-02 → T1 → 5e2a91c0d3b4 (Gsd-Task) → no test → TR1 missing"
    );
    assert!(route.steps[3].matches.is_empty());
    assert_eq!(route.missing, ["test"]);
}
