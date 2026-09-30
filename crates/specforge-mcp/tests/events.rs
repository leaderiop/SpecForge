use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_mcp::McpServer;
use specforge_mcp::notifications::pending_notifications;
use specforge_mcp::subscriptions;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;

// Leak a per-test temp project: process exits make cleanup unnecessary, and
// a real project root is required now that ops perform real work.
fn attach_project(state: &mut specforge_mcp::state::McpState) {
    let dir = tempfile::TempDir::new().unwrap();
    let config = json!({"name":"t","version":"0.1.0","extensions":[]});
    std::fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();
    std::fs::write(
        dir.path().join("test.spec"),
        "behavior alpha \"Alpha\" {\n}\nfeature beta \"Beta\" {\n    behaviors [alpha]\n}\n",
    )
    .unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir); // outlives the test
    state.project_root = Some(root);
}

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn call_tool(server: &mut McpServer, name: &str, args: Value) -> Value {
    call(
        server,
        "tools/call",
        json!({"name": name, "arguments": args}),
    )
}

fn has_event(server: &McpServer, event_name: &str) -> bool {
    server.state().events.iter().any(|e| e.name == event_name)
}

/// The params of every `event_name` event, oldest first.
fn event_params(server: &McpServer, event_name: &str) -> Vec<Value> {
    server
        .state()
        .events
        .iter()
        .filter(|e| e.name == event_name)
        .map(|e| e.params.clone())
        .collect()
}

/// The params of the one `event_name` event.
fn only_event(server: &McpServer, event_name: &str) -> Value {
    let params = event_params(server, event_name);
    assert_eq!(params.len(), 1, "{event_name}: {params:?}");
    params.into_iter().next().unwrap()
}

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    attach_project(server.state_mut());
    server
}

// E:mcp_initialized — verify integration "mcp initialization emits event with tool counts"
#[specforge_test(
    behavior = "mcp_initialized",
    verify = "mcp initialization emits event with tool counts"
)]
fn event_mcp_initialized() {
    let mut server = init_server();
    let params = only_event(&server, "mcp_initialized");
    // The core surface: 33 tools, 8 resources, 5 prompts.
    assert_eq!(params["tools_count"], 33, "{params}");
    assert_eq!(params["resources_count"], 8, "{params}");
    assert_eq!(params["prompts_count"], 5, "{params}");
    let listed = |server: &mut McpServer, method: &str, key: &str| {
        call(server, method, json!({}))["result"][key]
            .as_array()
            .unwrap()
            .len()
    };
    assert_eq!(listed(&mut server, "tools/list", "tools"), 33);
    assert_eq!(listed(&mut server, "resources/list", "resources"), 8);
    assert_eq!(listed(&mut server, "prompts/list", "prompts"), 5);
}

// E:mcp_server_shutdown — verify integration "emits mcp_server_shutdown with correct counts when MCP server shuts down"
#[specforge_test(
    behavior = "mcp_server_shutdown",
    verify = "emits mcp_server_shutdown with correct counts when MCP server shuts down"
)]
fn event_mcp_server_shutdown() {
    let mut server = init_server();
    // Two subscriptions and one notification waiting to be sent.
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");
    subscriptions::subscribe(
        server.state_mut(),
        "client2",
        "specforge/diagnosticsChanged",
    );
    server
        .state_mut()
        .notification_outbox
        .push(json!({"jsonrpc": "2.0", "method": "specforge/graphChanged", "params": {}}));
    call(&mut server, "shutdown", json!({}));
    let params = only_event(&server, "mcp_server_shutdown");
    assert_eq!(params["pending_notifications_flushed"], 1, "{params}");
    assert_eq!(params["subscriptions_released"], 2, "{params}");
    assert_eq!(params["wasm_engines_released"], 0, "{params}");
}

// E:mcp_initialization_failed — verify integration "emits mcp_initialization_failed when MCP server fails to initialize"
#[specforge_test(
    behavior = "mcp_initialization_failed",
    verify = "emits mcp_initialization_failed when MCP server fails to initialize"
)]
fn event_mcp_initialization_failed() {
    let mut server = init_server();
    call(&mut server, "initialize", json!({}));
    assert!(has_event(&server, "mcp_initialization_failed"));
}

