//! Tests for wiring extension surface contributions to MCP tool/resource registries.
//! Verifies that manifest-declared MCP tools and resources appear in the MCP server's
//! discovery responses after initialization with a project containing surface-contributing extensions.
//! Also tests dynamic kind-based tools and resources generated from the graph.

use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use specforge_test::prelude::*;
use std::fs;
use tempfile::TempDir;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    call(
        server,
        "tools/call",
        json!({"name": tool_name, "arguments": args}),
    )
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

fn read_resource(server: &mut McpServer, uri: &str) -> Value {
    call(server, "resources/read", json!({"uri": uri}))
}

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".into(),
        start_line: 1,
        start_col: 0,
        end_line: 5,
        end_col: 0,
    }
}

/// Create a server with a graph containing multiple entity kinds for dynamic tool/resource tests.
fn init_server_with_kinds() -> McpServer {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));

    let state = server.state_mut();
    let mut graph = Graph::new();

    // Add features
    for (id, title) in [("feat_auth", "Authentication"), ("feat_search", "Search")] {
        let mut fields = FieldMap::new();
        fields.push("status".into(), FieldValue::Identifier("planned".into()));
        graph.add_node(Node {
            id: EntityId { raw: id.into() },
            kind: EntityKind {
                raw: "feature".into(),
            },
            title: Some(title.into()),
            fields,
            source_span: span(),
            methods: Vec::new(),
        });
    }

    // Add behaviors
    let mut fields = FieldMap::new();
    fields.push("contract".into(), FieldValue::String("MUST login".into()));
    graph.add_node(Node {
        id: EntityId {
            raw: "login_behavior".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Login".into()),
        fields,
        source_span: span(),
        methods: Vec::new(),
    });

    state.serve_graph(graph, Vec::new());
    server
}

/// Create a server with extension surface contributions injected directly.
/// Tests MCP surface wiring, not the compile-time extension loading pipeline.
fn init_server_with_surfaces() -> (McpServer, TempDir) {
    use specforge_mcp::types::{McpResourceDescriptor, McpToolDescriptor};
    use specforge_registry::{SurfaceRegistryEntry, SurfaceType};

    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("core.spec"),
        r#"behavior greet "Greet" {
    status planned
    contract "greet users"
}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{
    "name": "test-surfaces",
    "version": "0.1.0",
    "extensions": []
}
"#,
    )
    .unwrap();

    let mut server = McpServer::new();
    let project_root = dir.path().to_str().unwrap();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": project_root}),
    );

    // Inject extension surface contributions directly into server state
    let state = server.state_mut();
    state.tool_registry.push(McpToolDescriptor {
        name: "test.list_items".into(),
        description: "List all items via MCP".into(),
        input_schema: json!({"type": "object", "properties": {"kind": {"type": "string"}}}),
        category: Some("core".into()),
        source: Some("@test/ext".into()),
        ..Default::default()
    });
    state.resource_registry.push(McpResourceDescriptor {
        uri: "specforge://test/items".into(),
        name: "test-items".into(),
        description: Some("All items resource".into()),
        mime_type: Some("application/json".into()),
    });
    state.edit_environment(|env| {
        env.registries.surfaces.push(SurfaceRegistryEntry {
            extension_name: "@test/surfaces".into(),
            surface_type: SurfaceType::McpTool,
            contribution_name: "test.list_items".into(),
            export_name: "mcp__list_items".into(),
        });
        env.registries.surfaces.push(SurfaceRegistryEntry {
            extension_name: "@test/surfaces".into(),
            surface_type: SurfaceType::McpResource,
            contribution_name: "test-items".into(),
            export_name: "mcp__test_items".into(),
        });
    });

    (server, dir)
}

#[test]
fn extension_mcp_tools_in_registry() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    // Core tools must still be present
    assert!(
        tool_names.contains(&"specforge.query"),
        "core tool specforge.query must be present"
    );

    // Extension tool must also be present
    assert!(
        tool_names.contains(&"test.list_items"),
        "extension MCP tool 'test.list_items' must appear in registry. Got: {:?}",
        tool_names
    );
}

#[test]
fn extension_mcp_resources_in_registry() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();

    // Core resources must still be present
    assert!(
        uris.contains(&"specforge://graph"),
        "core resource must be present"
    );

    // Extension resource must also be present
    assert!(
        uris.contains(&"specforge://test/items"),
        "extension MCP resource 'specforge://test/items' must appear in registry. Got: {:?}",
        uris
    );
}

