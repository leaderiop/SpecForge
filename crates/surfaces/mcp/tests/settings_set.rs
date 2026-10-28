//! K16 (ADR 0296): the MCP mutation `specforge.settings_set` writes `specforge.json` through the write lock. Pins
//! for the tickets that deliver it; ignored until then.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{SETTINGS_EXTENSION, settings_extension};
use specforge_wasm::testing::InProcessRuntime;

use crate::support::*;

fn served() -> Served {
    let runtime = Arc::new(InProcessRuntime::new().with(settings_extension));
    TestProject::new()
        .config(|c| c["settings"] = json!({"@t/s": {"profile": "compat"}}))
        .enabling(&[SETTINGS_EXTENSION])
        .serve_in(runtime)
}

#[test]
#[ignore = "pins K16; T14 un-ignores"]
fn set_writes_an_admitted_value() {
    let mut server = served();
    let reply = call_tool(
        &mut server,
        "specforge.settings_set",
        json!({"extension": "@t/s", "name": "level", "value": 2}),
    );
    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    let document: Value = serde_json::from_str(&tool_text(&reply)).unwrap();
    assert_eq!(document["changed"], true, "{document}");

    let config = server.project().config_on_disk();
    assert_eq!(config["settings"]["@t/s"]["level"], 2, "{config}");
    // The other member is as it was.
    assert_eq!(config["settings"]["@t/s"]["profile"], "compat");
}

#[test]
#[ignore = "pins K16; T14 un-ignores"]
fn settings_set_is_a_mutation_and_writes_through_the_lock() {
    let mut server = served();
    let tools = call(&mut server, "tools/list", json!({}));
    let tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "specforge.settings_set")
        .unwrap_or_else(|| panic!("no specforge.settings_set in the tool list"));
    assert_eq!(tool["annotations"]["readOnlyHint"], false, "{tool}");

    let reply = call_tool(
        &mut server,
        "specforge.settings_set",
        json!({"extension": "@t/s", "name": "level", "value": 3}),
    );
    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    // The write went through the lock: it is released, and the journal is gone.
    let leftovers: Vec<String> = std::fs::read_dir(server.root().join(".specforge"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| name.contains("lock") || name.contains("journal"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}
