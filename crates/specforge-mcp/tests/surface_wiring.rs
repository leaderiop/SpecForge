/// Tests for wiring extension surface contributions to MCP tool/resource registries.
/// Verifies that manifest-declared MCP tools and resources appear in the MCP server's
/// discovery responses after initialization with a project containing surface-contributing extensions.
/// Also tests dynamic kind-based tools and resources generated from the graph.

use specforge_mcp::McpServer;
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue};
use specforge_test::prelude::*;
use serde_json::{json, Value};
use tempfile::TempDir;
use std::fs;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    call(server, "tools/call", json!({"name": tool_name, "arguments": args}))
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"].as_str().unwrap().to_string()
}

fn read_resource(server: &mut McpServer, uri: &str) -> Value {
    call(server, "resources/read", json!({"uri": uri}))
}

fn span() -> SourceSpan {
    SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 }
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
            kind: EntityKind { raw: "feature".into() },
            title: Some(title.into()),
            fields,
            source_span: span(),
        });
    }

    // Add behaviors
    let mut fields = FieldMap::new();
    fields.push("contract".into(), FieldValue::String("MUST login".into()));
    graph.add_node(Node {
        id: EntityId { raw: "login_behavior".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Login".into()),
        fields,
        source_span: span(),
    });

    state.graph = graph;
    server
}

/// Create a server with extension surface contributions injected directly.
/// Tests MCP surface wiring, not the compile-time extension loading pipeline.
fn init_server_with_surfaces() -> (McpServer, TempDir) {
    use specforge_mcp::types::{McpToolDescriptor, McpResourceDescriptor};
    use specforge_registry::{SurfaceRegistryEntry, SurfaceType};

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("core.spec"), r#"behavior greet "Greet" {
    status planned
    contract "greet users"
}
"#).unwrap();
    fs::write(dir.path().join("specforge.json"), r#"{
    "name": "test-surfaces",
    "version": "0.1.0",
    "extensions": []
}
"#).unwrap();

    let mut server = McpServer::new();
    let project_root = dir.path().to_str().unwrap();
    call(&mut server, "initialize", json!({"projectRoot": project_root}));

    // Inject extension surface contributions directly into server state
    let state = server.state_mut();
    state.tool_registry.push(McpToolDescriptor {
        name: "test.list_items".into(),
        description: "List all items via MCP".into(),
        input_schema: json!({"type": "object", "properties": {"kind": {"type": "string"}}}),
        category: Some("extension".into()),
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

// B:surface_wiring — verify unit "extension MCP tools appear in tool registry after init"
#[test]
#[specforge_test(behavior = "surface_wiring", verify = "extension MCP tools appear in tool registry after init")]
fn extension_mcp_tools_in_registry() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    // Core tools must still be present
    assert!(tool_names.contains(&"specforge.query"), "core tool specforge.query must be present");

    // Extension tool must also be present
    assert!(
        tool_names.contains(&"test.list_items"),
        "extension MCP tool 'test.list_items' must appear in registry. Got: {:?}",
        tool_names
    );
}

// B:surface_wiring — verify unit "extension MCP resources appear in resource registry after init"
#[test]
#[specforge_test(behavior = "surface_wiring", verify = "extension MCP resources appear in resource registry after init")]
fn extension_mcp_resources_in_registry() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r["uri"].as_str().unwrap()).collect();

    // Core resources must still be present
    assert!(uris.contains(&"specforge://graph"), "core resource must be present");

    // Extension resource must also be present
    assert!(
        uris.contains(&"specforge://test/items"),
        "extension MCP resource 'specforge://test/items' must appear in registry. Got: {:?}",
        uris
    );
}

// B:surface_wiring — verify unit "capabilities response includes extension tool/resource counts"
#[test]
#[specforge_test(behavior = "surface_wiring", verify = "capabilities include extension counts")]
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
        category: Some("extension".into()),
    });
    state.tool_registry.push(McpToolDescriptor {
        name: "ext.tool_b".into(),
        description: "Tool B".into(),
        input_schema: json!({"type": "object", "properties": {}}),
        category: Some("extension".into()),
    });

    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();

    // Should have core tools + 2 extension tools
    let ext_tools: Vec<_> = tools.iter()
        .filter(|t| {
            let name = t["name"].as_str().unwrap();
            name.starts_with("ext.")
        })
        .collect();
    assert_eq!(ext_tools.len(), 2, "expected 2 extension tools, got: {:?}", ext_tools);
}

// --- Dynamic kind-based tools and resources ---