#[test]
fn capabilities_include_extension_counts() {
    use specforge_mcp::types::McpToolDescriptor;

    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));

    // Inject 2 extension tools directly
    let state = server.state_mut();
    state.tool_registry.push(McpToolDescriptor {
        name: "ext.tool_a".into(),
        description: "Tool A".into(),
        input_schema: json!({"type": "object", "properties": {}}),
        category: Some("core".into()),
        source: Some("@test/ext".into()),
        ..Default::default()
    });
    state.tool_registry.push(McpToolDescriptor {
        name: "ext.tool_b".into(),
        description: "Tool B".into(),
        input_schema: json!({"type": "object", "properties": {}}),
        category: Some("core".into()),
        source: Some("@test/ext".into()),
        ..Default::default()
    });

    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();

    // Should have core tools + 2 extension tools
    let ext_tools: Vec<_> = tools
        .iter()
        .filter(|t| {
            let name = t["name"].as_str().unwrap();
            name.starts_with("ext.")
        })
        .collect();
    assert_eq!(
        ext_tools.len(),
        2,
        "expected 2 extension tools, got: {:?}",
        ext_tools
    );
}

// --- Dynamic kind-based tools and resources ---

// B:provide_mcp_entities_by_kind — verify unit "specforge.list returns entities filtered by kind"
#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "specforge.list returns entities filtered by kind"
)]
fn list_tool_returns_entities_by_kind() {
    let mut server = init_server_with_kinds();
    let resp = call_tool(&mut server, "specforge.list", json!({"kind": "feature"}));
    assert!(
        resp["error"].is_null(),
        "specforge.list tool must not return error: {:?}",
        resp
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entities = parsed.as_array().unwrap();

    assert_eq!(
        entities.len(),
        2,
        "expected 2 features, got: {:?}",
        entities
    );
    let ids: Vec<&str> = entities.iter().map(|e| e["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"feat_auth"));
    assert!(ids.contains(&"feat_search"));
}

/// The ids `specforge.list` returns for `args`.
fn listed_ids(server: &mut McpServer, args: Value) -> Vec<String> {
    let resp = call_tool(server, "specforge.list", args);
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "specforge.list keeps the entities whose fields hold the where values"
)]
fn list_tool_filters_by_field_values() {
    let mut server = init_server_with_kinds();
    let planned = json!({"kind": "feature", "where": {"status": "planned"}});
    assert_eq!(
        listed_ids(&mut server, planned),
        ["feat_auth", "feat_search"]
    );
    let by_contract = json!({"where": {"contract": "MUST login"}});
    assert_eq!(listed_ids(&mut server, by_contract), ["login_behavior"]);
    let done = json!({"kind": "feature", "where": {"status": "done"}});
    assert!(listed_ids(&mut server, done).is_empty());
}

#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "specforge.list pages the entities sorted by id with offset and limit"
)]
fn list_tool_pages_sorted_entities() {
    let mut server = init_server_with_kinds();
    assert_eq!(
        listed_ids(&mut server, json!({})),
        ["feat_auth", "feat_search", "login_behavior"]
    );
    let page = json!({"offset": 1, "limit": 1});
    assert_eq!(listed_ids(&mut server, page), ["feat_search"]);
}

#[test]
fn list_tool_refuses_a_malformed_filter_or_page() {
    let mut server = init_server_with_kinds();
    for args in [
        json!({"where": "status=done"}),
        json!({"limit": -1}),
        json!({"offset": "one"}),
        json!({"limit": 1.5}),
    ] {
        let resp = call_tool(&mut server, "specforge.list", args.clone());
        assert_eq!(resp["result"]["isError"], true, "{args}: {resp}");
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "invalid_input", "{args}: {error}");
    }
}

