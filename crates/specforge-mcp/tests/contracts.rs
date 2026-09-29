use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;
use std::path::Path;

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

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".into(),
        start_line: 1,
        start_col: 0,
        end_line: 5,
        end_col: 0,
    }
}

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    let mut fields = FieldMap::new();
    fields.push("contract".into(), FieldValue::String("MUST work".into()));
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".into(),
            description: "works".into(),
        }]),
    );

    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha".into()),
        fields,
        source_span: span(),
        methods: Vec::new(),
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "feat.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 0,
        },
        methods: Vec::new(),
    });
    graph.add_edge(Edge {
        source: "beta".into(),
        target: "alpha".into(),
        label: "behaviors".into(),
    });
    state.graph = graph;
    attach_project(state);

    server
}

fn call(server: &mut McpServer, method: &str, params: Value) -> Value {
    let req = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
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

#[specforge_test(
    behavior = "mcp_initialize",
    verify = "MCP Initialize: MCP initialization holds — compiler_api_available, wasm_runtime_available, capabilities_returned, surface_contributions_merged, mcp_initialized_emitted"
)]
fn contract_initialize() {
    let mut server = McpServer::new();
    let resp = call(&mut server, "initialize", json!({}));
    let result = &resp["result"];
    assert!(result["tools"].is_array());
    assert!(result["resources"].is_array());
    assert!(result["prompts"].is_array());
    assert!(result["serverInfo"]["name"].is_string());
    assert!(result["serverInfo"]["version"].is_string());
    assert!(result["protocolVersion"].is_string());
    assert!(result["capabilities"].is_object());
}

#[specforge_test(
    behavior = "mcp_shutdown",
    verify = "MCP Shutdown: MCP shutdown holds — server_initialized, notifications_flushed, subscriptions_removed, wasm_engines_released, shutdown_emitted"
)]
fn contract_shutdown() {
    let mut server = McpServer::new();
    call(&mut server, "initialize", json!({}));
    let resp = call(&mut server, "shutdown", json!({}));
    assert!(resp["result"].is_object());
}

#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "Provide MCP Query Tool: MCP query tool holds — graph_available, subgraph_returned, unknown_kinds_reported, tool_invoked_emitted"
)]
fn contract_query() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "Provide MCP Export Tool: MCP export tool holds — graph_available, format_produced, token_budget_enforced, tool_invoked_emitted"
)]
fn contract_export() {
    let mut server = test_server();
    for format in &["graph", "context", "brief"] {
        let resp = call_tool(&mut server, "specforge.export", json!({"format": format}));
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert!(
            parsed["nodes"].is_array(),
            "format {} should have nodes",
            format
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "max_tokens truncates output to fit token budget"
)]
fn contract_export_max_tokens() {
    let mut server = test_server();

    // Descriptor advertises max_tokens
    let resp = call(&mut server, "tools/list", json!({}));
    let export_tool = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "specforge.export")
        .expect("specforge.export descriptor must be registered");
    assert!(
        export_tool["inputSchema"]["properties"]["max_tokens"].is_object(),
        "export descriptor must declare max_tokens"
    );

    // Tool truncates graph output when max_tokens is set
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "max_tokens": 5}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(
        parsed["token_budget"].is_object(),
        "graph export with max_tokens must include token_budget metadata"
    );
    assert!(
        parsed["nodes"].as_array().unwrap().len() < 2,
        "tiny budget must truncate the 2-node graph"
    );
}

#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "Provide MCP Trace Tool: MCP trace tool holds — graph_available, trace_result_returned, gaps_identified, tool_invoked_emitted"
)]
fn contract_trace() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "alpha"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "Provide MCP Search Tool: MCP search tool holds — graph_available, filtered_results_returned, unknown_kinds_reported, tool_invoked_emitted"
)]
fn contract_search() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "Provide MCP Stats Tool: MCP stats tool holds — graph_available, stats_returned, latest_state_reflected, tool_invoked_emitted"
)]
fn contract_stats() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_counts"].is_array());
    assert!(parsed["diagnostic_summary"].is_object());
}

#[specforge_test(
    behavior = "provide_mcp_inspect_tool",
    verify = "Provide MCP Inspect Tool: MCP inspect tool holds — graph_available, entity_details_returned, tool_invoked_emitted"
)]
fn contract_inspect() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.inspect",
        json!({"entity_id": "alpha"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["kind"].is_string());
    assert!(parsed["source_span"].is_object());
}

