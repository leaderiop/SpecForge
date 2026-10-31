//! K17 (ADR 0297, D2): a writing collector's plan is judged against the project with the report its collect wrote,
//! so an error the recorded results alone cause (a check pass that `reads_tests`) does not refuse the plan.
//! `@t/tests` reports E900 for every failed result; `@t/core`'s `git` collector answers a failing result and a plan.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta};
use specforge_ops::Containment;
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
        json!([
            {"code": "E900", "title": "A test failed", "explanation": "A recorded result failed."},
            {"code": "E901", "title": "A commit is recorded", "explanation": "A commit entity exists."},
        ]),
    );
    c.raw_category(
        DeclaredCategory::Passes,
        json!([
            {"name": "failures", "phase": "check", "reads_tests": true, "reports": ["E900"]},
            {"name": "commits", "phase": "check", "reports": ["E901"]},
        ]),
    );
    c
}

/// E900 once a report with a failed result is sent; nothing before.
fn handle(export: &str, input: &[u8]) -> Option<Result<Vec<u8>, String>> {
    let input: Value = serde_json::from_slice(input).unwrap_or(Value::Null);
    if export == "__pass_commits" {
        // E901 once the commit `c9` is in the project; nothing before (the other tests record `c1`).
        let recorded = input["entities"].as_array().into_iter().flatten().any(|e| {
            e["kind"] == "commit" && e["id"].as_str().is_some_and(|id| id.ends_with("c9"))
        });
        let diagnostics = if recorded {
            json!([{"code": "E901", "severity": "Error", "message": "a commit is recorded"}])
        } else {
            json!([])
        };
        let output = json!({"diagnostics": diagnostics, "summary": {}, "values": []});
        return Some(Ok(output.to_string().into_bytes()));
    }
    if export != "__pass_failures" {
        return None;
    }
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
    collect_and_apply_in(p, ops, &Containment::unbounded())
}

/// [`collect_and_apply`] over a view bounded by `containment`, as MCP's is.
fn collect_and_apply_in(
    p: &TestProject,
    ops: Value,
    containment: &Containment,
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
    let view = ProjectView::of(&compiled).bounded_by(containment);
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
    commit_as("c1", phase)
}

fn commit_as(id: &str, phase: &str) -> Value {
    json!([{"op": "create", "kind": "commit", "id": id, "title": "C", "file": "commits.spec",
        "revision": "absent", "fields": {"phase": phase, "sha": "abc"}}])
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collectors plan is judged against the report its collect wrote"
)]
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

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a collector's plan is judged for host errors only"
)]
#[ignore = "pins K20a; T5 un-ignores"]
fn a_collectors_plan_is_not_refused_for_an_extensions_error() {
    // The recorded commit makes the extension's E901 stand: reported after the write, not a refusal of it.
    let p = project();
    let (applied, runtime) = collect_and_apply(&p, commit_as("c9", "03"));
    applied.unwrap_or_else(|refusal| panic!("{refusal}"));
    let written = std::fs::read_to_string(p.root().join("commits.spec")).unwrap();
    assert!(written.contains("commit c9 \"C\""), "{written}");
    let reported = compiled(&p, &runtime).diagnostics();
    assert!(
        reported.iter().any(|d| d.code.ends_with("E901")),
        "{reported:?}"
    );
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collectors plan is judged against the report its collect wrote"
)]
fn a_collector_plan_adding_an_error_itself_is_still_e080() {
    // The commit names a phase that does not exist (E003): refused, and the report's E900 is not named.
    let p = project();
    let (applied, _) = collect_and_apply(&p, commit("99"));
    let refusal = applied.unwrap_err();
    assert!(refusal.starts_with("E080"), "{refusal}");
    assert!(refusal.contains("E003"), "{refusal}");
    assert!(!refusal.contains("E900"), "{refusal}");
    assert!(!p.root().join("commits.spec").exists());
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collectors plan is judged against the report its collect wrote"
)]
fn a_collect_without_plans_compiles_once() {
    let p = project();
    let runtime = Arc::new(
        GsdWrites::new()
            .runtime()
            .with_handler(tests_extension, handle),
    );
    let compiled = compiled(&p, &runtime);
    let view = ProjectView::of(&compiled);
    let passes = || {
        runtime
            .calls()
            .iter()
            .filter(|c| c.export == "__pass_failures")
            .count()
    };
    let before = passes();
    assert!(
        apply_collected(&view, Vec::new(), false, caller())
            .unwrap()
            .is_empty()
    );
    assert_eq!(passes(), before);
}

#[cfg(unix)]
#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collectors plan is applied within its calls containment"
)]
fn a_collectors_plan_reads_the_format_configuration_within_the_calls_containment() {
    let p = project();
    let elsewhere = tempfile::TempDir::new().unwrap();
    let outside = elsewhere.path().join("c.toml");
    std::fs::write(&outside, "indent_width = 8\n").unwrap();
    std::os::unix::fs::symlink(&outside, p.root().join(".specforgefmt.toml")).unwrap();
    let (applied, _) = collect_and_apply_in(&p, commit("03"), &Containment::within(p.root()));
    applied.unwrap_or_else(|refusal| panic!("{refusal}"));
    // The bounded read refuses the link: the defaults lay the commit out, not the linked file's eight columns.
    let written = std::fs::read_to_string(p.root().join("commits.spec")).unwrap();
    assert!(written.contains("\n  phase v1.0/03"), "{written}");
}
