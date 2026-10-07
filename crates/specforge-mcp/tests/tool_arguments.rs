//! How a core tool's arguments are listed and read (plan 08, ADR 0033).
//!
//! The first two tests pin today's behaviour, bugs included: the core
//! tools' listing as a client receives it, and what a call answers (and
//! writes) for each way of sending an argument. They are unlinked
//! characterisation tests; the ticket that changes a line of either
//! snapshot re-blesses it in its own commit, so the diff shows the
//! user-visible change.

use crate::support::*;
use serde_json::{Value, json};
use specforge_test::prelude::*;
use std::path::{Path, PathBuf};

#[test]
fn core_tool_listing_today() {
    let listing = serde_json::to_string_pretty(&core_tools()).expect("a listing");
    insta::assert_snapshot!(listing);
}

/// `main.spec`: `alpha`, misformatted on purpose, and `beta`, which refines it.
const MAIN: &str = concat!(
    "behavior alpha \"Alpha\" {\n",
    "      contract    \"The system MUST work\"\n",
    "}\n",
    "behavior beta \"Beta\" {\n",
    "  refines [alpha]\n",
    "}\n",
);

/// A fresh served project holding [`MAIN`], `@specforge/software` enabled.
fn served() -> Served {
    TestProject::new()
        .enabling(&["@specforge/software"])
        .file("main.spec", MAIN)
        .serve_components()
}

/// The text of a refusal: `refused <code> argument=<a>: <message>`.
fn refusal(error: &Value) -> String {
    format!(
        "refused {} argument={}: {}",
        error["code"].as_str().unwrap_or("?"),
        error["argument"].as_str().unwrap_or("-"),
        error["message"].as_str().unwrap_or("?"),
    )
}

/// The one fact a row records about a successful reply of `tool`.
fn fact(tool: &str, result: &Value, response: &Value) -> String {
    let count = |value: &Value| value.as_array().map_or(0, Vec::len);
    match tool {
        "specforge.format" => format!("check_only={}", result["check_only"]),
        "specforge.rename" => format!("dry_run={}", result["dry_run"]),
        "specforge.migrate" => format!("dry_run={}", result["dry_run"]),
        "specforge.query" | "specforge.export" => format!("nodes={}", count(&result["nodes"])),
        "specforge.search" => format!("results={}", count(result)),
        "specforge.validate" => {
            let verdict = &response["result"]["_meta"]["specforge/check"];
            format!(
                "ok={} errors={} warnings={}",
                verdict["ok"], verdict["errors"], verdict["warnings"]
            )
        }
        _ => String::new(),
    }
}

/// One tool call from a fresh project: `tool | arguments | outcome`.
fn tool_row(tool: &str, label: &str, arguments: impl FnOnce(&Path) -> Value) -> String {
    let mut served = served();
    let root = served.root().to_path_buf();
    let before = files_under(&root);
    let response = call_tool(&mut served, tool, arguments(&root));
    let after = files_under(&root);
    let wrote: Vec<PathBuf> = changed_files(&root, &before, &after);
    let outcome = if response["result"]["isError"] == true {
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default();
        let error: Value = serde_json::from_str(text).unwrap_or_else(|_| json!({"message": text}));
        refusal(&error)
    } else if let Some(error) = response.get("error") {
        format!("refused rpc {error}")
    } else {
        let result = tool_json(&response);
        let fact = fact(tool, &result, &response);
        format!("ok {fact}").trim_end().to_string()
    };
    format!(
        "{} | {label} | {outcome} | wrote {:?}",
        tool.trim_start_matches("specforge."),
        wrote
    )
}

/// One prompt request from a fresh project.
fn prompt_row(prompt: &str, arguments: Value) -> String {
    let mut served = served();
    let label = arguments.to_string();
    let response = get_prompt(
        &mut served,
        &format!("specforge://prompts/{prompt}"),
        arguments,
    );
    let outcome = match response.get("error") {
        Some(error) => format!(
            "refused {} argument={}: {}",
            error["code"],
            error["data"]["argument"].as_str().unwrap_or("-"),
            error["message"].as_str().unwrap_or("?"),
        ),
        None => "ok".to_string(),
    };
    format!("prompt {prompt} | {label} | {outcome}")
}