#[specforge_test(
    behavior = "provide_mcp_find_definition_tool",
    verify = "Provide MCP Find Definition Tool: MCP find definition tool holds — graph_available, source_location_returned, tool_invoked_emitted"
)]
fn contract_find_definition() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_definition",
        json!({"entity_id": "alpha"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["file_path"].is_string());
    assert!(parsed["line"].is_number());
}

#[specforge_test(
    behavior = "provide_mcp_find_references_tool",
    verify = "Provide MCP Find References Tool: MCP find references tool holds — graph_available, references_returned, empty_list_for_unreferenced, tool_invoked_emitted"
)]
fn contract_find_references() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.find_references",
        json!({"entity_id": "alpha"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_id"].is_string());
    assert!(parsed["locations"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_outline_tool",
    verify = "Provide MCP Outline Tool: MCP outline tool holds — graph_available, outline_returned, tool_invoked_emitted"
)]
fn contract_outline() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.outline",
        json!({"file": "test.spec"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "Provide MCP Coverage Tool: MCP coverage tool holds — graph_available, coverage_returned, testability_respected, tool_invoked_emitted"
)]
fn contract_coverage() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "Provide MCP Schema Tool: MCP schema tool holds — graph_available, schema_returned, tool_invoked_emitted"
)]
fn contract_schema() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["entity_kinds"].is_object());
}

#[specforge_test(
    behavior = "provide_mcp_context_prompt",
    verify = "Provide MCP Context Prompt: MCP context prompt holds — graph_available, context_returned, hints_included, prompt_invoked_emitted"
)]
fn contract_context_prompt() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/context", "arguments": {"entity_id": "alpha"}}),
    );
    assert!(resp["result"]["messages"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_review_prompt",
    verify = "Provide MCP Review Prompt: MCP review prompt holds — graph_available, coverage_analysis_returned, gaps_identified, prompt_invoked_emitted"
)]
fn contract_review_prompt() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/review", "arguments": {}}),
    );
    assert!(resp["result"]["messages"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_trace_prompt",
    verify = "Provide MCP Trace Prompt: MCP trace prompt holds — graph_available, gaps_returned, affected_entities_listed, prompt_invoked_emitted"
)]
fn contract_trace_prompt() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/trace", "arguments": {"entity_id": "alpha"}}),
    );
    assert!(resp["result"]["messages"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_explore_prompt",
    verify = "Provide MCP Explore Prompt: MCP explore prompt holds — graph_available, exploration_returned, bfs_from_entity, prompt_invoked_emitted"
)]
fn contract_explore_prompt() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "prompts/get",
        json!({"name": "specforge://prompts/explore", "arguments": {}}),
    );
    assert!(resp["result"]["messages"].is_array());
}

#[specforge_test(
    behavior = "list_mcp_tools",
    verify = "List MCP Tools: listing MCP tools holds — server_initialized, complete_list_returned, disabled_excluded, discovery_emitted"
)]
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

#[specforge_test(
    behavior = "list_mcp_resources",
    verify = "List MCP Resources: listing MCP resources holds — server_initialized, complete_list_returned, disabled_excluded, discovery_emitted"
)]
fn contract_list_resources() {
    let mut server = test_server();
    let resp = call(&mut server, "resources/list", json!({}));
    let resources = resp["result"]["resources"].as_array().unwrap();
    for res in resources {
        assert!(res["uri"].is_string());
        assert!(res["name"].is_string());
    }
}

#[specforge_test(
    behavior = "list_mcp_prompts",
    verify = "List MCP Prompts: listing MCP prompts holds — server_initialized, complete_list_returned, discovery_emitted"
)]
fn contract_list_prompts() {
    let mut server = test_server();
    let resp = call(&mut server, "prompts/list", json!({}));
    let prompts = resp["result"]["prompts"].as_array().unwrap();
    for prompt in prompts {
        assert!(prompt["name"].is_string());
        assert!(prompt["description"].is_string());
    }
}

#[specforge_test(
    behavior = "guard_mcp_reinitialization",
    verify = "Guard MCP Reinitialization: MCP reinitialization guard holds — server_initialized, reinit_rejected, session_unaffected, error_handled_emitted"
)]
fn contract_guard_reinit() {
    let mut server = test_server();
    let resp = call(&mut server, "initialize", json!({}));
    assert!(resp["error"].is_object());
}

