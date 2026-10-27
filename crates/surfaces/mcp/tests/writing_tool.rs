//! A writing command is an MCP mutation (ADR 0289): its tool is not read-only, offers `dry_run`, and answers the
//! command's output, the plan the host applied and the files it wrote.

use std::sync::Arc;

use crate::support::*;
use serde_json::{Value, json};
use specforge_project::SharedRuntime;
use specforge_project::testing::{
    CORE, GSD_WRITES_EXTENSIONS, GsdWrites, gsd_writes_answer, revision_of,
};
use specforge_test::prelude::*;

const A: &str = "milestone v1.0 \"One\" {}\n\nphase 03 \"Three\" {\n  milestone v1.0\n}\n";
const U: &str = "uat_test U1 \"One\" {\n  phase 03\n}\n";

fn served() -> Served {
    let ops = json!([{"op": "set", "entity": "v1.0/03/U1", "field": "result", "value": "pass",
        "revision": revision_of(U.as_bytes())}]);
    let runtime: SharedRuntime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "cmd__record", gsd_writes_answer(0, ops))
            .runtime(),
    );
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("m.spec", A)
        .file("u.spec", U);
    Served::initialized_in(project, runtime)
}

#[specforge_test(
    behavior = "writing_command_tool",
    verify = "a writing command's tool is a mutation that answers its plan and the files it wrote"
)]
fn a_writing_command_tool_is_a_mutation() {
    let mut server = served();
    let listed = call(&mut server, "tools/list", json!({}));
    let tool = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "specforge.core.record")
        .expect("the command is a tool")
        .clone();
    assert_eq!(tool["annotations"]["readOnlyHint"], false, "{tool}");
    assert!(
        tool["inputSchema"]["properties"]["dry_run"].is_object(),
        "{tool}"
    );

    let reply: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.core.record",
        json!({}),
    )))
    .unwrap();
    assert!(
        reply.get("output").is_some() && reply["plan"].is_object(),
        "{reply}"
    );
    assert_eq!(
        reply["files_written"],
        json!([".gitattributes", "specforge-harness", "u.spec"]),
        "{reply}"
    );
    // The next read sees the write.
    let inspected: Value = serde_json::from_str(&tool_text(&call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "v1.0/03/U1"}),
    )))
    .unwrap();
    assert_eq!(inspected["fields"]["result"], "pass", "{inspected}");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "a writing collector's plan is previewed with dry_run over MCP"
)]
fn a_writing_collectors_plan_is_previewed_with_dry_run() {
    let ops = json!([{"op": "create", "kind": "commit", "id": "c1", "title": "C", "file": "commits.spec",
        "revision": "absent", "fields": {"phase": "03", "sha": "abc"}}]);
    let answer = specforge_wasm::WasmCallResult::Ok(
        json!({"entity_results": [], "plan": {"ops": ops}})
            .to_string()
            .into_bytes(),
    );
    let runtime: SharedRuntime = Arc::new(
        GsdWrites::new()
            .answering(CORE, "collect__git", answer)
            .runtime(),
    );
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("m.spec", A)
        .file("u.spec", U)
        .file(".specforge/reports/git.json", "{}");
    let mut server = Served::initialized_in(project, runtime);
    let collect = |server: &mut Served, dry_run: bool| {
        call_tool(
            server,
            "specforge.collect",
            json!({"runner": "git", "dry_run": dry_run}),
        )
    };
    let reply = collect(&mut server, true);
    assert!(!tool_text(&reply).is_empty(), "{reply}");
    assert!(
        !server.root().join("commits.spec").exists(),
        "dry_run wrote"
    );
    collect(&mut server, false);
    assert!(server.root().join("commits.spec").exists());
}
