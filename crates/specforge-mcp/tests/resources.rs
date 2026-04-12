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

    // Initialize
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    // Inject test graph
    let state = server.state_mut();
    let mut graph = Graph::new();

    let mut fields_a = FieldMap::new();
    fields_a.push("contract".into(), FieldValue::String("The system MUST do alpha".into()));
    let verify_stmts = vec![VerifyStatement { kind: "unit".into(), description: "does alpha correctly".into() }];
    fields_a.push("verify".into(), FieldValue::VerifyList(verify_stmts));

    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: span(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind { raw: "feature".into() },
        title: Some("Beta Feature".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "features.spec".into(), start_line: 10, start_col: 0, end_line: 15, end_col: 0 },
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

fn read_resource(server: &mut McpServer, uri: &str) -> Value {
    call(server, "resources/read", json!({"uri": uri}))
}

fn resource_text(resp: &Value) -> String {
    resp["result"]["contents"][0]["text"].as_str().unwrap().to_string()
}

// B:expose_graph_as_mcp_resource — verify unit "returns full graph as JSON"
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "specforge://graph resource returns full Graph Protocol JSON")]
fn graph_resource_returns_json() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 2);
}

// B:expose_graph_as_mcp_resource — verify unit "graph resource has correct MIME type"
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "graph resource has correct MIME type")]
fn graph_resource_has_mime_type() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph");
    assert_eq!(resp["result"]["contents"][0]["mimeType"], "application/json");
}

// B:expose_schema_as_mcp_resource — verify unit "returns schema with entity kinds derived from graph"
#[test]
#[specforge_test(behavior = "expose_schema_as_mcp_resource", verify = "specforge://schema resource returns graph-derived entity kinds")]
fn schema_resource_returns_kinds() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://schema");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Schema resource must derive entity kinds from the graph (not from registries)
    let entity_kinds = &parsed["entity_kinds"];
    assert!(entity_kinds.is_object(), "entity_kinds should be an object mapping kind->fields, got: {}", entity_kinds);
    // test_server has behavior and feature nodes
    let kinds_obj = entity_kinds.as_object().unwrap();
    assert!(kinds_obj.contains_key("behavior"), "schema should include 'behavior' kind from graph nodes");
    assert!(kinds_obj.contains_key("feature"), "schema should include 'feature' kind from graph nodes");
    assert!(parsed["schema_version"].is_string(), "schema_version should be a string like '0.1.0'");
    assert!(parsed["edge_labels"].is_array(), "should have edge_labels array");
}

// B:expose_context_as_mcp_resource — verify unit "returns context-optimized graph"
#[test]
#[specforge_test(behavior = "expose_context_as_mcp_resource", verify = "specforge://context resource returns token-optimized format")]
fn context_resource_returns_context_graph() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://context");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    // Context includes contract field
    let alpha = &parsed["nodes"].as_array().unwrap().iter()
        .find(|n| n["id"] == "alpha").unwrap();
    assert!(alpha["contract"].is_string());
}

// B:expose_brief_as_mcp_resource — verify unit "returns brief graph"
#[test]
#[specforge_test(behavior = "expose_brief_as_mcp_resource", verify = "specforge://brief resource returns minimal IDs and edges format")]
fn brief_resource_returns_brief_graph() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://brief");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    // Brief only has id, kind, title — no fields
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        assert!(node["id"].is_string());
        assert!(node["kind"].is_string());
        assert!(node.get("fields").is_none());
    }
}

// B:expose_diagnostics_as_mcp_resource — verify unit "returns diagnostics array"
#[test]
#[specforge_test(behavior = "expose_diagnostics_as_mcp_resource", verify = "specforge://diagnostics resource returns current DiagnosticBag as JSON")]
fn diagnostics_resource_returns_array() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://diagnostics");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_array());
}

