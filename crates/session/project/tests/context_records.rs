//! The records a context route keeps of its pack (ADR 0356 D12): the GSD-shaped executor route records the plan's
//! `context_pack` and raises `gsd/W910` when the pack it recorded has moved, when it reaches no subject or several, or
//! when its core outgrew the recorded budget; the plan's owner gate decides when it is checked, and a check evaluates
//! one pack per record and none without one.

use std::sync::Arc;

use serde_json::json;
use specforge_common::{Diagnostic, DiagnosticData, Severity};
use specforge_emitter::{EmitFormat, generate_schema};
use specforge_project::context::{pack, testing::packs_evaluated};
use specforge_project::testing::{
    CODE_CORE, CONTEXT_PACK, CONTEXT_PLAN, CONTEXT_STALE, GSD_CONTEXT_SPEC, GSD_JUDGED,
    TestProject, code_core_builder, gsd_context_builder, gsd_context_project,
    gsd_routed_project_over, recording,
};
use specforge_project::{CompiledProject, SharedRuntime};
use specforge_registry::PlainDisk;
use specforge_test::prelude::*;
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

/// The stale code as the project prints it.
const STALE: &str = "gsd/W910";

/// `@gsd/core` with its context routes and `requirement-pack`, which reads a requirement and records it in a plan's
/// `context_pack` through the plan's requirements; the plan-status pass gives each of `complete` the status `complete`.
fn runtime(complete: &[&str]) -> SharedRuntime {
    let values: Vec<_> = complete
        .iter()
        .map(|plan| json!({"entity": plan, "field": "status", "value": "complete"}))
        .collect();
    let runtime = InProcessRuntime::new()
        .with(|| {
            let mut c = gsd_context_builder();
            c.context("requirement-pack", |r| {
                r.subject("requirement")
                    .budget(2000)
                    .core("requirement", "self")
                    .record("plan.context_pack", "self.requirements")
                    .stale(CONTEXT_STALE);
            });
            c
        })
        .with(code_core_builder)
        .answer_raw(
            GSD_JUDGED,
            "__pass_plan_status",
            WasmCallResult::Ok(
                json!({"diagnostics": [], "summary": {}, "values": values})
                    .to_string()
                    .into_bytes(),
            ),
        );
    Arc::new(runtime)
}

/// The digest the executor's pack of the plan has at `budget` in `project`.
fn digest_at(project: &TestProject, budget: usize) -> String {
    let compiled = project.compile_in(runtime(&[]));
    let route = compiled
        .environment()
        .registries()
        .contexts()
        .named("gsd/executor")
        .expect("the executor route");
    let schema = generate_schema(compiled.environment().registries());
    pack(
        &compiled,
        &PlainDisk,
        route,
        CONTEXT_PLAN,
        budget,
        EmitFormat::Context,
        &schema,
    )
    .expect("the pack fits")
    .digest
}

/// The executor's record of `digest` at `budget`, as written.
fn record(route: &str, digest: &str, budget: u32) -> String {
    format!("{{ route \"{route}\" digest \"{digest}\" max_tokens {budget} }}")
}

fn stale(compiled: &CompiledProject) -> Vec<Diagnostic> {
    compiled
        .diagnostics()
        .iter()
        .filter(|d| d.code == STALE)
        .cloned()
        .collect()
}

/// The task `T1` of the plan renamed: a core member of the executor's pack moves.
fn moved(project: &TestProject) {
    let text = project.read("a.spec");
    let renamed = text.replace("task T1 \"The store\"", "task T1 \"The store, atomic\"");
    assert_ne!(text, renamed);
    project.write("a.spec", renamed);
}

