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

    state.graph = graph;
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
    state.surface_entries.push(SurfaceRegistryEntry {
        extension_name: "@test/surfaces".into(),
        surface_type: SurfaceType::McpTool,
        contribution_name: "test.list_items".into(),
        export_name: "mcp__list_items".into(),
        enabled: true,
    });
    state.surface_entries.push(SurfaceRegistryEntry {
        extension_name: "@test/surfaces".into(),
        surface_type: SurfaceType::McpResource,
        contribution_name: "test-items".into(),
        export_name: "mcp__test_items".into(),
        enabled: true,
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

    // A call reaches the command's cmd__ export with the tool arguments,
    // and the command's stdout is the tool result.
    let args = json!({"format": "md", "verbose": true});
    let resp = call_tool(&mut server, "specforge.cmds.report", args.clone());
    assert_eq!(
        resp["result"],
        json!({"content": [{"type": "text", "text": "3 of 4 covered"}], "isError": false})
    );
    assert_eq!(
        ext.calls(),
        [(EXT.to_string(), "cmd__report".to_string(), args)]
    );

    // A failing command is a failed tool result carrying its stderr.
    let failing = json!({"exit_code": 2, "stdout": "", "stderr": "no tests found"});
    let (mut server, _ext, _dir) =
        fake_extension::initialized(FakeExtension::new().with_output("cmd__report", failing));
    let resp = call_tool(
        &mut server,
        "specforge.cmds.report",
        json!({"format": "md"}),
    );
    assert_eq!(
        resp["result"],
        json!({"content": [
            {"type": "text", "text": ""},
            {"type": "text", "text": "no tests found"}
        ], "isError": true})
    );
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
        .surface_entries
        .iter()
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
                "format": {"type": "string", "enum": ["md", "json"], "description": "Output format"},
                "verbose": {"type": "boolean"},
                "limit": {"type": "integer"},
                "out": {"type": "string"}
            },
            "required": ["format"]
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
            "category": "core",
            "source": "@test/cmds"
        })]
    );
    let i017: Vec<_> = server
        .state()
        .diagnostics
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