// B:provide_mcp_entities_by_kind — verify unit "specforge.list returns empty array for unknown kind"
#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "specforge.list returns empty for unknown kind"
)]
fn list_tool_empty_for_unknown_kind() {
    let mut server = init_server_with_kinds();
    let resp = call_tool(
        &mut server,
        "specforge.list",
        json!({"kind": "nonexistent"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

// B:provide_mcp_entities_by_kind — verify unit "specforge://entities/{kind} resource returns entities as JSON"
#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "entity-by-kind resource returns entities"
)]
fn entities_by_kind_resource() {
    let mut server = init_server_with_kinds();
    let resp = read_resource(&mut server, "specforge://entities/behavior");
    let contents = &resp["result"]["contents"];
    assert!(
        contents.is_array(),
        "resource must return contents array, got: {:?}",
        resp
    );
    let text = contents[0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    let entities = parsed.as_array().unwrap();
    assert_eq!(entities.len(), 1, "expected 1 behavior");
    assert_eq!(entities[0]["id"].as_str().unwrap(), "login_behavior");
}

// B:provide_mcp_entities_by_kind — verify unit "specforge.list tool is registered"
#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "specforge.list tool appears in tool list"
)]
fn list_tool_registered() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(
        names.contains(&"specforge.list"),
        "specforge.list must be in tool list"
    );
}

// --- Extension tool dispatch ---

#[test]
fn extension_tool_dispatches() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call_tool(&mut server, "test.list_items", json!({"kind": "feature"}));
    // Should NOT be METHOD_NOT_FOUND — extension tools should be recognized
    let error_code = resp["error"]["code"].as_i64();
    assert_ne!(
        error_code,
        Some(-32602),
        "extension tool must not return METHOD_NOT_FOUND, got: {:?}",
        resp
    );
}

// B:handle_mcp_protocol_error — verify unit "unknown tool still returns METHOD_NOT_FOUND"
#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "truly unknown tool returns -32602 Invalid params (MCP spec example)"
)]
fn unknown_tool_returns_invalid_params() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call_tool(&mut server, "totally.unknown.tool", json!({}));
    assert_eq!(resp["error"]["code"].as_i64(), Some(-32602));
}

// B:list_mcp_tools — verify unit "re-compilation preserves core tools"
#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "returns core-provided descriptors when no extensions installed"
)]
fn recompilation_refreshes_surfaces() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("core.spec"),
        r#"behavior greet "Greet" {
    status planned
}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{
    "name": "test-recompile",
    "version": "0.1.0",
    "extensions": []
}
"#,
    )
    .unwrap();

    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": dir.path().to_str().unwrap()}),
    );

    // Exactly the core tools: nothing missing, no extension tool.
    let core_tools = [
        "specforge.add_extension",
        "specforge.analyze",
        "specforge.collect",
        "specforge.coverage",
        "specforge.doctor",
        "specforge.explain",
        "specforge.export",
        "specforge.extensions",
        "specforge.find_definition",
        "specforge.find_implementation",
        "specforge.find_references",
        "specforge.find_spec_for_source",
        "specforge.format",
        "specforge.infer_gaps",
        "specforge.infer_progress",
        "specforge.infer_session",
        "specforge.init",
        "specforge.inspect",
        "specforge.list",
        "specforge.migrate",
        "specforge.model",
        "specforge.outline",
        "specforge.outline_extensions",
        "specforge.providers",
        "specforge.query",
        "specforge.remove_extension",
        "specforge.rename",
        "specforge.render",
        "specforge.schema",
        "specforge.search",
        "specforge.stats",
        "specforge.suggest_fixes",
        "specforge.trace",
        "specforge.validate",
    ];
    let listed = |resp: &Value| -> Vec<String> {
        let tools = resp["result"]["tools"].as_array().unwrap();
        for tool in tools {
            assert_eq!(
                tool["source"], "core",
                "no extension is installed, yet {tool} is listed"
            );
        }
        let mut names: Vec<String> = tools
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        names.sort();
        names
    };
    let resp1 = call(&mut server, "tools/list", json!({}));
    assert_eq!(listed(&resp1), core_tools);

    // Recompile by calling validate
    let validate_resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({
            "path": dir.path().to_str().unwrap()
        }),
    );
    assert_eq!(validate_resp["result"]["isError"], false, "{validate_resp}");

    // The same core set after recompilation.
    let resp2 = call(&mut server, "tools/list", json!({}));
    assert_eq!(listed(&resp2), core_tools);
}

// B:provide_mcp_entities_by_kind — verify unit "entities resource registered in resource list"
#[specforge_test(
    behavior = "provide_mcp_entities_by_kind",
    verify = "entities resource template in resource template list"
)]
fn entities_resource_registered() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "resources/templates/list", json!({}));
    let templates = resp["result"]["resourceTemplates"].as_array().unwrap();
    let uris: Vec<&str> = templates
        .iter()
        .map(|t| t["uriTemplate"].as_str().unwrap())
        .collect();
    assert!(
        uris.contains(&"specforge://entities/{kind}"),
        "specforge://entities/{{kind}} must be in the resource template list. Got: {:?}",
        uris
    );
}

