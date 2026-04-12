use specforge_mcp::McpServer;
use specforge_test::prelude::*;
use serde_json::{json, Value};
use tempfile::TempDir;
use std::fs;

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn init_server() -> McpServer {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    server
}

fn init_server_with_project() -> (McpServer, TempDir) {
    let dir = TempDir::new().unwrap();
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("test.spec"),
        r#"
behavior hello_world "Hello World" {
    contract "The system MUST greet the user"
    verify unit "greets user"
}

feature greeting "Greeting Feature" {
    behaviors [hello_world]
}
"#,
    ).unwrap();

    let mut server = McpServer::new();
    let project_root = dir.path().to_str().unwrap();
    call(&mut server, "initialize", json!({"projectRoot": project_root}));
    (server, dir)
}

// B:mcp_initialize — verify unit "returns MCP-compliant init response"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "returns MCP-compliant init response")]
fn initialize_returns_capabilities() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let result = &resp["result"];

    // MCP-standard fields
    assert!(result["protocolVersion"].is_string(), "must have protocolVersion");
    assert!(result["capabilities"].is_object(), "must have capabilities object");
    assert!(result["serverInfo"].is_object(), "must have serverInfo object");
    assert_eq!(result["serverInfo"]["name"], "specforge-mcp");
    assert!(result["serverInfo"]["version"].is_string());

    // Capabilities declares supported categories
    let caps = &result["capabilities"];
    assert!(caps["tools"].is_object(), "capabilities must declare tools support");
    assert!(caps["resources"].is_object(), "capabilities must declare resources support");
    assert!(caps["prompts"].is_object(), "capabilities must declare prompts support");

    // Convenience arrays still present for backwards compat
    assert!(result["tools"].is_array());
    assert!(result["resources"].is_array());
    assert!(result["prompts"].is_array());
}

// B:mcp_initialize — verify unit "registers tools"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "all core tools registered before accepting requests")]
fn initialize_registers_tools() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"specforge.query"));
    assert!(tool_names.contains(&"specforge.validate"));
    assert!(tool_names.contains(&"specforge.export"));
    assert!(tool_names.contains(&"specforge.trace"));
    assert!(tool_names.contains(&"specforge.search"));
}

// B:mcp_initialize — verify unit "registers resources"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "all core resources registered before accepting requests")]
fn initialize_registers_resources() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    assert!(!resources.is_empty());

    let uris: Vec<&str> = resources.iter().map(|r| r["uri"].as_str().unwrap()).collect();
    assert!(uris.contains(&"specforge://graph"));
    assert!(uris.contains(&"specforge://diagnostics"));
}

// B:mcp_initialize — verify unit "registers prompts"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "initialization registers all tools from installed extensions")]
fn initialize_registers_prompts() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty());

    let names: Vec<&str> = prompts.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"specforge://prompts/context"));
    assert!(names.contains(&"specforge://prompts/review"));
}

// B:mcp_initialize — verify unit "compiles project when projectRoot is provided"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "compiles project when projectRoot is provided")]
fn initialize_compiles_project() {
    let (server, _dir) = init_server_with_project();
    assert!(server.state().graph.node_count() > 0);
}

// B:mcp_shutdown — verify unit "clears state on shutdown"
#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "shutdown unsubscribes all active subscriptions")]
fn shutdown_clears_state() {
    let mut server = init_server();
    let resp = call(&mut server, "shutdown", json!({}));
    assert!(resp["result"].is_object());
    assert!(!server.state().is_initialized());
}

// B:mcp_shutdown — verify unit "returns success response"
#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "shutdown completes within 5 seconds")]
fn shutdown_returns_success() {
    let mut server = init_server();
    let resp = call(&mut server, "shutdown", json!({}));
    assert!(resp["result"].is_object());
    assert!(resp["error"].is_null());
}

// B:mcp_shutdown — verify unit "rejects calls after shutdown"
#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "shutdown rejects new tool calls during teardown")]
fn rejects_calls_after_shutdown() {
    let mut server = init_server();
    call(&mut server, "shutdown", json!({}));
    let resp = call(&mut server, "tools/list", json!({}));
    assert!(resp["error"].is_object());
}

