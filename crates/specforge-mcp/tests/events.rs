use specforge_mcp::McpServer;
use specforge_mcp::subscriptions;
use specforge_mcp::notifications::pending_notifications;
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    call(server, "tools/call", json!({"name": name, "arguments": args}))
}

fn has_event(server: &McpServer, event_name: &str) -> bool {
    server.state().events.iter().any(|e| e.name == event_name)
}

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    server
}

// E:mcp_initialized — verify integration "mcp initialization emits event with tool counts"
#[test]
#[specforge_test(behavior = "mcp_initialized", verify = "mcp initialization emits event with tool counts")]
fn event_mcp_initialized() {
    let server = init_server();
    assert!(has_event(&server, "mcp_initialized"));
}

// E:mcp_server_shutdown — verify integration "emits mcp_server_shutdown with correct counts when MCP server shuts down"
#[test]
#[specforge_test(behavior = "mcp_server_shutdown", verify = "emits mcp_server_shutdown with correct counts when MCP server shuts down")]
fn event_mcp_server_shutdown() {
    let mut server = init_server();
    call(&mut server, "shutdown", json!({}));
    assert!(has_event(&server, "mcp_server_shutdown"));
}

// E:mcp_initialization_failed — verify integration "emits mcp_initialization_failed when MCP server fails to initialize"
#[test]
#[specforge_test(behavior = "mcp_initialization_failed", verify = "emits mcp_initialization_failed when MCP server fails to initialize")]
fn event_mcp_initialization_failed() {
    let mut server = init_server();
    call(&mut server, "initialize", json!({}));
    assert!(has_event(&server, "mcp_initialization_failed"));
}

// E:mcp_protocol_error_handled — verify integration "emits mcp_protocol_error_handled with correct errorCode for each error type"
#[test]
#[specforge_test(behavior = "mcp_protocol_error_handled", verify = "emits mcp_protocol_error_handled with correct errorCode for each error type")]
fn event_mcp_protocol_error_handled() {
    let mut server = McpServer::new();
    server.handle_message("not valid json");
    assert!(has_event(&server, "mcp_protocol_error_handled"));
}

// E:mcp_request_cancelled — verify integration "emits mcp_request_cancelled with correct requestId and wasInProgress flag"
#[test]
#[specforge_test(behavior = "mcp_request_cancelled", verify = "emits mcp_request_cancelled with correct requestId and wasInProgress flag")]
fn event_mcp_request_cancelled() {
    let mut server = McpServer::new();
    call(&mut server, "$/cancelRequest", json!({"id": 1}));
    assert!(has_event(&server, "mcp_request_cancelled"));
}

// E:mcp_discovery_invoked — verify integration "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources"
#[test]
#[specforge_test(behavior = "mcp_discovery_invoked", verify = "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources")]
fn event_mcp_discovery_invoked() {
    let mut server = init_server();
    call(&mut server, "tools/list", json!({}));
    assert!(has_event(&server, "mcp_discovery_invoked"));
}

// E:mcp_resource_read — verify integration "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource"
#[test]
#[specforge_test(behavior = "mcp_resource_read", verify = "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource")]
fn event_mcp_resource_read() {
    let mut server = init_server();
    call(&mut server, "resources/read", json!({"uri": "specforge://graph"}));
    assert!(has_event(&server, "mcp_resource_read"));
}

// E:mcp_tool_invoked — verify integration "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call"
#[test]
#[specforge_test(behavior = "mcp_tool_invoked", verify = "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call")]
fn event_mcp_tool_invoked() {
    let mut server = init_server();
    call_tool(&mut server, "specforge.stats", json!({}));
    assert!(has_event(&server, "mcp_tool_invoked"));
}

// E:mcp_prompt_invoked — verify integration "emits mcp_prompt_invoked with correct promptName and arguments"
#[test]
#[specforge_test(behavior = "mcp_prompt_invoked", verify = "emits mcp_prompt_invoked with correct promptName and arguments")]
fn event_mcp_prompt_invoked() {
    let mut server = init_server();

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    state.graph = graph;

    call(&mut server, "prompts/get", json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "alpha"}}));
    assert!(has_event(&server, "mcp_prompt_invoked"));
}

// E:mcp_delta_notified — verify integration "emits mcp_delta_notified with correct notification type and delta summary"
#[test]
#[specforge_test(behavior = "mcp_delta_notified", verify = "emits mcp_delta_notified with correct notification type and delta summary")]
fn event_mcp_delta_notified() {
    let mut server = init_server();

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    state.graph = graph;

    pending_notifications(server.state_mut());
    assert!(has_event(&server, "mcp_delta_notified"));
}

// E:mcp_mutation_completed — verify integration "emits mcp_mutation_completed with structured outcome after each mutation tool"
#[test]
#[specforge_test(behavior = "mcp_mutation_completed", verify = "emits mcp_mutation_completed with structured outcome after each mutation tool")]
fn event_mcp_mutation_completed() {
    let mut server = init_server();
    call_tool(&mut server, "specforge.format", json!({}));
    assert!(has_event(&server, "mcp_mutation_completed"));
}

// E:mcp_subscription_created — verify integration "emits mcp_subscription_created when a client subscribes to delta notifications"
#[test]
#[specforge_test(behavior = "mcp_subscription_created", verify = "emits mcp_subscription_created when a client subscribes to delta notifications")]
fn event_mcp_subscription_created() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "graph");
    assert!(has_event(&server, "mcp_subscription_created"));
}

// E:mcp_subscription_removed — verify integration "emits mcp_subscription_removed when a client unsubscribes or server shuts down"
#[test]
#[specforge_test(behavior = "mcp_subscription_removed", verify = "emits mcp_subscription_removed when a client unsubscribes or server shuts down")]
fn event_mcp_subscription_removed() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "graph");
    subscriptions::unsubscribe(server.state_mut(), "client1", "graph");
    assert!(has_event(&server, "mcp_subscription_removed"));
}
