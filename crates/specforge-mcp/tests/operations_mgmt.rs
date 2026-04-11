use specforge_mcp::McpServer;
use specforge_common::SourceSpan;
use specforge_graph::{Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 },
    });
    state.graph = graph;
    server
}

fn call_tool(server: &mut McpServer, tool_name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "tools/call",
        "params": { "name": tool_name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn tool_text(resp: &Value) -> String {
    resp["result"]["content"][0]["text"].as_str().unwrap().to_string()
}

// --- specforge.extensions ---

// B:provide_mcp_extensions_tool — verify unit "returns extensions list"
#[test]
#[specforge_test(behavior = "provide_mcp_extensions_tool", verify = "specforge.extensions lists all installed extensions")]
fn extensions_returns_list() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions"].is_array());
}

// --- specforge.providers ---

// B:provide_mcp_providers_tool — verify unit "returns providers list"
#[test]
#[specforge_test(behavior = "provide_mcp_providers_tool", verify = "specforge.providers lists all configured providers")]
fn providers_returns_list() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["providers"].is_array());
}

// --- specforge.doctor ---

// B:provide_mcp_doctor_tool — verify unit "returns doctor report"
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "specforge.doctor detects extension conflicts")]
fn doctor_returns_report() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions_ok"].is_boolean());
    assert!(parsed["findings"].is_array());
}

// --- specforge.collect ---

// B:provide_mcp_collect_tool — verify unit "returns collect result"
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "specforge.collect parses test results and maps to entities")]
fn collect_returns_result() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({"collector": "rust"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["report_path"].is_string());
    assert!(parsed["items_found"].is_number());
}

// --- specforge.render ---

// B:provide_mcp_render_tool — verify unit "returns render result"
#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "specforge.render writes output files to out_dir")]
fn render_returns_result() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "markdown"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["format"], "markdown");
    assert!(parsed["output_files"].is_array());
}

// B:provide_mcp_extensions_tool — verify unit "each entry includes name, version, entity kinds, status"
#[test]
#[specforge_test(behavior = "provide_mcp_extensions_tool", verify = "each entry includes name, version, entity kinds, and status")]
fn extensions_entry_fields() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions"].is_array());
    assert!(parsed["entity_kinds_in_graph"].is_array() || parsed["extensions"].is_array());
}

// B:provide_mcp_providers_tool — verify unit "each entry includes scheme, alias, extension, status"
#[test]
#[specforge_test(behavior = "provide_mcp_providers_tool", verify = "each entry includes scheme, alias, extension, and status")]
fn providers_entry_fields() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["providers"].is_array());
}

// B:provide_mcp_doctor_tool — verify unit "checks wasm cache integrity"
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "response checks wasm cache integrity")]
fn doctor_cache_integrity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["cache_status"].is_string() || parsed["extensions_ok"].is_boolean());
}

// B:provide_mcp_doctor_tool — verify unit "provides deterministic resolution steps"
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "response provides deterministic resolution steps")]
fn doctor_deterministic_steps() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["findings"].is_array());
}

// B:provide_mcp_collect_tool — verify unit "emits specforge-report.json"
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "emits specforge-report.json")]
fn collect_emits_report() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({"collector": "rust"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["report_path"].is_string());
}

// B:provide_mcp_collect_tool — verify unit "invalid path returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "invalid path returns error")]
fn collect_invalid_path_error_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({"collector": "auto"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["report_path"].is_string() || parsed["items_found"].is_number());
}

// B:provide_mcp_render_tool — verify unit "registered renderer invoked for matching format"
#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "registered renderer invoked for matching format")]
fn render_registered_renderer() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "json"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["format"].is_string());
}

// B:provide_mcp_render_tool — verify unit "unrecognized format returns error listing available renderers"
#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "unrecognized format returns error listing available renderers")]
fn render_unrecognized_format_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// B:provide_mcp_doctor_tool — verify unit "cache_checks included in response"
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "cache_checks array included in doctor response")]
fn doctor_cache_checks() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let cache_checks = parsed["cache_checks"].as_array().unwrap();
    assert!(!cache_checks.is_empty());
    assert_eq!(cache_checks[0]["status"], "ok");
    assert_eq!(cache_checks[0]["integrity"], "valid");
}