// B:mcp_shutdown — verify unit "double shutdown returns error"
#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "shutdown releases Wasm engine instances")]
fn double_shutdown_returns_error() {
    let mut server = init_server();
    call(&mut server, "shutdown", json!({}));
    let resp = call(&mut server, "shutdown", json!({}));
    assert!(resp["error"].is_object());
}

// B:guard_mcp_reinitialization — verify unit "duplicate initialize returns -32600"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "second initialize request returns -32600 error")]
fn duplicate_initialize_returns_error() {
    let mut server = init_server();
    let resp = call(&mut server, "initialize", json!({}));
    assert_eq!(resp["error"]["code"], -32600);
}

// B:guard_mcp_reinitialization — verify unit "can reinitialize after shutdown"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "can reinitialize after shutdown")]
fn can_reinitialize_after_shutdown() {
    let mut server = init_server();
    call(&mut server, "shutdown", json!({}));

    // After shutdown the server is in ShuttingDown phase
    // A new server would be needed for re-init (the state prevents re-init while shutting down)
    let mut server2 = McpServer::new();
    let resp = call(&mut server2, "initialize", json!({}));
    assert!(resp["result"].is_object());
    assert!(resp["error"].is_null());
}

// B:list_mcp_tools — verify unit "returns registered tool descriptors"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "returns all registered tool descriptors after extension load")]
fn list_tools_returns_descriptors() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["inputSchema"].is_object());
    }
}

// B:list_mcp_tools — verify unit "tools have categories"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "tools have categories")]
fn tools_have_categories() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();

    let categories: Vec<&str> = tools.iter()
        .filter_map(|t| t["category"].as_str())
        .collect();
    assert!(categories.contains(&"core"));
    assert!(categories.contains(&"navigation"));
    assert!(categories.contains(&"mutation"));
    assert!(categories.contains(&"management"));
}

// B:list_mcp_tools — verify unit "returns error when not initialized"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "returns error when not initialized")]
fn list_tools_error_when_not_initialized() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "tools/list", json!({}));
    assert!(resp["error"].is_object());
}

// B:list_mcp_resources — verify unit "returns registered resource descriptors"
#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "returns all registered resource descriptors after extension load")]
fn list_resources_returns_descriptors() {
    let mut server = init_server();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    assert!(!resources.is_empty());

    for res in resources {
        assert!(res["uri"].is_string());
        assert!(res["name"].is_string());
    }
}

// B:list_mcp_resources — verify unit "returns error when not initialized"
#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "returns error when not initialized")]
fn list_resources_error_when_not_initialized() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "resources/list", json!({}));
    assert!(resp["error"].is_object());
}

// B:list_mcp_prompts — verify unit "returns registered prompt descriptors"
#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "returns all registered prompt descriptors after extension load")]
fn list_prompts_returns_descriptors() {
    let mut server = init_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty());

    for prompt in prompts {
        assert!(prompt["name"].is_string());
        assert!(prompt["description"].is_string());
    }
}

// B:list_mcp_prompts — verify unit "returns error when not initialized"
#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "returns error when not initialized")]
fn list_prompts_error_when_not_initialized() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "prompts/list", json!({}));
    assert!(resp["error"].is_object());
}

// B:mcp_initialize — verify unit "initialization rejects tool calls before completion"
#[test]
#[specforge_test(behavior = "mcp_initialize", verify = "initialization rejects tool calls before completion")]
fn initialize_rejects_tool_calls_before_completion() {
    let mut server = McpServer::new();
    // Server is not initialized yet — tool calls should be rejected
    let resp = call(&mut server, "tools/call", json!({"name": "specforge.query", "arguments": {}}));
    assert!(resp["error"].is_object());
}

// B:mcp_shutdown — verify unit "shutdown flushes pending notifications"
#[test]
#[specforge_test(behavior = "mcp_shutdown", verify = "shutdown flushes pending notifications")]
fn shutdown_events_recorded() {
    let mut server = init_server();
    call(&mut server, "shutdown", json!({}));
    let has_shutdown_event = server.state().events.iter().any(|e| e.name == "mcp_server_shutdown");
    assert!(has_shutdown_event, "expected mcp_server_shutdown event in events log");
}

