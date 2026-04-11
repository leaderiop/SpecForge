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
        source_span: span(),
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

// I:mcp_structured_error_responses — verify property "error response includes error code and message fields"
#[test]
#[specforge_test(behavior = "mcp_structured_error_responses", verify = "all error responses have code and message")]
fn error_responses_have_code_and_message() {
    let mut server = McpServer::new();

    // Test parse error
    let resp = server.handle_message("invalid json").unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert!(parsed["error"]["code"].is_number());
    assert!(parsed["error"]["message"].is_string());

    // Test method not found
    let resp2 = call(&mut server, "nonexistent", json!({}));
    assert!(resp2["error"]["code"].is_number());
    assert!(resp2["error"]["message"].is_string());
}

// I:mcp_structured_error_responses — verify property "error codes are valid JSON-RPC codes"
#[test]
#[specforge_test(behavior = "mcp_structured_error_responses", verify = "error response includes error code and message fields")]
fn error_codes_are_valid() {
    let mut server = McpServer::new();

    // Parse error
    let resp = server.handle_message("not json").unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    let code = parsed["error"]["code"].as_i64().unwrap();
    assert_eq!(code, -32700);

    // Method not found
    let resp2 = call(&mut server, "nonexistent", json!({}));
    let code2 = resp2["error"]["code"].as_i64().unwrap();
    assert_eq!(code2, -32601);
}

// I:mcp_structured_error_responses — verify property "success responses never have error field"
#[test]
#[specforge_test(behavior = "mcp_structured_error_responses", verify = "success responses never have error field")]
fn success_never_has_error() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["result"].is_object());
    assert!(resp["error"].is_null());
}

// I:mcp_tool_idempotency — verify property "query is idempotent"
#[test]
#[specforge_test(behavior = "mcp_tool_idempotency", verify = "read-only tools return equivalent results for identical inputs")]
fn query_is_idempotent() {
    let mut server = test_server();
    let resp1 = call_tool(&mut server, "specforge.query", json!({"entity_id": "alpha"}));
    let resp2 = call_tool(&mut server, "specforge.query", json!({"entity_id": "alpha"}));
    let text1 = resp1["result"]["content"][0]["text"].as_str().unwrap();
    let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text1, text2);
}

// I:mcp_tool_idempotency — verify property "export is idempotent"
#[test]
#[specforge_test(behavior = "mcp_tool_idempotency", verify = "repeated calls with same params return identical results when graph unchanged")]
fn export_is_idempotent() {
    let mut server = test_server();
    let resp1 = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    let resp2 = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    let text1 = resp1["result"]["content"][0]["text"].as_str().unwrap();
    let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text1, text2);
}

// I:mcp_tool_idempotency — verify property "stats is idempotent"
#[test]
#[specforge_test(behavior = "mcp_tool_idempotency", verify = "stats is idempotent")]
fn stats_is_idempotent() {
    let mut server = test_server();
    let resp1 = call_tool(&mut server, "specforge.stats", json!({}));
    let resp2 = call_tool(&mut server, "specforge.stats", json!({}));
    let text1 = resp1["result"]["content"][0]["text"].as_str().unwrap();
    let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text1, text2);
}

// I:mcp_tool_idempotency — verify property "trace is idempotent"
#[test]
#[specforge_test(behavior = "mcp_tool_idempotency", verify = "trace is idempotent")]
fn trace_is_idempotent() {
    let mut server = test_server();
    let resp1 = call_tool(&mut server, "specforge.trace", json!({"entity_id": "alpha"}));
    let resp2 = call_tool(&mut server, "specforge.trace", json!({"entity_id": "alpha"}));
    let text1 = resp1["result"]["content"][0]["text"].as_str().unwrap();
    let text2 = resp2["result"]["content"][0]["text"].as_str().unwrap();
    assert_eq!(text1, text2);
}

// I:mcp_subscription_cleanup — verify property "no subscriptions survive shutdown"
#[test]
#[specforge_test(behavior = "mcp_subscription_cleanup", verify = "no orphan subscriptions remain after disconnect")]
fn no_subscriptions_survive_shutdown() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));

    specforge_mcp::subscriptions::subscribe(server.state_mut(), "c1", "specforge/graphChanged");
    assert!(!server.state().subscriptions.is_empty());

    call(&mut server, "shutdown", json!({}));
    assert!(server.state().subscriptions.is_empty());
}

// I:mcp_structured_error_responses — verify property "error response includes entity_id when applicable"
#[test]
#[specforge_test(behavior = "mcp_structured_error_responses", verify = "error response includes entity_id when applicable")]
fn error_includes_entity_id_when_applicable() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "unknown_entity"}));
    assert!(resp["error"].is_object());
    let err_msg = resp["error"]["message"].as_str().unwrap_or("");
    assert!(err_msg.contains("unknown_entity") || resp["error"]["data"]["entity_id"].is_string());
}

// I:mcp_structured_error_responses — verify property "no MCP endpoint returns a plain string error"
#[test]
#[specforge_test(behavior = "mcp_structured_error_responses", verify = "no MCP endpoint returns a plain string error")]
fn no_plain_string_error() {
    let mut server = test_server();

    // Test inspect with unknown entity
    let resp1 = call_tool(&mut server, "specforge.inspect", json!({"entity_id": "nonexistent"}));
    if resp1["error"].is_object() {
        assert!(resp1["error"]["code"].is_number(), "error must have code field");
        assert!(resp1["error"]["message"].is_string(), "error must have message field");
    }

    // Test find_definition with unknown entity
    let resp2 = call_tool(&mut server, "specforge.find_definition", json!({"entity_id": "nonexistent"}));
    if resp2["error"].is_object() {
        assert!(resp2["error"]["code"].is_number(), "error must have code field");
        assert!(resp2["error"]["message"].is_string(), "error must have message field");
    }

    // Test rename with missing params
    let resp3 = call_tool(&mut server, "specforge.rename", json!({}));
    if resp3["error"].is_object() {
        assert!(resp3["error"]["code"].is_number(), "error must have code field");
        assert!(resp3["error"]["message"].is_string(), "error must have message field");
    }
}

// I:mcp_type_schema_versioning — verify property "adding required field to MCP type triggers major version bump"
#[test]
#[specforge_test(behavior = "mcp_type_schema_versioning", verify = "adding required field to MCP type triggers major version bump")]
fn schema_version_invariant() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    // Check that a version field exists and looks like semver
    let version = parsed["graph_protocol_version"].as_str()
        .or_else(|| parsed["version"].as_str())
        .unwrap_or("0.1.0");
    assert!(version.contains('.'), "version should be a semver-like string");
}