// E:mcp_protocol_error_handled — verify integration "emits mcp_protocol_error_handled with correct errorCode for each error type"
#[specforge_test(
    behavior = "mcp_protocol_error_handled",
    verify = "emits mcp_protocol_error_handled with correct errorCode for each error type"
)]
fn event_mcp_protocol_error_handled() {
    let mut server = McpServer::new();
    server.handle_message("not valid json");
    server.handle_message(r#"{"id": 2}"#);
    call(&mut server, "initialize", json!({}));
    call(&mut server, "no/such/method", json!({}));
    call(&mut server, "tools/call", json!({}));
    let errors: Vec<(i64, String)> = event_params(&server, "mcp_protocol_error_handled")
        .iter()
        .map(|p| {
            (
                p["errorCode"].as_i64().unwrap(),
                p["errorMessage"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let codes: Vec<i64> = errors.iter().map(|(c, _)| *c).collect();
    assert_eq!(codes, vec![-32700, -32600, -32601, -32602], "{errors:?}");
    assert!(errors.iter().all(|(_, m)| !m.is_empty()), "{errors:?}");
    let last = event_params(&server, "mcp_protocol_error_handled")
        .pop()
        .unwrap();
    assert_eq!(last["method"], "tools/call");
}

// E:mcp_request_cancelled — verify integration "emits mcp_request_cancelled with correct requestId and wasInProgress flag"
#[specforge_test(
    behavior = "mcp_request_cancelled",
    verify = "emits mcp_request_cancelled with correct requestId and wasInProgress flag"
)]
fn event_mcp_request_cancelled() {
    let mut server = init_server();
    call(&mut server, "ping", json!({}));
    // The MCP notification names the request by requestId.
    server.handle_message(
        &json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
            "params": {"requestId": 41}})
        .to_string(),
    );
    call(&mut server, "$/cancelRequest", json!({"id": 42}));
    let events = event_params(&server, "mcp_request_cancelled");
    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(events[0]["requestId"], 41);
    assert_eq!(events[1]["requestId"], 42);
    // Requests run one at a time, so none is in progress when a cancel
    // arrives.
    for event in &events {
        assert_eq!(event["wasInProgress"], false, "{event}");
    }
}

// E:mcp_discovery_invoked — verify integration "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources"
#[specforge_test(
    behavior = "mcp_discovery_invoked",
    verify = "emits mcp_discovery_invoked with correct discoveryType when agent lists tools, prompts, or resources"
)]
fn event_mcp_discovery_invoked() {
    let mut server = init_server();
    call(&mut server, "tools/list", json!({}));
    call(&mut server, "prompts/list", json!({}));
    call(&mut server, "resources/list", json!({}));
    let discoveries: Vec<(String, u64)> = event_params(&server, "mcp_discovery_invoked")
        .iter()
        .map(|p| {
            (
                p["kind"].as_str().unwrap().to_string(),
                p["result_count"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        discoveries,
        vec![
            ("tools".to_string(), 33),
            ("prompts".to_string(), 5),
            ("resources".to_string(), 8),
        ]
    );
}

// E:mcp_resource_read — verify integration "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource"
#[specforge_test(
    behavior = "mcp_resource_read",
    verify = "emits mcp_resource_read with correct resourceUri when agent reads any MCP resource"
)]
fn event_mcp_resource_read() {
    let mut server = init_server();
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph"}),
    );
    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );
    let uris: Vec<Value> = event_params(&server, "mcp_resource_read")
        .into_iter()
        .map(|p| p["uri"].clone())
        .collect();
    assert_eq!(
        uris,
        vec![json!("specforge://graph"), json!("specforge://diagnostics")]
    );
}

// E:mcp_tool_invoked — verify integration "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call"
#[specforge_test(
    behavior = "mcp_tool_invoked",
    verify = "emits mcp_tool_invoked with correct toolName, category, and parameters for any tool call"
)]
fn event_mcp_tool_invoked() {
    let mut server = init_server();
    call_tool(&mut server, "specforge.stats", json!({}));
    call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    let events = event_params(&server, "mcp_tool_invoked");
    assert_eq!(
        events,
        vec![
            json!({"tool": "specforge.stats", "category": "core", "params": {}}),
            json!({"tool": "specforge.inspect", "category": "navigation",
                "params": {"entity_id": "alpha"}}),
        ]
    );
}