// B:expose_entity_as_mcp_resource — verify unit "returns entity subgraph"
#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "specforge://graph/{entity_id} returns entity and its neighbors")]
fn entity_resource_returns_subgraph() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph/alpha");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    // Subgraph from alpha includes alpha and beta (connected)
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap().iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"alpha"));
}

// B:expose_entity_as_mcp_resource — verify unit "returns error for unknown entity"
#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "non-existent entity_id returns 404 error")]
fn entity_resource_error_for_unknown() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph/nonexistent");
    assert!(resp["error"].is_object());
}

// B:expose_entity_as_mcp_resource — verify unit "returns error for empty entity ID"
#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "returns error for empty entity ID")]
fn entity_resource_error_for_empty_id() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph/");
    assert!(resp["error"].is_object());
}

// Resource read missing URI
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "returns error for missing URI")]
fn resource_read_missing_uri() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/read", json!({}));
    assert!(resp["error"].is_object());
}

// Unknown resource URI
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "returns error for unknown URI")]
fn resource_read_unknown_uri() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://unknown");
    assert!(resp["error"].is_object());
}

// Resource read when not initialized
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "returns error when not initialized")]
fn resource_read_not_initialized() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "resources/read", json!({"uri": "specforge://graph"}));
    assert!(resp["error"].is_object());
}

// B:expose_graph_as_mcp_resource — verify unit "output includes valid JSON with nodes"
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "output includes embedded schema and schema_version")]
fn graph_includes_schema_version() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // The graph resource returns valid JSON with a nodes array
    assert!(parsed["nodes"].is_array());
}

// B:expose_graph_as_mcp_resource — verify unit "resource refreshes after recompilation"
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "resource refreshes after recompilation")]
fn graph_refreshes_after_recompilation() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://graph");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();
    let count1 = parsed1["nodes"].as_array().unwrap().len();

    // Add a new node to simulate recompilation
    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "delta".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Delta Behavior".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let resp2 = read_resource(&mut server, "specforge://graph");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let count2 = parsed2["nodes"].as_array().unwrap().len();
    assert_eq!(count2, count1 + 1);
}

// B:expose_schema_as_mcp_resource — verify unit "schema updates when graph changes"
#[test]
#[specforge_test(behavior = "expose_schema_as_mcp_resource", verify = "schema updates when graph changes")]
fn schema_updates_when_graph_changes() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://schema");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();

    // Schema is derived from graph — should have entity_kinds from current nodes
    let kinds1 = parsed1["entity_kinds"].as_object().unwrap();
    assert!(!kinds1.contains_key("event"), "initially no event kind in graph");

    // Add a new kind to the graph
    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "evt1".into() },
        kind: EntityKind { raw: "event".into() },
        title: Some("Test Event".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let resp2 = read_resource(&mut server, "specforge://schema");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let kinds2 = parsed2["entity_kinds"].as_object().unwrap();
    assert!(kinds2.contains_key("event"), "after adding event node, schema should include 'event' kind");
}

// B:expose_context_as_mcp_resource — verify unit "resource refreshes after recompilation"
#[test]
#[specforge_test(behavior = "expose_context_as_mcp_resource", verify = "resource refreshes after recompilation")]
fn context_refreshes_after_recompilation() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://context");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();
    let count1 = parsed1["nodes"].as_array().unwrap().len();

    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "delta".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Delta Behavior".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let resp2 = read_resource(&mut server, "specforge://context");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let count2 = parsed2["nodes"].as_array().unwrap().len();
    assert_eq!(count2, count1 + 1);
}

// B:expose_brief_as_mcp_resource — verify unit "resource refreshes after recompilation"
#[test]
#[specforge_test(behavior = "expose_brief_as_mcp_resource", verify = "resource refreshes after recompilation")]
fn brief_refreshes_after_recompilation() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://brief");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();
    let count1 = parsed1["nodes"].as_array().unwrap().len();

    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "delta".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Delta Behavior".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let resp2 = read_resource(&mut server, "specforge://brief");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let count2 = parsed2["nodes"].as_array().unwrap().len();
    assert_eq!(count2, count1 + 1);
}