// --- Auto-promoted CLI commands (`@test/cmds`, see fake_extension.rs) ---

use crate::fake_extension::{self, EXT, FakeExtension};

/// The listed descriptor called `name`, or `None`.
fn listed_tool(server: &mut McpServer, name: &str) -> Option<Value> {
    let resp = call(server, "tools/list", json!({}));
    resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .cloned()
}

#[specforge_test(
    behavior = "auto_promote_commands_to_mcp_tools",
    verify = "CLI command auto-promoted to MCP tool"
)]
fn cli_command_auto_promoted_to_mcp_tool() {
    let output = json!({"exit_code": 0, "stdout": "3 of 4 covered", "stderr": ""});
    let (mut server, ext, _dir) =
        fake_extension::initialized(FakeExtension::new().with_output("cmd__report", output));

    let listed = listed_tool(&mut server, "specforge.cmds.report").expect("report promoted");
    assert_eq!(listed["description"], "Write a coverage report");
    // Its role is core; where it comes from is its source.
    assert_eq!(listed["category"], "core");
    assert_eq!(listed["source"], "@test/cmds");

    // A call reaches the command's cmd__ export with the tool arguments as
    // its args, beside the project root and the served graph, and the
    // command's stdout is the tool result.
    let args = json!({"style": "md", "verbose": true});
    let resp = call_tool(&mut server, "specforge.cmds.report", args.clone());
    assert_eq!(
        resp["result"],
        json!({"content": [{"type": "text", "text": "3 of 4 covered"}], "isError": false})
    );
    let calls = ext.calls();
    let [(extension, export, input)] = calls.as_slice() else {
        panic!("one call: {calls:?}")
    };
    assert_eq!((extension.as_str(), export.as_str()), (EXT, "cmd__report"));
    assert_eq!(input["args"], args);
    assert!(input["cwd"].is_string(), "{input}");
    assert!(input["graph"]["nodes"].is_array(), "{input}");

    // A failing command is a failed tool result carrying its stderr.
    let failing = json!({"exit_code": 2, "stdout": "", "stderr": "no tests found"});
    let (mut server, _ext, _dir) =
        fake_extension::initialized(FakeExtension::new().with_output("cmd__report", failing));
    let resp = call_tool(&mut server, "specforge.cmds.report", json!({"style": "md"}));
    assert_eq!(
        resp["result"],
        json!({"content": [
            {"type": "text", "text": ""},
            {"type": "text", "text": "no tests found"}
        ], "isError": true})
    );
}

/// The `tools/call` result of `specforge.cmds.report` when its export
/// returns `output`, and the input the export got.
fn promoted_report(output: Value) -> (Value, Value) {
    let (mut server, ext, _dir) =
        fake_extension::initialized(FakeExtension::new().with_output("cmd__report", output));
    let resp = call_tool(&mut server, "specforge.cmds.report", json!({"style": "md"}));
    let calls = ext.calls();
    let [(_, _, input)] = calls.as_slice() else {
        panic!("one call: {calls:?}")
    };
    (resp["result"].clone(), input.clone())
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP a failure's JSON error object is an isError result carrying it, and output that is not one object is text"
)]
fn over_mcp_a_commands_output_is_structured_only_when_it_is_one_object() {
    let ran = |exit_code: i32, stdout: &str, stderr: &str| {
        promoted_report(json!({"exit_code": exit_code, "stdout": stdout, "stderr": stderr})).0
    };
    let text = |blocks: &[&str]| -> Value {
        blocks
            .iter()
            .map(|t| json!({"type": "text", "text": t}))
            .collect()
    };

    // The command is asked for json, with the host's UTC date.
    let (_, input) = promoted_report(json!({"exit_code": 0, "stdout": "{}", "stderr": ""}));
    assert_eq!(input["format"], "json", "{input}");
    let today = input["today"].as_str().unwrap();
    assert!(
        chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").is_ok(),
        "{input}"
    );
    assert_eq!(input["args"], json!({"style": "md"}), "only declared args");

    // One object on stdout is the structured result, beside its text.
    let result = ran(0, r#"{"covered": 3}"#, "");
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"], json!({"covered": 3}));

    // JSON that is not an object, or an object beside a warning, is text.
    for stdout in ["[1, 2]", "3", "\"done\""] {
        let result = ran(0, stdout, "");
        assert_eq!(
            result,
            json!({"content": text(&[stdout]), "isError": false}),
            "{stdout}"
        );
    }
    let result = ran(0, "{}", "warning: stale");
    assert_eq!(
        result,
        json!({"content": text(&["{}", "warning: stale"]), "isError": false})
    );

    // A failure's one error object on stderr is the isError result.
    let error = json!({"code": "ENTITY_NOT_FOUND", "message": "milestone 'm2' not found"});
    let result = ran(1, "", &error.to_string());
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], error);
    assert_eq!(result["content"], text(&[&error.to_string()]));

    // A failure that wrote prose, or an array, is a failed text result.
    for stderr in ["error: no tests found", "[\"a\"]"] {
        let result = ran(2, "", stderr);
        assert_eq!(
            result,
            json!({"content": text(&["", stderr]), "isError": true}),
            "{stderr}"
        );
    }
    // A failure's object beside stdout is not the error object alone.
    let result = ran(1, "partial", &error.to_string());
    assert_eq!(result["isError"], true, "{result}");
    assert!(result.get("structuredContent").is_none(), "{result}");
}