// E:mcp_prompt_invoked — verify integration "emits mcp_prompt_invoked with correct promptName and arguments"
#[specforge_test(
    behavior = "mcp_prompt_invoked",
    verify = "emits mcp_prompt_invoked with correct promptName and arguments"
)]
fn event_mcp_prompt_invoked() {
    let mut server = init_server();

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "test.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    state.graph = graph;
    attach_project(state);

    call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "alpha"}}),
    );
    assert_eq!(
        only_event(&server, "mcp_prompt_invoked"),
        json!({"prompt": "specforge://prompts/context", "arguments": {"entity_id": "alpha"}})
    );
}

// E:mcp_delta_notified — verify integration "emits mcp_delta_notified with correct notification type and delta summary"
#[specforge_test(
    behavior = "mcp_delta_notified",
    verify = "emits mcp_delta_notified with correct notification type and delta summary"
)]
fn event_mcp_delta_notified() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "specforge/graphChanged");

    // Attach a real project plus a watch snapshot marker so the routed read
    // performs an honest recompile (C9-07 staleness path).
    attach_project(server.state_mut());
    let root = server.state().project_root.clone().unwrap();
    let marker_dir = root.join(".specforge");
    std::fs::create_dir_all(&marker_dir).unwrap();
    std::fs::write(marker_dir.join("graph.json"), "{}").unwrap();

    call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );

    let notifications = pending_notifications(server.state_mut());
    assert_eq!(
        notifications.len(),
        1,
        "subscribed client must receive the graph delta: {notifications:?}"
    );
    assert_eq!(notifications[0]["method"], "specforge/graphChanged");
    // The empty graph became alpha, beta and the beta -> alpha edge.
    assert_eq!(
        only_event(&server, "mcp_delta_notified"),
        json!({"kind": "graph", "subscribers": 1, "added": 2, "removed": 0,
            "added_edges": 1, "removed_edges": 0})
    );
}

// E:mcp_mutation_completed — verify integration "emits mcp_mutation_completed with structured outcome after each mutation tool"
#[specforge_test(
    behavior = "mcp_mutation_completed",
    verify = "emits mcp_mutation_completed with structured outcome after each mutation tool"
)]
fn event_mcp_mutation_completed() {
    let mut server = init_server();
    let resp = call_tool(&mut server, "specforge.format", json!({}));
    let result: Value =
        serde_json::from_str(resp["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let event = only_event(&server, "mcp_mutation_completed");
    assert_eq!(event["tool"], "specforge.format");
    assert_eq!(event["success"], true);
    // The outcome is the tool's structured result.
    assert_eq!(event["outcome"], result);
    assert!(event["outcome"].is_object(), "{event}");

    // A read-only tool completes no mutation.
    call_tool(&mut server, "specforge.stats", json!({}));
    assert_eq!(event_params(&server, "mcp_mutation_completed").len(), 1);
}

// E:mcp_subscription_created — verify integration "emits mcp_subscription_created when a client subscribes to delta notifications"
#[specforge_test(
    behavior = "mcp_subscription_created",
    verify = "emits mcp_subscription_created when a client subscribes to delta notifications"
)]
fn event_mcp_subscription_created() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "graph");
    assert!(has_event(&server, "mcp_subscription_created"));
}

// E:mcp_subscription_removed — verify integration "emits mcp_subscription_removed when a client unsubscribes or server shuts down"
#[specforge_test(
    behavior = "mcp_subscription_removed",
    verify = "emits mcp_subscription_removed when a client unsubscribes or server shuts down"
)]
fn event_mcp_subscription_removed() {
    let mut server = init_server();
    subscriptions::subscribe(server.state_mut(), "client1", "graph");
    subscriptions::unsubscribe(server.state_mut(), "client1", "graph");
    assert!(has_event(&server, "mcp_subscription_removed"));
}