#[test]
fn argument_reading_today() {
    let fixed = |value: Value| move |_: &Path| value.clone();
    let rows = [
        tool_row(
            "specforge.format",
            r#"{"check":true}"#,
            fixed(json!({"check": true})),
        ),
        tool_row(
            "specforge.format",
            r#"{"check":true,"write":true}"#,
            fixed(json!({"check": true, "write": true})),
        ),
        tool_row(
            "specforge.format",
            r#"{"check":"true"}"#,
            fixed(json!({"check": "true"})),
        ),
        tool_row(
            "specforge.format",
            r#"{"diff":"true"}"#,
            fixed(json!({"diff": "true"})),
        ),
        tool_row(
            "specforge.rename",
            r#"{"entity_id":"alpha","new_name":"gamma","dry_run":"true"}"#,
            fixed(json!({"entity_id": "alpha", "new_name": "gamma", "dry_run": "true"})),
        ),
        tool_row(
            "specforge.migrate",
            r#"{"dry_run":"true"}"#,
            fixed(json!({"dry_run": "true"})),
        ),
        tool_row(
            "specforge.query",
            r#"{"entity_id":"alpha","depth":"0"}"#,
            fixed(json!({"entity_id": "alpha", "depth": "0"})),
        ),
        tool_row(
            "specforge.query",
            r#"{"entity_id":"alpha","kinds":"behavior"}"#,
            fixed(json!({"entity_id": "alpha", "kinds": "behavior"})),
        ),
        tool_row(
            "specforge.search",
            r#"{"query":"a","limit":"1"}"#,
            fixed(json!({"query": "a", "limit": "1"})),
        ),
        tool_row(
            "specforge.list",
            r#"{"limit":"1"}"#,
            fixed(json!({"limit": "1"})),
        ),
        tool_row(
            "specforge.validate",
            r#"{"strict":"yes"}"#,
            fixed(json!({"strict": "yes"})),
        ),
        tool_row(
            "specforge.export",
            r#"{"format":"brief","scop":"alpha"}"#,
            fixed(json!({"format": "brief", "scop": "alpha"})),
        ),
        tool_row(
            "specforge.stats",
            r#"{"use_cached":true}"#,
            fixed(json!({"use_cached": true})),
        ),
        tool_row(
            "specforge.stats",
            r#"{"path":"<served root>"}"#,
            |root| json!({"path": root.to_str().expect("a UTF-8 root")}),
        ),
        tool_row(
            "specforge.doctor",
            r#"{"use_cached":"true"}"#,
            fixed(json!({"use_cached": "true"})),
        ),
        tool_row("specforge.infer_session", "{}", fixed(json!({}))),
        prompt_row("context", json!({"entity_id": 42})),
        prompt_row("review", json!({"entity_id": "alpha", "depth": "two"})),
        prompt_row("context", json!({"entity_id": "alpha", "bogus": "x"})),
    ];
    insta::assert_snapshot!(rows.join("\n"));
}

fn format_changes(arguments: Value) -> Vec<PathBuf> {
    let mut served = served();
    let root = served.root().to_path_buf();
    let before = files_under(&root);
    let reply = call_tool(&mut served, "specforge.format", arguments);
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    changed_files(&root, &before, &files_under(&root))
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "the format tool advertises no default for write, and check or diff without write writes nothing"
)]
fn format_states_the_write_rule_not_a_default() {
    let format = core_tools()
        .into_iter()
        .find(|tool| tool.name == "specforge.format")
        .expect("the format tool");
    let write = &format.input_schema["properties"]["write"];
    assert_eq!(write["type"], "boolean");
    assert!(write.get("default").is_none(), "{write}");

    let none: Vec<PathBuf> = Vec::new();
    assert_eq!(format_changes(json!({"check": true})), none);
    assert_eq!(format_changes(json!({"diff": true})), none);
    assert_eq!(
        format_changes(json!({})),
        [PathBuf::from("main.spec")],
        "an absent write writes"
    );
}