#[specforge_test(
    behavior = "auto_promote_commands_to_mcp_tools",
    verify = "auto-promoted tool name follows specforge.{ext}.{cmd} pattern"
)]
fn auto_promoted_tool_name_follows_pattern() {
    let (server, _ext, _dir) = fake_extension::initialized(FakeExtension::new());
    // `@test/cmds` has the short name `cmds`; each promoted tool is
    // specforge.cmds.<command id>, dispatched to the command's export.
    let promoted: Vec<(String, String, String)> = server
        .state()
        .surface_entries()
        .filter(|e| e.surface_type == specforge_registry::SurfaceType::AutoPromotedTool)
        .map(|e| {
            (
                e.contribution_name.clone(),
                e.extension_name.clone(),
                e.export_name.clone(),
            )
        })
        .collect();
    assert_eq!(
        promoted,
        [(
            "specforge.cmds.report".to_string(),
            EXT.to_string(),
            "cmd__report".to_string()
        )]
    );
}

#[specforge_test(
    behavior = "auto_promote_commands_to_mcp_tools",
    verify = "a command the CLI refuses, such as one declaring an arg named format, is not promoted"
)]
fn a_command_the_cli_refuses_is_no_tool() {
    let command = |id: &str, arg: &str| {
        json!({"id": id, "title": id, "description": id, "export": format!("cmd__{id}"),
            "args": [{"name": arg, "arg_type": "string"}]})
    };
    let (mut server, _ext, _dir) = fake_extension::initialized(
        FakeExtension::new()
            .with_command(command("render", "format"))
            .with_command(command("open", "path"))
            .with_command(command("draw", "shape")),
    );
    // The rule the CLI refuses a command line by is the one MCP promotes by.
    let refused = |arg: &str| {
        specforge_ops::command::refusal(&specforge_protocol_types::CommandDescriptor {
            id: "x".into(),
            title: "x".into(),
            description: String::new(),
            category: None,
            export: "cmd__x".into(),
            args: vec![specforge_protocol_types::CommandArgDescriptor {
                name: arg.into(),
                arg_type: specforge_protocol_types::CommandArgType::String,
                required: false,
                default_value: None,
                description: None,
            }],
            sandbox: None,
        })
    };
    assert!(refused("format").is_some());
    assert!(refused("shape").is_none());
    assert!(listed_tool(&mut server, "specforge.cmds.render").is_none());
    assert!(listed_tool(&mut server, "specforge.cmds.open").is_none());
    assert!(listed_tool(&mut server, "specforge.cmds.draw").is_some());
}

#[specforge_test(
    behavior = "auto_promote_commands_to_mcp_tools",
    verify = "derived input_schema computed from command args"
)]
fn derived_input_schema_from_command_args() {
    let (mut server, _ext, _dir) = fake_extension::initialized(FakeExtension::new());
    let listed = listed_tool(&mut server, "specforge.cmds.report").unwrap();
    assert_eq!(
        listed["inputSchema"],
        json!({
            "type": "object",
            "properties": {
                "style": {"type": "string", "enum": ["md", "json"], "description": "Output style"},
                "verbose": {"type": "boolean"},
                "limit": {"type": "integer"},
                "out": {"type": "string"}
            },
            "required": ["style"]
        })
    );
}