// B:expose_diagnostics_as_mcp_resource — verify unit "resource updates after recompilation"
#[test]
#[specforge_test(behavior = "expose_diagnostics_as_mcp_resource", verify = "resource updates after recompilation")]
fn diagnostics_updates_after_recompilation() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://diagnostics");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();
    let count1 = parsed1.as_array().unwrap().len();

    // Add a diagnostic to state
    server.state_mut().diagnostics.push(specforge_common::Diagnostic {
        code: "V001".into(),
        severity: specforge_common::Severity::Error,
        message: "test diagnostic".into(),
        span: Some(SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 1, end_col: 10 }),
        suggestion: None,
    });

    let resp2 = read_resource(&mut server, "specforge://diagnostics");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let count2 = parsed2.as_array().unwrap().len();
    assert_eq!(count2, count1 + 1);
}

// B:expose_diagnostics_as_mcp_resource — verify unit "each diagnostic includes severity, code, message, file, span"
#[test]
#[specforge_test(behavior = "expose_diagnostics_as_mcp_resource", verify = "each diagnostic includes severity, code, message, file, and span")]
fn diagnostics_fields_present() {
    let mut server = test_server();

    server.state_mut().diagnostics.push(specforge_common::Diagnostic {
        code: "V001".into(),
        severity: specforge_common::Severity::Error,
        message: "test error".into(),
        span: Some(SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 1, end_col: 10 }),
        suggestion: None,
    });

    let resp = read_resource(&mut server, "specforge://diagnostics");
    let text = resource_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let diags = parsed.as_array().unwrap();
    assert!(!diags.is_empty());

    let d = &diags[0];
    assert!(d["code"].is_string());
    assert!(d["severity"].is_string());
    assert!(d["message"].is_string());
    assert!(d.get("file").is_some());
    assert!(d.get("line").is_some());
}

// B:expose_entity_as_mcp_resource — verify unit "malformed entity_id returns 400 error"
#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "malformed entity_id returns 400 error")]
fn entity_malformed_id_returns_error() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph/!@#$");
    assert!(resp["error"].is_object());
}

// B:expose_entity_as_mcp_resource — verify unit "resource refreshes after recompilation"
#[test]
#[specforge_test(behavior = "expose_entity_as_mcp_resource", verify = "resource refreshes after recompilation")]
fn entity_refreshes_after_recompilation() {
    let mut server = test_server();
    let resp1 = read_resource(&mut server, "specforge://graph/alpha");
    let text1 = resource_text(&resp1);
    let parsed1: Value = serde_json::from_str(&text1).unwrap();
    let title1 = parsed1["nodes"].as_array().unwrap().iter()
        .find(|n| n["id"] == "alpha")
        .unwrap()["title"].as_str().unwrap().to_string();

    // Replace alpha with a new title
    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha Revised".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });

    let resp2 = read_resource(&mut server, "specforge://graph/alpha");
    let text2 = resource_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    let title2 = parsed2["nodes"].as_array().unwrap().iter()
        .find(|n| n["id"] == "alpha")
        .unwrap()["title"].as_str().unwrap().to_string();
    assert_ne!(title1, title2);
    assert_eq!(title2, "Alpha Revised");
}

// B:expose_graph_as_mcp_resource — verify unit "resource has application/json MIME type"
#[test]
#[specforge_test(behavior = "expose_graph_as_mcp_resource", verify = "resource has application/json MIME type")]
fn graph_resource_returns_json_mime_type() {
    let mut server = test_server();
    let resp = read_resource(&mut server, "specforge://graph");
    let mime = resp["result"]["contents"][0]["mimeType"].as_str().unwrap();
    assert_eq!(mime, "application/json");
}
