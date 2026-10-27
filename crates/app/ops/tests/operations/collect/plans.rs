//! A writing collector's edit plan (ADR 0289), applied after its report is read. The fixture is `@t/core`'s `git`
//! and `@t/ui`'s `dom` collectors ([`specforge_project::testing::GsdWrites`]), each answering the plan the test spells.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_ops::analysis::collect::{self, Mode, Request};
use specforge_ops::authoring::apply::apply_collected;
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdWrites};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;

use crate::plans::{compiled, project, revision};

/// A `collect__<name>` answer: no results, and `ops` as the plan.
fn plan(ops: Value) -> WasmCallResult {
    WasmCallResult::Ok(
        json!({"entity_results": [], "plan": {"ops": ops}})
            .to_string()
            .into_bytes(),
    )
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collector receives the graph and its plan is applied after the report"
)]
fn a_collector_plan_creates_commits_and_records_results() {
    let p = project().file(".specforge/reports/git.json", "{}");
    let ops = json!([{"op": "create", "kind": "commit", "id": "c1", "title": "C", "file": "commits.spec",
        "revision": "absent", "fields": {"phase": "03", "sha": "abc"}}]);
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "collect__git", plan(ops))
            .runtime(),
    );
    let compiled = compiled(&p, &runtime);
    let view = ProjectView::of(&compiled);
    let mut outcome = collect::collect(
        &view,
        Request {
            runner: Some("git"),
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        &ScriptedMachine::new(),
    )
    .unwrap();
    let plans = std::mem::take(&mut outcome.plans);
    let outcome = outcome.with_applied(apply_collected(&view, plans, false, caller()).unwrap());

    let written = std::fs::read_to_string(p.root().join("commits.spec")).unwrap();
    assert!(written.contains("commit c1 \"C\""), "{written}");
    assert!(outcome.runners[0].plan.is_some());
    // The collector was handed the graph and the revisions of the sources it may name.
    let sent = runtime
        .calls()
        .into_iter()
        .find(|c| c.export == "collect__git")
        .unwrap();
    let input = &sent.input;
    assert!(input["graph"].is_object(), "{input}");
    assert_eq!(
        input["revisions"]["03-01.spec"],
        revision(&p, "03-01.spec"),
        "{input}"
    );
}