#[specforge_test(
    behavior = "auto_promote_commands_to_mcp_tools",
    verify = "explicit MCP tool wins over auto-promoted tool with I017"
)]
fn explicit_mcp_tool_wins_over_auto_promoted() {
    let (mut server, ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": true})),
    );
    // One descriptor named specforge.cmds.check: the explicit one.
    let resp = call(&mut server, "tools/list", json!({}));
    let named: Vec<&Value> = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["name"] == "specforge.cmds.check")
        .collect();
    assert_eq!(
        named,
        [&json!({
            "name": "specforge.cmds.check",
            "description": "Explicit check tool",
            "inputSchema": {"type": "object", "properties": {"strict": {"type": "boolean"}}},
            "outputSchema": {"type": "object", "properties": {"checked": {"type": "boolean"}}},
            "category": "core",
            "source": "@test/cmds"
        })]
    );
    let i017: Vec<_> = server
        .state()
        .diagnostics()
        .iter()
        .filter(|d| d.code == "I017")
        .map(|d| (d.severity, d.message.clone()))
        .collect();
    assert_eq!(
        i017,
        [(
            specforge_common::Severity::Info,
            "command 'check' not auto-promoted: explicit MCP tool 'specforge.cmds.check' already exists"
                .to_string()
        )]
    );

    // A call reaches the explicit tool's mcp__ export, not cmd__check.
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({"strict": true}));
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(
        ext.calls(),
        [(
            EXT.to_string(),
            "mcp__check".to_string(),
            json!({"strict": true})
        )]
    );
}

#[specforge_test(
    behavior = "commands_auto_promoted",
    verify = "emits commands_auto_promoted with correct promoted and conflict counts"
)]
fn event_commands_auto_promoted() {
    let (server, _ext, _dir) = fake_extension::initialized(FakeExtension::new());
    let events: Vec<Value> = server
        .state()
        .events
        .iter()
        .filter(|e| e.name == "commands_auto_promoted")
        .map(|e| {
            let mut params = e.params.clone();
            assert!(params["timestamp"].is_string(), "{params}");
            params.as_object_mut().unwrap().remove("timestamp");
            params
        })
        .collect();
    assert_eq!(events, [json!({"promotedCount": 1, "conflictCount": 1})]);

    // A project whose extensions contribute no command promotes nothing
    // and says nothing.
    let (server, _dir) = init_server_with_surfaces();
    assert!(
        !server
            .state()
            .events
            .iter()
            .any(|e| e.name == "commands_auto_promoted")
    );
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "the served project's runtime is the one its compile loaded and serves later calls until the project reloads"
)]
fn one_runtime_serves_extension_calls_until_the_next_compile() {
    let dir = TempDir::new().unwrap();
    let config = json!({"name": "rt", "version": "0.1.0",
        "extensions": ["@specforge/software", "@specforge/testing"]});
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    fs::write(
        dir.path().join("main.spec"),
        "behavior greet \"Greet\" {\n  category command\n  contract \"MUST greet\"\n}\n",
    )
    .unwrap();
    let mut server = McpServer::new();
    call(
        &mut server,
        "initialize",
        json!({"projectRoot": dir.path().to_str().unwrap()}),
    );
    let runtime = |server: &McpServer| {
        std::sync::Arc::clone(
            server
                .state()
                .session()
                .runtime()
                .expect("the served session runs its extensions"),
        )
    };
    let compiled = runtime(&server);

    // analyze runs the extensions' passes in it, call after call, and a
    // validate of the unchanged project keeps it.
    let analyze = json!({"use_cached": true});
    call_tool(&mut server, "specforge.analyze", analyze.clone());
    call_tool(&mut server, "specforge.analyze", analyze);
    call_tool(&mut server, "specforge.validate", json!({}));
    assert!(
        std::sync::Arc::ptr_eq(&compiled, &runtime(&server)),
        "later calls reuse it while the project's environment is unchanged"
    );

    // The environment changes on disk: the reload may load other modules,
    // so later calls run in its runtime.
    let config = json!({"name": "rt", "version": "0.1.0",
        "extensions": ["@specforge/software"]});
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    call_tool(&mut server, "specforge.validate", json!({}));
    assert!(!std::sync::Arc::ptr_eq(&compiled, &runtime(&server)));
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "an extension tool's declared output_schema is listed as its outputSchema"
)]
fn an_extension_tool_lists_its_declared_output_schema() {
    let (mut server, _ext, _dir) = fake_extension::initialized(FakeExtension::new());
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let check = tools
        .iter()
        .find(|t| t["name"] == "specforge.cmds.check")
        .unwrap();
    assert_eq!(
        check["outputSchema"],
        json!({"type": "object", "properties": {"checked": {"type": "boolean"}}})
    );
    // An auto-promoted command declares none.
    let report = tools
        .iter()
        .find(|t| t["name"] == "specforge.cmds.report")
        .unwrap();
    assert!(report.get("outputSchema").is_none(), "{report}");
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "input validated against declared input_schema"
)]
fn extension_tool_input_is_checked_against_its_schema() {
    let (mut server, ext, _dir) = fake_extension::initialized(
        FakeExtension::new()
            .with_output("mcp__check", json!({"checked": true}))
            .with_output("cmd__report", json!("")),
    );
    // strict is a boolean; a string never reaches the module.
    let resp = call_tool(
        &mut server,
        "specforge.cmds.check",
        json!({"strict": "yes"}),
    );
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "invalid_input", "{error}");
    assert!(
        error["message"].as_str().unwrap().contains("$.strict"),
        "{error}"
    );
    // The auto-promoted report requires style, one of md or json.
    for arguments in [json!({}), json!({"style": "xml"})] {
        let resp = call_tool(&mut server, "specforge.cmds.report", arguments.clone());
        let error = crate::tool_errors::mcp_error(&resp);
        assert_eq!(error["code"], "invalid_input", "{arguments}: {error}");
    }
    assert!(ext.calls().is_empty(), "no export ran: {:?}", ext.calls());

    // Valid input reaches the export.
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({"strict": true}));
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(ext.calls().len(), 1);
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "output that does not match the declared output_schema is a schema_mismatch error"
)]
fn extension_tool_output_is_checked_against_its_schema() {
    // check declares {"checked": boolean}; this module returns a string.
    let (mut server, ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": "yes"})),
    );
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({"strict": true}));
    assert_eq!(ext.calls().len(), 1, "the module ran");
    assert_eq!(resp["result"]["isError"], true, "{resp}");
    assert!(resp["result"].get("structuredContent").is_none(), "{resp}");
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "schema_mismatch", "{error}");
    assert_eq!(
        error["data"]["violations"],
        json!(["$.checked: expected boolean, got string"]),
        "{error}"
    );

    // An output that matches is the result.
    let (mut server, _ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": true})),
    );
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({"strict": true}));
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    assert_eq!(
        resp["result"]["structuredContent"],
        json!({"checked": true})
    );
}