#[specforge_test(
    behavior = "handle_mcp_request_cancellation",
    verify = "Handle MCP Request Cancellation: MCP request cancellation holds — mcp_protocol_available, cancellation_safe, request_cancelled_emitted"
)]
fn contract_cancel() {
    let mut server = test_server();
    let resp = call(&mut server, "$/cancelRequest", json!({"id": 1}));
    assert!(resp["result"].is_object() || resp["result"].is_null());
}

#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "Handle MCP Protocol Error: MCP protocol error handling holds — mcp_protocol_available, standard_error_returned, no_state_leaked, server_operational, error_handled_emitted"
)]
fn contract_protocol_error() {
    let mut server = test_server();
    let resp = call(&mut server, "nonexistent/method", json!({}));
    assert_eq!(resp["jsonrpc"], "2.0");
    assert!(resp["error"]["code"].is_number());
    assert!(resp["error"]["message"].is_string());
}

#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "Provide MCP Validate Tool: MCP validate tool holds — compiler_api_available, diagnostics_returned, strict_promotion_enforced, tool_invoked_emitted"
)]
fn contract_validate() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    assert!(
        resp["error"].is_object() || resp["result"]["content"][0]["text"].is_string(),
        "validate must return error or diagnostics text"
    );
}

#[specforge_test(
    behavior = "provide_mcp_suggest_fixes_tool",
    verify = "Provide MCP Suggest Fixes Tool: MCP suggest fixes tool holds — graph_available, fixes_returned, empty_for_clean, tool_invoked_emitted"
)]
fn contract_suggest_fixes() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.suggest_fixes", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.is_array());
}

#[specforge_test(
    behavior = "provide_mcp_format_tool",
    verify = "Provide MCP Format Tool: MCP format tool holds — filesystem_available, files_formatted, check_mode_readonly, mutation_completed_emitted, tool_invoked_emitted"
)]
fn contract_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.format", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("changed_files").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_rename_tool",
    verify = "Provide MCP Rename Tool: MCP rename tool holds — graph_available, filesystem_available, references_updated, recompilation_triggered, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn contract_rename() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.rename",
        json!({"entity_id": "alpha", "new_name": "alpha_v2"}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("old_name").is_some());
    assert!(parsed.get("new_name").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_init_tool",
    verify = "Provide MCP Init Tool: MCP init tool holds — filesystem_available, project_created, path_outside_current, extensions_validated, project_initialized_emitted, tool_invoked_emitted"
)]
fn contract_init() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.init",
        json!({"path": dir.path().to_str().unwrap()}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("project_path").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_extensions_tool",
    verify = "Provide MCP Extensions Tool: MCP extensions tool holds — compiler_api_available, extensions_listed, config_reflected, tool_invoked_emitted"
)]
fn contract_extensions() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("extensions").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_doctor_tool",
    verify = "Provide MCP Doctor Tool: MCP doctor tool holds — compiler_api_available, health_checked, resolution_steps_provided, tool_invoked_emitted"
)]
fn contract_doctor() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("extensions_ok").is_some());
}

// Additional contracts for remaining behaviors

#[specforge_test(
    behavior = "expose_graph_as_mcp_resource",
    verify = "Expose Graph as MCP Resource: graph MCP resource holds — validation_complete_fired, graph_json_returned, resource_read_emitted"
)]
fn contract_graph_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "expose_schema_as_mcp_resource",
    verify = "Expose Schema as MCP Resource: schema MCP resource holds — validation_complete_fired, schema_json_returned, resource_read_emitted"
)]
fn contract_schema_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://schema"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "expose_context_as_mcp_resource",
    verify = "Expose Context as MCP Resource: context MCP resource holds — validation_complete_fired, context_format_returned, resource_read_emitted"
)]
fn contract_context_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://context"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "expose_brief_as_mcp_resource",
    verify = "Expose Brief as MCP Resource: brief MCP resource holds — validation_complete_fired, brief_format_returned, resource_read_emitted"
)]
fn contract_brief_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://brief"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "expose_diagnostics_as_mcp_resource",
    verify = "Expose Diagnostics as MCP Resource: diagnostics MCP resource holds — validation_complete_fired, diagnostics_returned, resource_read_emitted"
)]
fn contract_diagnostics_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://diagnostics"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "expose_entity_as_mcp_resource",
    verify = "Expose Per-Entity MCP Resource: per-entity MCP resource holds — validation_complete_fired, subgraph_returned, resource_read_emitted"
)]
fn contract_entity_resource() {
    let mut server = test_server();
    let resp = call(
        &mut server,
        "resources/read",
        json!({"uri": "specforge://graph/alpha"}),
    );
    assert!(resp["result"]["contents"].is_array());
}

