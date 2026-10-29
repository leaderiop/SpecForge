//! K17 (ADR 0297, D2): a writing collector's plan is judged against the project with the report its collect wrote,
//! so an error the recorded results alone cause (a check pass that `reads_tests`) does not refuse the plan.
//! `@t/tests` reports E900 for every failed result; `@t/core`'s `git` collector answers a failing result and a plan.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_ops::analysis::collect::{self, Mode, Request};
use specforge_ops::authoring::apply::apply_collected;
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GSD_WRITES_EXTENSIONS, GsdWrites, TestProject};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;

use crate::plans::{MILESTONE, ONE, TWO, compiled};

const TESTS: &str = "@t/tests";

fn tests_extension() -> ContributionsBuilder {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(TESTS, "1.0.0"));
    c.raw_category(
        DeclaredCategory::Codes,
        json!([{"code": "E900", "title": "A test failed", "explanation": "A recorded result failed."}]),
    );
    c.raw_category(
        DeclaredCategory::Passes,
        json!([{"name": "failures", "phase": "check", "reads_tests": true, "reports": ["E900"]}]),
    );
    c
}

/// E900 once a report with a failed result is sent; nothing before.
fn handle(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    if export != "__pass_failures" {
        return None;
    }
    let input: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
    let failed = input["test_results"]["results"]
        .as_object()
        .into_iter()
        .flatten()
        .any(|(_, tests)| {
            tests["tests"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|t| t["status"] == "failed")
        });
    let diagnostics = if failed {
        json!([{"code": "E900", "severity": "Error", "message": "a recorded test failed"}])
    } else {
        json!([])
    };
    let output = json!({"diagnostics": diagnostics, "summary": {}, "values": []});
    Some(Ok(output.to_string().into_bytes()))
}

fn project() -> TestProject {
    let mut enabled = GSD_WRITES_EXTENSIONS.to_vec();
    enabled.push(TESTS);
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&enabled)
        .installing_enabled()
        .file("m.spec", MILESTONE)
        .file("03-01.spec", ONE)
        .file("03-02.spec", TWO)
        .file(".specforge/reports/git.json", "{}")
}

/// `git` answers a failing result for `U1` and a plan: ops as given.
fn answer(ops: Value) -> WasmCallResult {
    WasmCallResult::Ok(
        json!({"entity_results": [{"entity_id": "v1.0/03/U1",
            "test_results": [{"name": "t", "status": "failed"}]}], "plan": {"ops": ops}})
        .to_string()
        .into_bytes(),
    )
}

/// Collect, then apply the plans over the view compiled before the report; the refusal's code if any.
fn collect_and_apply(
    p: &TestProject,
    ops: Value,
) -> (
    Result<(), String>,
    Arc<specforge_wasm::testing::InProcessRuntime>,
) {
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "collect__git", answer(ops))
            .runtime()
            .with_handler(tests_extension, handle),
    );
    let compiled = compiled(p, &runtime);
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
    let applied = apply_collected(&view, plans, false, caller())
        .map(|_| ())
        .map_err(|e| format!("{}: {}", e.code, e.message));
    (applied, runtime)
}

fn commit(phase: &str) -> Value {
    json!([{"op": "create", "kind": "commit", "id": "c1", "title": "C", "file": "commits.spec",
        "revision": "absent", "fields": {"phase": phase, "sha": "abc"}}])
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collectors plan is judged against the report its collect wrote"
)]
#[ignore = "pins K17; T4 un-ignores"]
fn a_collectors_plan_is_judged_against_the_report_it_wrote() {
    let p = project();
    let (applied, runtime) = collect_and_apply(&p, commit("03"));
    applied.unwrap_or_else(|refusal| panic!("{refusal}"));
    let written = std::fs::read_to_string(p.root().join("commits.spec")).unwrap();
    assert!(written.contains("commit c1 \"C\""), "{written}");
    // The pass's error is reported by the project the report is in, not a refusal of the plan.
    let reported = compiled(&p, &runtime).diagnostics();
    assert!(
        reported.iter().any(|d| d.code.ends_with("E900")),
        "{reported:?}"
    );
}