#[test]
fn the_schema_check_finds_type_enum_and_required_violations() {
    let schema = json!({
        "type": "object",
        "properties": {
            "format": {"type": "string", "enum": ["md", "json"]},
            "paths": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["format"],
    });
    let check = |value: Value| specforge_mcp::json_schema::violations(&schema, &value);
    assert!(check(json!({"format": "md", "paths": ["a"]})).is_empty());
    assert_eq!(check(json!({})), ["$: missing required format"]);
    assert_eq!(check(json!({"format": "xml"})).len(), 1);
    assert_eq!(
        check(json!({"format": "md", "paths": [1]})),
        ["$.paths[0]: expected string, got integer"]
    );
    assert_eq!(check(json!(3)), ["$: expected object, got integer"]);
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "input JSON passed to mcp__ export"
)]
fn an_extension_tool_export_gets_its_arguments_as_json() {
    let (mut server, ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": true})),
    );
    call_tool(&mut server, "specforge.cmds.check", json!({"strict": true}));
    assert_eq!(
        ext.calls(),
        [(
            EXT.to_string(),
            "mcp__check".to_string(),
            json!({"strict": true})
        )]
    );
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "Wasm trap returned as structured MCP error"
)]
fn a_trapping_extension_tool_is_a_structured_error() {
    // No output for mcp__check: the guest routes no such export.
    let (mut server, ext, _dir) = fake_extension::initialized(FakeExtension::new());
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({}));
    let error = crate::tool_errors::mcp_error(&resp);
    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(error["diagnostic"]["code"], "E028", "{error}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .starts_with("MCP tool mcp__check() of '@test/cmds' trapped: guest_error: unknown export 'mcp__check'"),
        "{error}"
    );
    assert_eq!(error["tool"], "specforge.cmds.check", "{error}");
    assert_eq!(ext.calls().len(), 1, "the export was called");
    // The server keeps serving.
    assert!(call(&mut server, "ping", json!({}))["result"].is_object());
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_tool",
    verify = "tool output returned as MCP tool result"
)]
fn an_extension_tools_output_is_its_result() {
    let (mut server, _ext, _dir) = fake_extension::initialized(
        FakeExtension::new().with_output("mcp__check", json!({"checked": true})),
    );
    let resp = call_tool(&mut server, "specforge.cmds.check", json!({}));
    let result = &resp["result"];
    assert_eq!(result["isError"], false, "{resp}");
    assert_eq!(result["structuredContent"], json!({"checked": true}));
    let text: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    assert_eq!(text, json!({"checked": true}));
}