#[specforge_test(
    behavior = "report_stale_context_record",
    verify = "a record whose pack has moved raises its route's stale code at the record"
)]
fn an_unchanged_record_is_silent_and_a_moved_pack_raises_the_stale_code() {
    let digest = digest_at(&gsd_context_project(), 6000);
    let project = recording(
        gsd_context_project(),
        &record("gsd/executor", &digest, 6000),
        runtime(&[]),
    );
    let fresh = project.compile_in(runtime(&[]));
    assert!(stale(&fresh).is_empty(), "{:?}", stale(&fresh));
    assert!(
        !fresh.diagnostics().iter().any(|d| d.code == "E083"),
        "{:?}",
        fresh.diagnostics()
    );

    moved(&project);
    let compiled = project.compile_in(runtime(&[]));
    let found = stale(&compiled);
    assert_eq!(found.len(), 1, "{found:?}");
    let finding = &found[0];
    assert_eq!(finding.severity, Severity::Warning);
    assert_eq!(finding.origin(), Some(GSD_JUDGED));
    assert!(finding.message.contains("has moved"), "{}", finding.message);
    assert!(finding.message.contains(&digest), "{}", finding.message);
    let line = project
        .read("a.spec")
        .lines()
        .position(|line| line.contains(CONTEXT_PACK))
        .expect("the record's line")
        + 1;
    let span = finding.span.as_ref().expect("a span");
    assert!(span.file.as_str().ends_with("a.spec"), "{span:?}");
    assert_eq!(span.start_line, line, "{span:?}");
    assert_eq!(
        finding.data.as_deref(),
        Some(&DiagnosticData::Subject {
            entity: CONTEXT_PLAN.to_string()
        })
    );
    assert_eq!(
        finding.suggestion.as_deref(),
        Some(
            format!(
                "run `specforge context gsd/executor {CONTEXT_PLAN} --record {CONTEXT_PLAN}` again"
            )
            .as_str()
        )
    );
}

#[specforge_test(
    behavior = "report_stale_context_record",
    verify = "a record is checked only when its field's owner gate admits it"
)]
fn a_pending_gate_checks_a_record_only_while_its_owner_is_open() {
    let digest = digest_at(&gsd_context_project(), 6000);
    let project = recording(
        gsd_context_project(),
        &record("gsd/executor", &digest, 6000),
        runtime(&[]),
    );
    moved(&project);
    // Open (no status): checked. Complete: the gate closes, and the moved record says nothing.
    assert_eq!(stale(&project.compile_in(runtime(&[]))).len(), 1);
    assert!(stale(&project.compile_in(runtime(&[CONTEXT_PLAN]))).is_empty());
}

#[specforge_test(
    behavior = "report_stale_context_record",
    verify = "a record with no subject, or whose core no longer fits its budget, is stale"
)]
fn a_record_whose_subject_is_gone_or_whose_core_outgrew_its_budget_is_stale() {
    // Over its budget: the core of the executor's pack is far above ten tokens.
    let project = recording(
        gsd_context_project(),
        &record("gsd/executor", "sha256:0", 10),
        runtime(&[]),
    );
    let found = stale(&project.compile_in(runtime(&[])));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("no longer holds its core"),
        "{}",
        found[0].message
    );

    // The requirement pack's subject: the plan names two requirements, then none.
    let two = recording(
        gsd_context_project(),
        &record("gsd/requirement-pack", "sha256:0", 2000),
        runtime(&[]),
    );
    let found = stale(&two.compile_in(runtime(&[])));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("several"), "{}", found[0].message);
    let none = gsd_routed_project_over(
        &GSD_CONTEXT_SPEC.replace(
            "  requirements [STORE-01, STORE-02]\n  files_modified",
            "  files_modified",
        ),
        &[GSD_JUDGED, CODE_CORE],
    );
    let none = recording(
        none,
        &record("gsd/requirement-pack", "sha256:0", 2000),
        runtime(&[]),
    );
    let found = stale(&none.compile_in(runtime(&[])));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("reaches no 'requirement'"),
        "{}",
        found[0].message
    );
}

#[specforge_test(
    behavior = "report_stale_context_record",
    verify = "a check evaluates one pack per held record and none for a project without one"
)]
fn a_check_evaluates_one_pack_per_record_and_none_without_records() {
    let bare = gsd_context_project();
    let digest = digest_at(&bare, 6000);
    let before = packs_evaluated();
    let _ = bare.compile_in(runtime(&[]));
    assert_eq!(packs_evaluated() - before, 0, "no record, no pack");

    let recorded = recording(
        gsd_context_project(),
        &record("gsd/executor", &digest, 6000),
        runtime(&[]),
    );
    let before = packs_evaluated();
    let compiled = recorded.compile_in(runtime(&[]));
    assert_eq!(packs_evaluated() - before, 1, "one record, one pack");
    assert!(stale(&compiled).is_empty());
}
