//! C9-11: the declarative surface must agree with the implementation at
//! runtime. Every tool, prompt and resource the server lists is called
//! through the real router; a listed name with no handler fails here
//! instead of reaching an agent as "Unknown tool" or "Unknown operation".

use serde_json::{Value, json};
use specforge_mcp::McpServer;
use specforge_test::prelude::*;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

/// A server initialized over a throwaway project holding behavior `alpha`.
/// Tools that write only ever touch this directory.
fn server_over_scratch_project() -> (McpServer, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        json!({"name": "t", "version": "0.1.0", "extensions": []}).to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\n",
    )
    .unwrap();
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": dir.path().to_string_lossy()}),
    );
    (server, dir)
}

/// Why a response says the name it was sent to is unknown, if it does:
/// the router's `-32601` (an operation with no arm) or an "Unknown ..."
/// message (a name with no handler at all).
fn unknown_name_error(resp: &Value) -> Option<String> {
    let error = resp.get("error")?;
    let message = error["message"].as_str().unwrap_or_default();
    (error["code"] == -32601 || message.starts_with("Unknown ")).then(|| error.to_string())
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "every listed core tool dispatches to its handler"
)]
fn every_listed_core_tool_dispatches_to_its_handler() {
    let (mut server, _dir) = server_over_scratch_project();
    let tools = specforge_mcp::registry::default_tools();
    assert!(tools.len() >= 15, "the core tool surface is listed");

    let listed = call(&mut server, "tools/list", json!({}));
    let unrouted: Vec<String> = tools
        .iter()
        .filter_map(|tool| {
            assert!(
                listed["result"]["tools"]
                    .as_array()
                    .is_some_and(|l| l.iter().any(|t| t["name"] == tool.name.as_str())),
                "{} is a core tool but tools/list does not list it",
                tool.name
            );
            let resp = call(
                &mut server,
                "tools/call",
                json!({"name": tool.name, "arguments": {}}),
            );
            unknown_name_error(&resp).map(|e| format!("{}: {e}", tool.name))
        })
        .collect();
    assert!(
        unrouted.is_empty(),
        "listed tools the router does not know: {unrouted:#?}"
    );
}

#[specforge_test(
    behavior = "list_mcp_prompts",
    verify = "every listed core prompt resolves to a handler"
)]
fn every_listed_core_prompt_resolves_to_a_handler() {
    let (mut server, _dir) = server_over_scratch_project();
    let listed = call(&mut server, "prompts/list", json!({}));
    let prompts = listed["result"]["prompts"].as_array().cloned().unwrap();
    assert!(prompts.len() >= 5, "the core prompts are listed: {listed}");

    let unrouted: Vec<String> = prompts
        .iter()
        .filter_map(|prompt| {
            let name = prompt["name"].as_str().unwrap();
            let resp = call(
                &mut server,
                "prompts/get",
                json!({"name": name, "arguments": {"entity_id": "alpha"}}),
            );
            unknown_name_error(&resp).map(|e| format!("{name}: {e}"))
        })
        .collect();
    assert!(
        unrouted.is_empty(),
        "listed prompts with no handler: {unrouted:#?}"
    );
}

#[specforge_test(
    behavior = "list_mcp_resources",
    verify = "every listed core resource is readable"
)]
fn every_listed_core_resource_is_readable() {
    let (mut server, _dir) = server_over_scratch_project();
    let listed = call(&mut server, "resources/list", json!({}));
    let resources = listed["result"]["resources"].as_array().cloned().unwrap();
    assert!(
        resources.len() >= 8,
        "the core resources are listed: {listed}"
    );

    let unreadable: Vec<String> = resources
        .iter()
        .filter_map(|resource| {
            // A template is read at a value the scratch project holds.
            let uri = resource["uri"]
                .as_str()
                .unwrap()
                .replace("{entity_id}", "alpha")
                .replace("{kind}", "behavior");
            let resp = call(&mut server, "resources/read", json!({"uri": uri}));
            let contents = &resp["result"]["contents"];
            let read = contents.as_array().is_some_and(|c| !c.is_empty())
                && contents[0]["uri"].as_str().is_some()
                && contents[0]["text"].as_str().is_some();
            (!read).then(|| format!("{uri}: {resp}"))
        })
        .collect();
    assert!(
        unreadable.is_empty(),
        "listed resources that do not read: {unreadable:#?}"
    );
}
