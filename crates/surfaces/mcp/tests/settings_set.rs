//! K16 (ADR 0296): the MCP mutation `specforge.settings_set` writes `specforge.json` through the write lock.

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{SETTINGS_EXTENSION, settings_extension};
use specforge_wasm::testing::InProcessRuntime;

use specforge_test::prelude::*;

use crate::support::*;
use crate::tool_errors::mcp_error;

fn served() -> Served {
    let runtime = Arc::new(InProcessRuntime::new().with(settings_extension));
    TestProject::new()
        .config(|c| c["settings"] = json!({"@t/s": {"profile": "compat"}}))
        .enabling(&[SETTINGS_EXTENSION])
        .serve_in(runtime)
}

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "specforge.settings_set writes an admitted value through the write lock and the journal"
)]
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

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "specforge.settings_set writes an admitted value through the write lock and the journal"
)]
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
    // The write went through the lock (its file is there) and the journal is gone.
    let leftovers: Vec<String> = std::fs::read_dir(server.root().join(".specforge"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|name| name.contains("journal"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
    assert!(server.root().join(".specforge/write.lock").exists());
}

fn config_text(server: &Served) -> String {
    std::fs::read_to_string(server.root().join("specforge.json")).unwrap()
}

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "specforge.settings_set refuses E086 and E087 naming the argument, before it writes"
)]
fn settings_set_refuses_e086_and_e087_before_writing() {
    let mut server = served();
    let before = config_text(&server);
    let refused = |server: &mut Served, arguments: Value| {
        let reply = call_tool(server, "specforge.settings_set", arguments);
        mcp_error(&reply)
    };

    let secret = refused(
        &mut server,
        json!({"extension": "@t/s", "name": "api_key", "value": "hunter2-not-echoed"}),
    );
    assert_eq!(secret["argument"], "name", "{secret}");
    assert!(!secret.to_string().contains("hunter2"), "{secret}");
    let unknown = refused(
        &mut server,
        json!({"extension": "@t/s", "name": "nope", "value": 1}),
    );
    assert_eq!(unknown["argument"], "name", "{unknown}");
    let wrong = refused(
        &mut server,
        json!({"extension": "@t/s", "name": "level", "value": "two"}),
    );
    assert_eq!(wrong["argument"], "value", "{wrong}");
    let absent = refused(
        &mut server,
        json!({"extension": "@x/y", "name": "level", "value": 1}),
    );
    assert_eq!(absent["argument"], "extension", "{absent}");
    assert_eq!(config_text(&server), before, "nothing was written");
    // A refusal takes no lock and leaves no journal.
    for file in ["write-journal.json", "write.lock"] {
        assert!(
            !server.root().join(".specforge").join(file).exists(),
            "{file} after a refusal"
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "specforge.settings_set takes exactly one of value and unset"
)]
fn settings_set_needs_exactly_one_of_value_and_unset() {
    let mut server = served();
    for arguments in [
        json!({"extension": "@t/s", "name": "level"}),
        json!({"extension": "@t/s", "name": "level", "value": 2, "unset": true}),
    ] {
        let reply = call_tool(&mut server, "specforge.settings_set", arguments);
        let error = mcp_error(&reply);
        assert_eq!(error["argument"], "value", "{error}");
    }
    let reply = call_tool(
        &mut server,
        "specforge.settings_set",
        json!({"extension": "@t/s", "name": "profile", "unset": true}),
    );
    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    let config = server.project().config_on_disk();
    assert!(
        config["settings"]["@t/s"].get("profile").is_none(),
        "{config}"
    );
}

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "dry_run previews the change and lists no file written"
)]
fn settings_set_dry_run_previews_and_lists_no_files() {
    let mut server = served();
    let before = config_text(&server);

    let reply = call_tool(
        &mut server,
        "specforge.settings_set",
        json!({"extension": "@t/s", "name": "level", "value": 2, "dry_run": true}),
    );

    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    let document: Value = serde_json::from_str(&tool_text(&reply)).unwrap();
    assert_eq!(document["changed"], true, "{document}");
    assert_eq!(document["dry_run"], true, "{document}");
    assert_eq!(document["value"], 2, "{document}");
    assert!(document["files_written"].is_null(), "{document}");
    assert_eq!(config_text(&server), before);
}

#[specforge_test(
    behavior = "provide_mcp_settings_set_tool",
    verify = "specforge.settings_set takes a path to another project and does not reload the served one"
)]
fn settings_set_takes_a_path_and_does_not_serve_it() {
    let runtime = Arc::new(InProcessRuntime::new().with(settings_extension));
    let mut server = TestProject::new()
        .config(|c| c["settings"] = json!({"@t/s": {"profile": "compat"}}))
        .enabling(&[SETTINGS_EXTENSION])
        .serve_in(runtime.clone());
    let served_before = config_text(&server);
    // Another project, inside the served root (a path is bounded by it), that enables the same extension.
    let other = server.root().join("nested/other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(
        other.join("specforge.json"),
        r#"{"name": "o", "version": "0.1.0", "extensions": ["@t/s"]}"#,
    )
    .unwrap();
    specforge_installed::testing::install(&other, &*runtime, &[SETTINGS_EXTENSION]);
    std::fs::write(other.join("a.spec"), "").unwrap();
    let generation = server.state().session_generation();

    let reply = call_tool(
        &mut server,
        "specforge.settings_set",
        json!({"path": other.to_str().unwrap(), "extension": "@t/s", "name": "level", "value": 2}),
    );

    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(other.join("specforge.json")).unwrap())
            .unwrap();
    assert_eq!(written["settings"]["@t/s"]["level"], 2, "{written}");
    assert_eq!(
        config_text(&server),
        served_before,
        "the served project is untouched"
    );
    assert_eq!(server.state().session_generation(), generation);
}