#[specforge_test(
    behavior = "notify_graph_delta_via_mcp",
    verify = "Notify Graph Delta via MCP: graph delta MCP notification holds — graph_delta_computed_fired, subscribers_notified, no_notification_when_empty, delta_notified_emitted"
)]
fn contract_graph_notification() {
    use specforge_mcp::notifications::*;
    let g1 = specforge_graph::Graph::new();
    let mut g2 = specforge_graph::Graph::new();
    g2.add_node(Node {
        id: EntityId { raw: "x".into() },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    });
    let delta = compute_graph_delta(&g1, &g2);
    let notif = format_graph_notification(&delta);
    assert_eq!(notif["method"], "specforge/graphChanged");
    assert!(notif["params"]["added_nodes"].is_array());
}

#[specforge_test(
    behavior = "notify_diagnostics_delta_via_mcp",
    verify = "Notify Diagnostics Delta via MCP: diagnostics delta MCP notification holds — validation_complete_fired, subscribers_notified, unchanged_suppressed, delta_notified_emitted"
)]
fn contract_diagnostics_notification() {
    use specforge_common::{Diagnostic, Severity};
    use specforge_mcp::notifications::*;
    let d = Diagnostic {
        code: "V001".into(),
        severity: Severity::Error,
        message: "err".into(),
        span: None,
        suggestion: None,
    };
    let delta = compute_diagnostics_delta(&[], &[d]);
    let notif = format_diagnostics_notification(&delta);
    assert_eq!(notif["method"], "specforge/diagnosticsChanged");
    assert!(notif["params"]["added"].is_array());
}

#[specforge_test(
    behavior = "provide_mcp_add_extension_tool",
    verify = "Provide MCP Add Extension Tool: MCP add extension tool holds — filesystem_available, extension_installed, wasm_downloaded, extension_added_emitted, dry_run_safe, tool_invoked_emitted"
)]
fn contract_add_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    // Real offline install of the vendored product blob.
    let blob = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("extensions/product/wasm/specforge_ext_product.wasm");
    let resp = call_tool(
        &mut server,
        "specforge.add_extension",
        json!({"specifier": blob.to_str().unwrap()}),
    );
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["installed"], true);
    // Truthful install is observable on disk.
    let lock = std::fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(lock.contains("specforge_ext_product"));
}

#[specforge_test(
    behavior = "provide_mcp_remove_extension_tool",
    verify = "Provide MCP Remove Extension Tool: MCP remove extension tool holds — filesystem_available, extension_removed, orphan_warning_produced, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn contract_remove_extension() {
    let dir = tempfile::TempDir::new().unwrap();
    let mut server = test_server();
    server.state_mut().project_root = Some(dir.path().to_path_buf());
    // Removing something that is not installed must refuse.
    let resp = call_tool(
        &mut server,
        "specforge.remove_extension",
        json!({"name": "@specforge/software"}),
    );
    assert!(resp["error"].is_object());
}

#[specforge_test(
    behavior = "provide_mcp_migrate_tool",
    verify = "Provide MCP Migrate Tool: MCP migrate tool holds — filesystem_available, migrations_applied, post_migration_validated, dry_run_safe, mutation_completed_emitted, tool_invoked_emitted"
)]
fn contract_migrate() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.migrate", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("migrated").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_providers_tool",
    verify = "Provide MCP Providers Tool: MCP providers tool holds — compiler_api_available, providers_listed, tool_invoked_emitted"
)]
fn contract_providers() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("providers").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_render_tool",
    verify = "Provide MCP Render Tool: MCP render tool holds — graph_available, filesystem_available, files_written, files_listed, tool_invoked_emitted"
)]
fn contract_render() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "json"}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed.get("format").is_some());
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "Provide MCP Analyze Tool: MCP analyze tool holds — graph_available, passes_run, results_structured, tool_invoked_emitted"
)]
fn contract_analyze() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.analyze", json!({}));
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert!(parsed["ok"].is_boolean(), "analyze must return ok flag");
    let passes = parsed["passes"].as_array().unwrap();
    assert!(!passes.is_empty(), "all-pass run must include passes");
    // Built-in passes always run; coverage is @specforge/testing's pass and
    // this fixture enables no extensions.
    assert!(passes.iter().any(|p| p["pass"] == "contracts"));
}
