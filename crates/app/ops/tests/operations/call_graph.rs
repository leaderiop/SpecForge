//! The graph a call receives is the graph export of the checked project it runs over: authored values, computed values
//! and each entity's judgements (ADR 0299, K18). A command, a writing collector, a `reads_graph` prompt and a
//! `reads_graph` hook render it with the same `graph_export` call (`prompt.rs`, `hook/mod.rs`), so these tests read the
//! first two.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_extension_sdk::DeclaredCategory;
use specforge_ops::analysis::collect::{self, Mode, Request};
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GsdWrites, TestProject, gsd_judged_builder, gsd_judged_project,
};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;
use specforge_wasm::testing::InProcessRuntime;

use crate::plans::{compiled, run_command};

const SPEC: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n\nverification V \"V\" {\n  phase 03\n}\n\ndeliverable D \"D\" {\n  checks [ { kind unit  ref \"t\" } ]\n}\n";

fn project() -> TestProject {
    TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("a.spec", SPEC)
}

fn ok(output: Value) -> WasmCallResult {
    WasmCallResult::Ok(output.to_string().into_bytes())
}

/// `@t/core` whose compute pass `verify_status` gives `V` a status and the deliverable's check one.
fn computing() -> GsdWrites {
    GsdWrites::new().answering(
        CORE,
        "__pass_verify_status",
        ok(json!({"diagnostics": [], "summary": {}, "values": [
            {"entity": "v1.0/03/V", "field": "status", "value": "passed"},
            {"entity": "D", "field": "checks[0].status", "value": "pass"},
        ]})),
    )
}

/// The last input recorded for `export`.
fn input_of(runtime: &InProcessRuntime, export: &str) -> Value {
    runtime
        .calls()
        .into_iter()
        .rev()
        .find(|call| call.export == export)
        .unwrap_or_else(|| panic!("{export} was not called"))
        .input
}

fn node(input: &Value, id: &str) -> Value {
    input["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap_or_else(|| panic!("no node {id} in {input}"))
        .clone()
}

#[specforge_test(
    behavior = "compute_field_values",
    verify = "the graph a command and a writing collector receive carries the computed values"
)]
#[ignore = "pins K18; T3 un-ignores"]
fn a_commands_graph_carries_the_computed_values() {
    let p = project();
    let runtime = Arc::new(
        computing()
            .answering(CORE, "cmd__quiet", ok(json!({"exit_code": 0})))
            .runtime(),
    );
    let compiled = compiled(&p, &runtime);
    run_command(&p, &compiled, &runtime, "core", "quiet");

    let input = input_of(&runtime, "cmd__quiet");
    let v = node(&input, "v1.0/03/V");
    assert_eq!(v["fields"]["status"], "passed", "{v}");
    assert_eq!(v["computed"], json!(["status"]), "{v}");
    let d = node(&input, "D");
    assert_eq!(d["fields"]["checks"][0]["status"], "pass", "{d}");
}

#[specforge_test(
    behavior = "answer_judged_inputs",
    verify = "inspect, query, the graph export and a command's graph list each entity's judgements"
)]
#[ignore = "pins K18; T3 un-ignores"]
fn a_commands_graph_carries_each_entitys_judgements() {
    let project = gsd_judged_project("");
    let runtime = Arc::new(
        InProcessRuntime::new()
            .with(|| {
                let mut c = gsd_judged_builder();
                c.raw_category(
                    DeclaredCategory::Surfaces,
                    json!([{"commands": [{"id": "show", "title": "show", "description": "show",
                        "export": "cmd__show", "args": []}]}]),
                );
                c
            })
            .answer_raw(
                "@gsd/core",
                "__pass_plan_status",
                ok(json!({"diagnostics": [], "summary": {}, "values": []})),
            )
            .answer_raw("@gsd/core", "cmd__show", ok(json!({"exit_code": 0}))),
    );
    let compiled = project.compile_in(Arc::clone(&runtime) as SharedRuntime);
    run_command(&project, &compiled, &runtime, "core", "show");

    let input = input_of(&runtime, "cmd__show");
    let plan = node(&input, "v1.0/03/03-02");
    let listed = plan["judgements"].as_array().expect("judgements listed");
    let w311 = listed
        .iter()
        .find(|j| j["rule"] == "gsd/W311")
        .unwrap_or_else(|| panic!("no W311 in {plan}"));
    assert_eq!(w311["state"], "missing", "{w311}");
    assert_eq!(w311["passed"], false, "{w311}");
}

#[specforge_test(
    behavior = "compute_field_values",
    verify = "the graph a command and a writing collector receive carries the computed values"
)]
#[ignore = "pins K18; T3 un-ignores"]
fn a_writing_collectors_graph_carries_the_computed_values() {
    let p = project().file(".specforge/reports/git.json", "{}");
    let runtime = Arc::new(
        computing()
            .answering(CORE, "collect__git", ok(json!({"entity_results": []})))
            .runtime(),
    );
    let compiled = compiled(&p, &runtime);
    let view = ProjectView::of(&compiled);
    collect::collect(
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

    let input = input_of(&runtime, "collect__git");
    assert_eq!(node(&input, "v1.0/03/V")["fields"]["status"], "passed");
}
