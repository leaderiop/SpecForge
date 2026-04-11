use specforge_mcp::McpServer;
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn span() -> SourceSpan {
    SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 }
}

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    let mut fields = FieldMap::new();
    fields.push("contract".into(), FieldValue::String("MUST work".into()));
    fields.push("verify".into(), FieldValue::VerifyList(vec![
        VerifyStatement { kind: "unit".into(), description: "works".into() },
    ]));

    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha".into()),
        fields,
        source_span: span(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind { raw: "feature".into() },
        title: Some("Beta".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "feat.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    graph.add_edge(Edge { source: "beta".into(), target: "alpha".into(), label: "behaviors".into() });
    state.graph = graph;

    server
}

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    call(server, "tools/call", json!({"name": name, "arguments": args}))
}

#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "requires/ensures consistency for MCP initialization")]
fn contract_initialize() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let result = &resp["result"];
    assert!(result["tools"].is_array());
    assert!(result["resources"].is_array());
    assert!(result["prompts"].is_array());
    assert!(result["server_name"].is_string());
    assert!(result["server_version"].is_string());
}

#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "requires/ensures consistency for MCP shutdown")]
fn contract_shutdown() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "shutdown", json!({}));
    assert!(resp["result"].is_object());
}

#[test]
#[specforge_test(behavior = "provide_mcp_query_tool", verify = "requires/ensures consistency for MCP query tool")]
fn contract_query() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.query", json!({"entity_id": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_export_tool", verify = "requires/ensures consistency for MCP export tool")]
fn contract_export() {
    let mut server = test_server();
    for format in &["graph", "context", "brief"] {
        let resp = call_tool(&mut server, "specforge.export", json!({"format": format}));
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert!(parsed["nodes"].is_array(), "format {} should have nodes", format);
    }
}

#[test]
#[specforge_test(behavior = "provide_mcp_trace_tool", verify = "requires/ensures consistency for MCP trace tool")]
fn contract_trace() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.trace", json!({"entity_id": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_search_tool", verify = "requires/ensures consistency for MCP search tool")]
fn contract_search() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_stats_tool", verify = "requires/ensures consistency for MCP stats tool")]
fn contract_stats() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_counts"].is_array());
    assert!(parsed["diagnostic_summary"].is_object());
}

#[test]
#[specforge_test(behavior = "provide_mcp_inspect_tool", verify = "requires/ensures consistency for MCP inspect tool")]
fn contract_inspect() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["kind"].is_string());
    assert!(parsed["source_span"].is_object());
}

#[test]
#[specforge_test(behavior = "provide_mcp_find_definition_tool", verify = "requires/ensures consistency for MCP find definition tool")]
fn contract_find_definition() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_definition", json!({"entity_id": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["file_path"].is_string());
    assert!(parsed["line"].is_number());
}

#[test]
#[specforge_test(behavior = "provide_mcp_find_references_tool", verify = "requires/ensures consistency for MCP find references tool")]
fn contract_find_references() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.find_references", json!({"entity_id": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["locations"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_outline_tool", verify = "requires/ensures consistency for MCP outline tool")]
fn contract_outline() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.outline", json!({"file": "test.spec"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_coverage_tool", verify = "requires/ensures consistency for MCP coverage tool")]
fn contract_coverage() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_schema_tool", verify = "requires/ensures consistency for MCP schema tool")]
fn contract_schema() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_kinds"].is_object());
}

#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "requires/ensures consistency for MCP context prompt")]
fn contract_context_prompt() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/get", json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "alpha"}}));
    assert!(resp["result"]["messages"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "requires/ensures consistency for MCP review prompt")]
fn contract_review_prompt() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/get", json!({"name": "specforge://prompts/review", "arguments": {}}));
    assert!(resp["result"]["messages"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_trace_prompt", verify = "requires/ensures consistency for MCP trace prompt")]
fn contract_trace_prompt() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/get", json!({"name": "specforge://prompts/trace", "arguments": {"entity_id": "alpha"}}));
    assert!(resp["result"]["messages"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "requires/ensures consistency for MCP explore prompt")]
fn contract_explore_prompt() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/get", json!({"name": "specforge://prompts/explore", "arguments": {}}));
    assert!(resp["result"]["messages"].is_array());
}

#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "requires/ensures consistency for listing MCP tools")]
fn contract_list_tools() {
    let mut server = test_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"].is_object());
    }
}

#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "requires/ensures consistency for listing MCP resources")]
fn contract_list_resources() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    for res in resources {
        assert!(res["uri"].is_string());
        assert!(res["name"].is_string());
    }
}

#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "requires/ensures consistency for listing MCP prompts")]
fn contract_list_prompts() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    for prompt in prompts {
        assert!(prompt["name"].is_string());
        assert!(prompt["description"].is_string());
    }
}

#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "requires/ensures consistency for MCP reinitialization guard")]
fn contract_guard_reinit() {
    let mut server = test_server();
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["error"].is_object());
}