const SUMMARY: &str = "specforge://ext/cmds/summary";

fn with_summary() -> FakeExtension {
    FakeExtension::new().with_output(
        "mcp__summary",
        json!({"content": "{\"commands\":2}", "mime_type": "application/json"}),
    )
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "URI matched against registered templates"
)]
fn an_extension_resource_is_found_by_its_uri_template() {
    let (mut server, ext, _dir) = fake_extension::initialized(with_summary());
    let resp = read_resource(&mut server, SUMMARY);
    assert!(resp["error"].is_null(), "{resp}");
    // A URI no template of an extension matches reaches no export: the
    // template has no placeholder, so it names itself only.
    for other in [
        "specforge://ext/other/summary",
        "specforge://ext/cmds/summary/more",
    ] {
        let resp = read_resource(&mut server, other);
        assert_eq!(resp["error"]["code"], -32602, "{resp}");
        assert_eq!(
            resp["error"]["message"],
            format!("Unknown resource URI: {other}")
        );
    }
    assert_eq!(ext.calls().len(), 1, "{:?}", ext.calls());
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "URI passed to mcp__ export"
)]
fn an_extension_resource_export_gets_the_uri() {
    let (mut server, ext, _dir) = fake_extension::initialized(with_summary());
    read_resource(&mut server, SUMMARY);
    assert_eq!(
        ext.calls(),
        [(
            EXT.to_string(),
            "mcp__summary".to_string(),
            json!({"uri": SUMMARY})
        )]
    );
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "Wasm trap returned as structured MCP error"
)]
fn a_trapping_extension_resource_is_a_structured_error() {
    // No output for mcp__summary: the guest routes no such export.
    let (mut server, _ext, _dir) = fake_extension::initialized(FakeExtension::new());
    let resp = read_resource(&mut server, SUMMARY);
    let error = resp["error"]
        .as_object()
        .unwrap_or_else(|| panic!("{resp}"));
    let mut keys: Vec<&str> = error.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["code", "message"], "{resp}");
    assert_eq!(error["code"], -32602, "{resp}");
    assert!(
        error["message"].as_str().unwrap().starts_with(
            "E028: MCP resource mcp__summary() of '@test/cmds' trapped: guest_error: unknown export 'mcp__summary'"
        ),
        "{resp}"
    );
    assert!(call(&mut server, "ping", json!({}))["result"].is_object());
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "a resource whose answer is not its content and mime type is a structured MCP error"
)]
fn a_resource_answering_no_content_is_a_structured_error() {
    for answer in [
        json!("oops"),
        json!({"text": "t"}),
        json!({"content": "c"}),
        json!([1, 2]),
    ] {
        let (mut server, _ext, _dir) = fake_extension::initialized(
            FakeExtension::new().with_output("mcp__summary", answer.clone()),
        );
        let resp = read_resource(&mut server, SUMMARY);
        assert!(resp["result"].is_null(), "{answer}: {resp}");
        assert_eq!(resp["error"]["code"], -32602, "{resp}");
        assert!(
            resp["error"]["message"].as_str().unwrap().starts_with(
                "E028: MCP resource mcp__summary() of '@test/cmds' answered output that is not a McpResourceContent: "
            ),
            "{resp}"
        );
    }
}

#[specforge_test(
    behavior = "dispatch_surface_mcp_resource",
    verify = "resource content and mime_type returned to client"
)]
fn an_extension_resources_content_and_mime_type_are_returned() {
    let (mut server, _ext, _dir) = fake_extension::initialized(with_summary());
    let resp = read_resource(&mut server, SUMMARY);
    let content = &resp["result"]["contents"][0];
    assert_eq!(content["uri"], SUMMARY, "{resp}");
    assert_eq!(content["mimeType"], "application/json", "{resp}");
    assert_eq!(content["text"], "{\"commands\":2}", "{resp}");
}