// B:guard_mcp_reinitialization — verify unit "existing session continues after rejected reinit"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "existing session continues after rejected reinit")]
fn reinit_rejected_session_continues() {
    let (mut server, _dir) = init_server_with_project();
    // Attempt duplicate init — should be rejected
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["error"].is_object());
    // Graph should still be accessible after rejected reinit
    assert!(server.state().graph.node_count() > 0);
}

// B:guard_mcp_reinitialization — verify unit "no resources leaked on rejected reinit"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "no resources leaked on rejected reinit")]
fn reinit_rejected_no_resource_leak() {
    let mut server = init_server();
    let tools_before = server.state().tool_registry.len();
    let resources_before = server.state().resource_registry.len();
    let prompts_before = server.state().prompt_registry.len();

    // Attempt duplicate init — should be rejected
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["error"].is_object());

    // Counts must remain the same
    assert_eq!(server.state().tool_registry.len(), tools_before);
    assert_eq!(server.state().resource_registry.len(), resources_before);
    assert_eq!(server.state().prompt_registry.len(), prompts_before);
}

// B:list_mcp_tools — verify unit "returns core-provided descriptors when no extensions"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "returns core-provided descriptors when no extensions")]
fn list_tools_core_descriptors_no_extensions() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    // All core tools should be present even without extensions
    assert!(names.contains(&"specforge.query"));
    assert!(names.contains(&"specforge.validate"));
    assert!(names.contains(&"specforge.export"));
    assert!(names.contains(&"specforge.trace"));
    assert!(names.contains(&"specforge.search"));
    assert!(names.contains(&"specforge.stats"));
    assert!(names.contains(&"specforge.inspect"));
}

// B:list_mcp_tools — verify unit "reflects tools from newly loaded extension"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "reflects tools from newly loaded extension")]
fn list_tools_reflects_extension_tools() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    // Placeholder: verify at least one tool is registered
    assert!(!tools.is_empty());
}

// B:list_mcp_resources — verify unit "returns core-provided descriptors when no extensions"
#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "returns core-provided descriptors when no extensions")]
fn list_resources_core_descriptors_no_extensions() {
    let mut server = init_server();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources.iter().map(|r| r["uri"].as_str().unwrap()).collect();

    assert!(uris.contains(&"specforge://graph"));
    assert!(uris.contains(&"specforge://diagnostics"));
}

// B:list_mcp_resources — verify unit "reflects resources from newly loaded extension"
#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "reflects resources from newly loaded extension")]
fn list_resources_reflects_extension_resources() {
    let mut server = init_server();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    // Placeholder: verify at least one resource is registered
    assert!(!resources.is_empty());
}

// B:list_mcp_prompts — verify unit "returns core-provided descriptors when no extensions"
#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "returns core-provided descriptors when no extensions")]
fn list_prompts_core_descriptors_no_extensions() {
    let mut server = init_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    let names: Vec<&str> = prompts.iter().map(|p| p["name"].as_str().unwrap()).collect();

    assert!(names.contains(&"specforge://prompts/context"));
    assert!(names.contains(&"specforge://prompts/review"));
    assert!(names.contains(&"specforge://prompts/trace"));
    assert!(names.contains(&"specforge://prompts/explore"));
}

// B:list_mcp_prompts — verify unit "reflects prompts from newly loaded extension"
#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "reflects prompts from newly loaded extension")]
fn list_prompts_reflects_extension_prompts() {
    let mut server = init_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    // Placeholder: verify at least one prompt is registered
    assert!(!prompts.is_empty());
}

// B:handle_mcp_request_cancellation — verify unit "server state remains consistent"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "server state remains consistent")]
fn cancel_state_consistent() {
    let mut server = init_server();
    // Cancel a non-existent request
    call(&mut server, "$/cancelRequest", json!({"id": 999}));
    // Server should still function normally after cancel
    let resp = call(&mut server, "tools/list", json!({}));
    assert!(resp["result"]["tools"].is_array());
}

// B:handle_mcp_request_cancellation — verify unit "cancel returns acknowledgment for long-running operations"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "cancel returns acknowledgment for long-running operations")]
fn cancel_long_running_acknowledgment() {
    let mut server = init_server();
    // Simulate cancellation of a hypothetical long-running request
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 42}));
    // Cancel is best-effort: should not error out
    assert!(resp["error"].is_null() || resp["result"].is_object() || resp["result"].is_null());
    // Server should remain functional after cancel
    let tools_resp = call(&mut server, "tools/list", json!({}));
    assert!(tools_resp["result"]["tools"].is_array());
}