#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "requires/ensures consistency for MCP request cancellation")]
fn contract_cancel() {
    let mut server = test_server();
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 1}));
    assert!(resp["result"].is_object() || resp["result"].is_null());
}

#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "requires/ensures consistency for MCP protocol error handling")]
fn contract_protocol_error() {
    let mut server = test_server();
    let resp = call(&mut server, "nonexistent/method", json!({}));
    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(resp["error"]["code"].is_number());
    assert!(resp["error"]["message"].is_string());
}

#[test]
#[specforge_test(behavior = "provide_mcp_validate_tool", verify = "requires/ensures consistency for MCP validate tool")]
fn contract_validate() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    assert!(resp["error"].is_object() || resp["result"]["content"][0]["text"].is_string(),
        "validate must return error or diagnostics text");
}

#[test]
#[specforge_test(behavior = "provide_mcp_suggest_fixes_tool", verify = "requires/ensures consistency for MCP suggest fixes tool")]
fn contract_suggest_fixes() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.suggest_fixes", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_format_tool", verify = "requires/ensures consistency for MCP format tool")]
fn contract_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.format", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("changed_files").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_rename_tool", verify = "requires/ensures consistency for MCP rename tool")]
fn contract_rename() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.rename", json!({"entity_id": "alpha", "new_name": "alpha_v2"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("old_name").is_some());
    assert!(parsed.get("new_name").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_init_tool", verify = "requires/ensures consistency for MCP init tool")]
fn contract_init() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.init", json!({"path": "/tmp/test"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("project_path").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_extensions_tool", verify = "requires/ensures consistency for MCP extensions tool")]
fn contract_extensions() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("extensions").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "requires/ensures consistency for MCP doctor tool")]
fn contract_doctor() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("extensions_ok").is_some());
}

// Additional contracts for remaining behaviors

#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "requires/ensures consistency for graph MCP resource")]
fn contract_graph_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://graph"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "expose_schema_as_mcp_resource", verify = "requires/ensures consistency for schema MCP resource")]
fn contract_schema_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://schema"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "expose_context_as_mcp_resource", verify = "requires/ensures consistency for context MCP resource")]
fn contract_context_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://context"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "expose_brief_as_mcp_resource", verify = "requires/ensures consistency for brief MCP resource")]
fn contract_brief_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://brief"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "expose_diagnostics_as_mcp_resource", verify = "requires/ensures consistency for diagnostics MCP resource")]
fn contract_diagnostics_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://diagnostics"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "requires/ensures consistency for per-entity MCP resource")]
fn contract_entity_resource() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://graph/alpha"}));
    assert!(resp["result"]["contents"].is_array());
}

#[test]
#[specforge_test(behavior = "notify_graph_delta_via_mcp", verify = "requires/ensures consistency for graph delta MCP notification")]
fn contract_graph_notification() {
    use specforge_mcp::notifications::*;
    let g1 = specforge_graph::Graph::new();
    let mut g2 = specforge_graph::Graph::new();
    g2.add_node(Node {
        id: EntityId { raw: "x".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
    });
    let delta = compute_graph_delta(&g1, &g2);
    let notif = format_graph_notification(&delta);
    assert_eq!(notif["method"], "specforge/graphChanged");
    assert!(notif["params"]["added_nodes"].is_array());
}

#[test]
#[specforge_test(behavior = "notify_diagnostics_delta_via_mcp", verify = "requires/ensures consistency for diagnostics delta MCP notification")]
fn contract_diagnostics_notification() {
    use specforge_mcp::notifications::*;
    use specforge_common::{Diagnostic, Severity};
    let d = Diagnostic { code: "V001".into(), severity: Severity::Error, message: "err".into(), span: None, suggestion: None };
    let delta = compute_diagnostics_delta(&[], &[d]);
    let notif = format_diagnostics_notification(&delta);
    assert_eq!(notif["method"], "specforge/diagnosticsChanged");
    assert!(notif["params"]["added"].is_array());
}

#[test]
#[specforge_test(behavior = "provide_mcp_add_extension_tool", verify = "requires/ensures consistency for MCP add extension tool")]
fn contract_add_extension() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.add_extension", json!({"specifier": "@specforge/software"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("installed").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_remove_extension_tool", verify = "requires/ensures consistency for MCP remove extension tool")]
fn contract_remove_extension() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.remove_extension", json!({"name": "@specforge/software"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("success").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_migrate_tool", verify = "requires/ensures consistency for MCP migrate tool")]
fn contract_migrate() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.migrate", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("migrated").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_providers_tool", verify = "requires/ensures consistency for MCP providers tool")]
fn contract_providers() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("providers").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "requires/ensures consistency for MCP collect tool")]
fn contract_collect() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("report_path").is_some());
}

#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "requires/ensures consistency for MCP render tool")]
fn contract_render() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "json"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("format").is_some());
}