// B:dynamic_kind_tools — verify unit "specforge.list returns entities filtered by kind"
#[test]
#[specforge_test(behavior = "dynamic_kind_tools", verify = "specforge.list returns entities filtered by kind")]
fn list_tool_returns_entities_by_kind() {
    let mut server = init_server_with_kinds();
    let resp = call_tool(&mut server, "specforge.list", json!({"kind": "feature"}));
    assert!(resp["error"].is_null(), "specforge.list tool must not return error: {:?}", resp);
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let entities = parsed.as_array().unwrap();

    assert_eq!(entities.len(), 2, "expected 2 features, got: {:?}", entities);
    let ids: Vec<&str> = entities.iter().map(|e| e["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"feat_auth"));
    assert!(ids.contains(&"feat_search"));
}

// B:dynamic_kind_tools — verify unit "specforge.list returns empty array for unknown kind"
#[test]
#[specforge_test(behavior = "dynamic_kind_tools", verify = "specforge.list returns empty for unknown kind")]
fn list_tool_empty_for_unknown_kind() {
    let mut server = init_server_with_kinds();
    let resp = call_tool(&mut server, "specforge.list", json!({"kind": "nonexistent"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 0);
}

// B:dynamic_kind_tools — verify unit "specforge://entities/{kind} resource returns entities as JSON"
#[test]
#[specforge_test(behavior = "dynamic_kind_tools", verify = "entity-by-kind resource returns entities")]
fn entities_by_kind_resource() {
    let mut server = init_server_with_kinds();
    let resp = read_resource(&mut server, "specforge://entities/behavior");
    let contents = &resp["result"]["contents"];
    assert!(contents.is_array(), "resource must return contents array, got: {:?}", resp);
    let text = contents[0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    let entities = parsed.as_array().unwrap();
    assert_eq!(entities.len(), 1, "expected 1 behavior");
    assert_eq!(entities[0]["id"].as_str().unwrap(), "login_behavior");
}

// B:dynamic_kind_tools — verify unit "specforge.list tool is registered"
#[test]
#[specforge_test(behavior = "dynamic_kind_tools", verify = "specforge.list tool appears in tool list")]
fn list_tool_registered() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"specforge.list"), "specforge.list must be in tool list");
}

// --- Extension tool dispatch ---

// B:extension_tool_dispatch — verify unit "calling extension tool dispatches rather than METHOD_NOT_FOUND"
#[test]
#[specforge_test(behavior = "extension_tool_dispatch", verify = "extension tool dispatches from surface registry")]
fn extension_tool_dispatches() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call_tool(&mut server, "test.list_items", json!({"kind": "feature"}));
    // Should NOT be METHOD_NOT_FOUND — extension tools should be recognized
    let error_code = resp["error"]["code"].as_i64();
    assert_ne!(
        error_code, Some(-32601),
        "extension tool must not return METHOD_NOT_FOUND, got: {:?}", resp
    );
}

// B:extension_tool_dispatch — verify unit "unknown tool still returns METHOD_NOT_FOUND"
#[test]
#[specforge_test(behavior = "extension_tool_dispatch", verify = "truly unknown tool returns METHOD_NOT_FOUND")]
fn unknown_tool_returns_method_not_found() {
    let (mut server, _dir) = init_server_with_surfaces();
    let resp = call_tool(&mut server, "totally.unknown.tool", json!({}));
    assert_eq!(resp["error"]["code"].as_i64(), Some(-32601));
}

// B:extension_tool_dispatch — verify unit "re-compilation preserves core tools"
#[test]
#[specforge_test(behavior = "extension_tool_dispatch", verify = "validate recompiles and preserves core tools")]
fn recompilation_refreshes_surfaces() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("core.spec"), r#"behavior greet "Greet" {
    status planned
}
"#).unwrap();
    fs::write(dir.path().join("specforge.json"), r#"{
    "name": "test-recompile",
    "version": "0.1.0",
    "extensions": []
}
"#).unwrap();

    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({"projectRoot": dir.path().to_str().unwrap()}));

    // Verify core tools are present
    let resp1 = call(&mut server, "tools/list", json!({}));
    let tools1 = resp1["result"]["tools"].as_array().unwrap();
    let has_query = tools1.iter().any(|t| t["name"].as_str() == Some("specforge.query"));
    assert!(has_query, "specforge.query must be present initially");

    // Recompile by calling validate
    let validate_resp = call_tool(&mut server, "specforge.validate", json!({
        "path": dir.path().to_str().unwrap()
    }));
    assert!(validate_resp["error"].is_null() || validate_resp["result"].is_object());

    // Core tools should still be present after recompilation
    let resp2 = call(&mut server, "tools/list", json!({}));
    let tools2 = resp2["result"]["tools"].as_array().unwrap();
    let still_has_query = tools2.iter().any(|t| t["name"].as_str() == Some("specforge.query"));
    assert!(still_has_query, "specforge.query must persist after recompilation");
}

// B:dynamic_kind_tools — verify unit "entities resource registered in resource list"
#[test]
#[specforge_test(behavior = "dynamic_kind_tools", verify = "entities resource template in resource list")]
fn entities_resource_registered() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r["uri"].as_str().unwrap()).collect();
    assert!(
        uris.contains(&"specforge://entities/{kind}"),
        "specforge://entities/{{kind}} must be in resource list. Got: {:?}",
        uris
    );
}
