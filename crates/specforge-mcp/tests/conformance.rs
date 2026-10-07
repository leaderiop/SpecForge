//! C9-11: the declarative surface must agree with the implementation at
//! runtime. Every tool, prompt and resource the server lists is called
//! through the real router; a listed name with no handler fails here
//! instead of reaching an agent as "Unknown tool" or "Unknown operation".

use crate::support::*;
use serde_json::{Value, json};
use specforge_test::prelude::*;

/// A server initialized over a throwaway project holding behavior `alpha`.
/// Tools that write only ever touch this directory.
fn server_over_scratch_project() -> Served {
    TestProject::new()
        .file("test.spec", "behavior alpha \"Alpha\" {\n}\n")
        .serve_components()
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
    let mut server = server_over_scratch_project();
    let tools = crate::support::core_tools();
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
    let mut server = server_over_scratch_project();
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
    let mut server = server_over_scratch_project();
    // Plain resources, then the templated ones.
    let listed = call(&mut server, "resources/list", json!({}));
    let templates = call(&mut server, "resources/templates/list", json!({}));
    let uris: Vec<String> = listed["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["uri"])
        .chain(
            templates["result"]["resourceTemplates"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| &t["uriTemplate"]),
        )
        .map(|uri| uri.as_str().unwrap().to_string())
        .collect();
    assert!(
        uris.len() >= 8,
        "the core resources are listed: {listed} {templates}"
    );

    let unreadable: Vec<String> = uris
        .iter()
        .filter_map(|uri| {
            // A template is read at a value the scratch project holds.
            let uri = uri
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

/// `tools/list` of a server over the fake extension `@test/cmds`, which
/// contributes an explicit tool and two auto-promoted commands.
fn tools_with_an_extension() -> (Served, Vec<Value>) {
    use crate::fake_extension::{EXT, FakeExtension};
    let ext = FakeExtension::new();
    let mut server = TestProject::new()
        .enabling(&[EXT])
        .file("main.spec", "")
        .serve_in(ext.runtime());
    let listed = call(&mut server, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().cloned().unwrap();
    (server, tools)
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "every listed tool has a spec category and a source"
)]
fn every_listed_tool_has_a_spec_category_and_a_source() {
    let (_server, tools) = tools_with_an_extension();
    let core: Vec<&str> = specforge_mcp::tools::CORE_TOOLS
        .iter()
        .map(|t| t.name)
        .collect();
    for tool in &tools {
        let category = tool["category"].as_str().unwrap_or_default();
        assert!(
            ["core", "navigation", "mutation", "management"].contains(&category),
            "a role, never a provenance: {tool}"
        );
        let expected = if core.contains(&tool["name"].as_str().unwrap()) {
            "core"
        } else {
            crate::fake_extension::EXT
        };
        assert_eq!(tool["source"], expected, "{tool}");
    }
    let listed = |name: &str| {
        tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} listed"))
            .clone()
    };
    // infer_session rewrites specforge-infer.json; the other two only read.
    assert_eq!(listed("specforge.infer_session")["category"], "mutation");
    assert_eq!(listed("specforge.infer_progress")["category"], "core");
    assert_eq!(listed("specforge.infer_gaps")["category"], "core");
    assert_eq!(listed("specforge.cmds.check")["source"], "@test/cmds");
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "core tools are annotated: read-only tools readOnlyHint, writing tools how they write"
)]
fn core_tool_annotations_follow_what_each_tool_does() {
    use specforge_mcp::tool::{Access, Category, Handler};
    let (_server, tools) = tools_with_an_extension();
    for spec in specforge_mcp::tools::CORE_TOOLS {
        // One definition: a mutation is exactly a tool with a mutation
        // handler (it says what it wrote), and it writes.
        let mutation = matches!(spec.handler, Handler::Mutation(_));
        assert_eq!(
            mutation,
            spec.category == Category::Mutation,
            "{}",
            spec.name
        );
        if mutation {
            assert_ne!(spec.access, Access::ReadOnly, "{}", spec.name);
        }
        let listed = tools.iter().find(|t| t["name"] == spec.name).unwrap();
        let hints = &listed["annotations"];
        match spec.access {
            Access::ReadOnly => {
                assert_eq!(hints["readOnlyHint"], true, "{listed}");
                assert_eq!(hints["openWorldHint"], false, "{listed}");
            }
            Access::Writes {
                destructive,
                idempotent,
                open_world,
            } => {
                assert_eq!(hints["readOnlyHint"], false, "{listed}");
                assert_eq!(hints["destructiveHint"], destructive, "{listed}");
                assert_eq!(hints["idempotentHint"], idempotent, "{listed}");
                assert_eq!(hints["openWorldHint"], open_world, "{listed}");
            }
        }
    }
    let hint = |name: &str, key: &str| {
        tools.iter().find(|t| t["name"] == name).unwrap()["annotations"][key].clone()
    };
    assert_eq!(hint("specforge.query", "readOnlyHint"), true);
    assert_eq!(hint("specforge.format", "destructiveHint"), true);
    assert_eq!(hint("specforge.add_extension", "openWorldHint"), true);
    assert_eq!(hint("specforge.infer_session", "readOnlyHint"), false);
    // An extension declares no annotations: none are made up for it.
    let extension = tools.iter().find(|t| t["name"] == "specforge.cmds.check");
    assert!(extension.unwrap().get("annotations").is_none());
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "an extension tool is listed once across recompiles"
)]
fn an_extension_tool_is_listed_once_across_recompiles() {
    let (mut server, _) = tools_with_an_extension();
    for _ in 0..2 {
        // validate recompiles the served project.
        call(
            &mut server,
            "tools/call",
            json!({"name": "specforge.validate", "arguments": {}}),
        );
    }
    let listed = call(&mut server, "tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap();
    for name in ["specforge.cmds.check", "specforge.cmds.report"] {
        let count = tools.iter().filter(|t| t["name"] == name).count();
        assert_eq!(count, 1, "{name} listed {count} times");
    }
}
