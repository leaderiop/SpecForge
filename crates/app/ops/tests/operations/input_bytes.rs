//! Pins the collector and hook inputs as 1.10 sent them: a collector is told `today` only when it writes, a hook is
//! told `today`, and neither is told `now` (K14). The ticket that adds `now` re-blesses these with that one key.

use std::sync::Arc;

use serde_json::json;
use specforge_ops::analysis::collect::{self, Mode, Request};
use specforge_ops::testing::{ScriptedMachine, caller};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{CORE, GsdHosts, GsdWrites};
use specforge_test_macros::test as specforge_test;
use specforge_wasm::WasmCallResult;

use crate::host_support::{hosted, hosts_project, run_hook, write_event};
use crate::plans::{compiled, project};

fn keys(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[specforge_test(
    behavior = "answer_edit_plan",
    verify = "a writing collector receives the graph and its plan is applied after the report"
)]
fn command_collect_and_hook_inputs_are_byte_equal_without_k14() {
    // A writing collector: the graph, the revisions it may name, and the date.
    let p = project().file(".specforge/reports/git.json", "{}");
    let runtime = Arc::new(
        GsdWrites::new()
            .answering(
                CORE,
                "collect__git",
                WasmCallResult::Ok(json!({"entity_results": []}).to_string().into_bytes()),
            )
            .runtime(),
    );
    let compiled = compiled(&p, &runtime);
    collect::collect(
        &ProjectView::of(&compiled),
        Request {
            runner: Some("git"),
            mode: Mode::NoRun,
            announce: &mut |_, _| {},
            caller: caller(),
        },
        &ScriptedMachine::new(),
    )
    .unwrap();
    let sent = runtime
        .calls()
        .into_iter()
        .find(|c| c.export == "collect__git")
        .unwrap()
        .input;
    assert_eq!(
        keys(&sent),
        ["cwd", "graph", "reports", "revisions", "today"],
        "{sent}"
    );

    // A hook: the event, the host, the date and the facts of its paths.
    let hosted = hosted(hosts_project(json!({}), json!({})), GsdHosts::new());
    let root = hosted.project.root().display().to_string();
    run_hook(
        &hosted,
        "write_guard",
        &write_event("claude-code", &format!("{root}/m.spec"), "x"),
    );
    let sent = hosted.input("hook__write_guard");
    let keys = keys(&sent);
    assert!(keys.contains(&"today") && !keys.contains(&"now"), "{sent}");
}