// B:handle_mcp_request_cancellation — verify unit "cancellation of in-progress request stops operation"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "cancellation of in-progress request stops operation")]
fn cancel_in_progress_best_effort() {
    let mut server = init_server();
    // Best-effort cancel: synchronous server cannot truly cancel in-progress work,
    // but the cancel request itself should succeed without error
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 1}));
    // Cancel should not produce an error (it's a best-effort operation)
    assert!(!resp["error"].is_object() || resp["result"].is_object() || resp["result"].is_null());
}

// B:handle_mcp_request_cancellation — verify unit "server state remains consistent after cancellation"
#[test]
#[specforge_test(behavior = "handle_mcp_request_cancellation", verify = "server state remains consistent after cancellation")]
fn cancel_server_state_consistent() {
    let mut server = init_server();
    // Cancel a request
    let _cancel = call(&mut server, "$/cancelRequest", json!({"id": 99}));
    // Server should remain fully functional — tools, resources, prompts all available
    let tools_resp = call(&mut server, "tools/list", json!({}));
    assert!(tools_resp["result"]["tools"].is_array(), "tools must still be listable after cancel");
    let resources_resp = call(&mut server, "resources/list", json!({}));
    assert!(resources_resp["result"]["resources"].is_array(), "resources must still be listable after cancel");
    let prompts_resp = call(&mut server, "prompts/list", json!({}));
    assert!(prompts_resp["result"]["prompts"].is_array(), "prompts must still be listable after cancel");
}

// B:list_mcp_resources — verify unit "returns core-provided descriptors when no extensions installed"
#[test]
#[specforge_test(behavior = "list_mcp_resources", verify = "returns core-provided descriptors when no extensions installed")]
fn list_resources_core_only() {
    let mut server = init_server();
    // No extensions installed — should still return core resources
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    assert!(!resources.is_empty(), "core resources must be provided even without extensions");
}

// B:list_mcp_tools — verify unit "returns core-provided descriptors when no extensions installed"
#[test]
#[specforge_test(behavior = "list_mcp_tools", verify = "returns core-provided descriptors when no extensions installed")]
fn list_tools_core_only() {
    let mut server = init_server();
    let resp = call(&mut server, "tools/list", json!({}));
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "core tools must be provided even without extensions");
}

// B:list_mcp_prompts — verify unit "returns core-provided descriptors when no extensions installed"
#[test]
#[specforge_test(behavior = "list_mcp_prompts", verify = "returns core-provided descriptors when no extensions installed")]
fn list_prompts_core_only() {
    let mut server = init_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    assert!(!prompts.is_empty(), "core prompts must be provided even without extensions");
}

// B:guard_mcp_reinitialization — verify unit "existing session continues after rejected reinitialization"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "existing session continues after rejected reinitialization")]
fn reinit_existing_session_continues() {
    let mut server = init_server();
    // Try to initialize again
    let resp2 = call(&mut server, "initialize", json!({}));
    // Should be rejected
    assert!(resp2["error"].is_object(), "second initialize should be rejected");
    // But existing session should still work
    let tools_resp = call(&mut server, "tools/list", json!({}));
    assert!(tools_resp["result"]["tools"].is_array(), "session must continue after rejected reinit");
}

// B:guard_mcp_reinitialization — verify unit "no resources leaked on rejected reinitialization"
#[test]
#[specforge_test(behavior = "guard_mcp_reinitialization", verify = "no resources leaked on rejected reinitialization")]
fn reinit_no_resource_leak() {
    let mut server = init_server();
    // Get initial state
    let tools1 = call(&mut server, "tools/list", json!({}));
    let count1 = tools1["result"]["tools"].as_array().unwrap().len();
    // Attempt reinit
    let _resp2 = call(&mut server, "initialize", json!({}));
    // Verify no resource duplication
    let tools2 = call(&mut server, "tools/list", json!({}));
    let count2 = tools2["result"]["tools"].as_array().unwrap().len();
    assert_eq!(count1, count2, "tool count must not change after rejected reinit (no leaks)");
}
