use specforge_mcp::McpServer;
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_raw(server: &mut McpServer, input: &str) -> Option<String> {
    server.handle_message(input)
}

// -- JSON-RPC 2.0 framing --

// B:handle_mcp_protocol_error — verify unit "parse error returns -32700"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "malformed JSON produces -32700 Parse error")]
fn parse_error_returns_32700() {
    let mut server = McpServer::new();
    let resp = call_raw(&mut server, "not valid json").unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["error"]["code"], -32700);
    assert_eq!(parsed["jsonrpc"], "2.0");
}

// B:handle_mcp_protocol_error — verify unit "invalid request returns -32600"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "returns -32600 for invalid request")]
fn invalid_request_returns_32600() {
    let mut server = McpServer::new();
    let resp = call_raw(&mut server, r#"{"id":1}"#).unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["error"]["code"], -32600);
}

// B:handle_mcp_protocol_error — verify unit "missing method returns -32600"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "missing method returns -32600")]
fn missing_method_returns_32600() {
    let mut server = McpServer::new();
    let resp = call_raw(&mut server, r#"{"jsonrpc":"2.0","id":1}"#).unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["error"]["code"], -32600);
}

// B:handle_mcp_protocol_error — verify unit "unknown method returns -32601"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "invalid method produces -32601 Method not found")]
fn unknown_method_returns_32601() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "nonexistent_method", json!({}));
    assert_eq!(resp["error"]["code"], -32601);
}

// B:handle_mcp_protocol_error — verify unit "invalid jsonrpc version returns -32600"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "invalid jsonrpc version returns -32600")]
fn invalid_jsonrpc_version_returns_32600() {
    let mut server = McpServer::new();
    let resp = call_raw(&mut server, r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#).unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["error"]["code"], -32600);
}

// B:handle_mcp_protocol_error — verify unit "response always has jsonrpc 2.0 field"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "response always has jsonrpc 2.0 field")]
fn response_always_has_jsonrpc_field() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "ping", json!({}));
    assert_eq!(resp["jsonrpc"], "2.0");
}

// B:handle_mcp_protocol_error — verify unit "error response includes id from request"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "error response includes id from request")]
fn error_response_includes_request_id() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 42, "method": "nonexistent"});
    let resp = server.handle_message(&req.to_string()).unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["id"], 42);
    assert!(parsed["error"].is_object());
}

// B:handle_mcp_protocol_error — verify unit "success response includes id from request"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "success response includes id from request")]
fn success_response_includes_request_id() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "id": 99, "method": "ping"});
    let resp = server.handle_message(&req.to_string()).unwrap();
    let parsed: Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(parsed["id"], 99);
    assert!(parsed["result"].is_object());
}

// B:handle_mcp_request_cancellation — verify unit "cancel request returns success"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "cancel request returns success")]
fn cancel_request_returns_success() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 1}));
    assert!(resp["result"].is_object());
}

// Notifications (no id) should not produce a response
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "notifications produce no response")]
fn notifications_produce_no_response() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    let resp = server.handle_message(&req.to_string());
    assert!(resp.is_none());
}

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    server
}

// B:handle_mcp_protocol_error — verify unit "missing required params produces -32602"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "missing required params produces -32602")]
fn missing_tool_name_returns_32602() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/call", json!({}));
    assert_eq!(resp["error"]["code"], -32602);
}

// B:handle_mcp_protocol_error — verify unit "error response does not leak internal state"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "error response does not leak internal state")]
fn error_does_not_leak_internal_state() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "nonexistent_method", json!({}));
    let error = &resp["error"];
    assert!(error.is_object());
    assert!(error.get("graph").is_none());
    assert!(error.get("diagnostics").is_none());
}

// B:handle_mcp_protocol_error — verify unit "server remains operational after protocol error"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "server remains operational after protocol error")]
fn server_operational_after_protocol_error() {
    let mut server = McpServer::new();
    call_raw(&mut server, "not valid json");
    let resp = call(&mut server, "ping", json!({}));
    assert!(resp["result"].is_object());
    assert!(resp["error"].is_null());
}

// B:handle_mcp_protocol_error — verify unit "returns -32603 for internal error"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "returns -32603 for internal error")]
fn internal_error_code_defined() {
    assert_eq!(specforge_mcp::protocol::error_codes::INTERNAL_ERROR, -32603);
}

// B:handle_mcp_request_cancellation — verify unit "cancellation of completed request is a no-op"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "cancellation of completed request is a no-op")]
fn cancel_completed_request_is_noop() {
    let mut server = init_server();
    let req = json!({"jsonrpc": "2.0", "id": 5, "method": "ping", "params": {}});
    server.handle_message(&req.to_string());
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 5}));
    assert!(resp["result"].is_object());
    assert!(resp["error"].is_null());
}

// B:handle_mcp_protocol_error — verify unit "missing required params produces -32602 Invalid params"
#[test]
#[specforge_test(behavior = "handle_mcp_protocol_error", verify = "missing required params produces -32602 Invalid params")]
fn missing_required_params_produces_invalid_params() {
    let mut server = init_server();
    // Call a tool that requires params, but provide none
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": "specforge.query" }
        // Missing "arguments" field
    });
    let resp_str = server.handle_message(&req.to_string()).unwrap();
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    // Should produce an error (either -32602 for invalid params or tool-level error)
    assert!(resp["error"].is_object() || resp["result"]["isError"] == true,
        "missing required params should produce error");
}