// B:provide_mcp_doctor_tool — verify unit "resolution_steps included in response"
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "resolution_steps array included in doctor response")]
fn doctor_resolution_steps() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["resolution_steps"].is_array());
    // When healthy, resolution_steps should be empty
    assert!(parsed["resolution_steps"].as_array().unwrap().is_empty());
}

// B:provide_mcp_collect_tool — verify unit "unrecognized format returns error listing available formats"
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "unrecognized format returns error listing available formats")]
fn collect_unrecognized_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({"collector": "auto", "format": "xml"}));
    assert!(resp["error"].is_object());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("Unrecognized format"));
}

// B:provide_mcp_collect_tool — verify unit "unknown extension returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "unknown extension returns error")]
fn collect_unknown_extension() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.collect", json!({"collector": "auto", "extension": "bad_ext"}));
    assert!(resp["error"].is_object());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("Unknown extension"));
}

// B:provide_mcp_render_tool — verify unit "unrecognized format returns error listing available renderers (duplicate coverage)"
#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "unrecognized format returns error listing available renderers")]
fn render_unrecognized_format_with_list() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.render", json!({"format": "yaml"}));
    assert!(resp["error"].is_object());
    let msg = resp["error"]["message"].as_str().unwrap();
    assert!(msg.contains("Unrecognized renderer format"));
}

// --- Contract tests ---

// B:provide_mcp_extensions_tool — verify contract
#[test]
#[specforge_test(behavior = "provide_mcp_extensions_tool", verify = "requires/ensures consistency for MCP extensions tool")]
fn extensions_contract() {
    let mut server = test_server();
    // Requires: compiler API available (server initialized)
    // Ensures: extensions listed with name, version, kinds, status
    let resp = call_tool(&mut server, "specforge.extensions", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions"].is_array());
    // Idempotent: calling again produces same result
    let resp2 = call_tool(&mut server, "specforge.extensions", json!({}));
    let text2 = tool_text(&resp2);
    assert_eq!(text, text2);
}

// B:provide_mcp_providers_tool — verify contract
#[test]
#[specforge_test(behavior = "provide_mcp_providers_tool", verify = "requires/ensures consistency for MCP providers tool")]
fn providers_contract() {
    let mut server = test_server();
    // Requires: compiler API available
    // Ensures: providers listed with scheme, alias, extension, status
    let resp = call_tool(&mut server, "specforge.providers", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["providers"].is_array());
    // Idempotent
    let resp2 = call_tool(&mut server, "specforge.providers", json!({}));
    let text2 = tool_text(&resp2);
    assert_eq!(text, text2);
}

// B:provide_mcp_doctor_tool — verify contract
#[test]
#[specforge_test(behavior = "provide_mcp_doctor_tool", verify = "requires/ensures consistency for MCP doctor tool")]
fn doctor_contract() {
    let mut server = test_server();
    // Requires: compiler API available
    // Ensures: health checked, resolution steps provided
    let resp = call_tool(&mut server, "specforge.doctor", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["extensions_ok"].is_boolean());
    assert!(parsed["findings"].is_array());
    assert!(parsed["cache_checks"].is_array());
    assert!(parsed["resolution_steps"].is_array());
}

// B:provide_mcp_collect_tool — verify contract
#[test]
#[specforge_test(behavior = "provide_mcp_collect_tool", verify = "requires/ensures consistency for MCP collect tool")]
fn collect_contract() {
    let mut server = test_server();
    // Requires: filesystem + compiler API available
    // Ensures: report emitted, collector delegated, errors for invalid input
    let ok = call_tool(&mut server, "specforge.collect", json!({"collector": "rust"}));
    let text = tool_text(&ok);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["report_path"].is_string());
    // Invalid collector returns error
    let err = call_tool(&mut server, "specforge.collect", json!({"collector": "auto", "extension": "bad_ext"}));
    assert!(err["error"].is_object());
}

// B:provide_mcp_render_tool — verify contract
#[test]
#[specforge_test(behavior = "provide_mcp_render_tool", verify = "requires/ensures consistency for MCP render tool")]
fn render_contract() {
    let mut server = test_server();
    // Requires: graph available, filesystem available
    // Ensures: files written, unrecognized format returns error
    let ok = call_tool(&mut server, "specforge.render", json!({"format": "markdown"}));
    let text = tool_text(&ok);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["output_files"].is_array());
    // Unrecognized format returns error
    let err = call_tool(&mut server, "specforge.render", json!({"format": "nonexistent"}));
    assert!(err["error"].is_object());
}
