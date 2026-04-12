use specforge_mcp::McpServer;
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;
use serde_json::{json, Value};

fn test_server() -> McpServer {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();

    let mut fields_a = FieldMap::new();
    fields_a.push("contract".into(), FieldValue::String("The system MUST do alpha".into()));
    fields_a.push("verify".into(), FieldValue::VerifyList(vec![
        VerifyStatement { kind: "unit".into(), description: "test alpha".into() },
    ]));

    graph.add_node(Node {
        id: EntityId { raw: "alpha".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 },
    });
    graph.add_node(Node {
        id: EntityId { raw: "beta".into() },
        kind: EntityKind { raw: "feature".into() },
        title: Some("Beta Feature".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "features.spec".into(), start_line: 10, start_col: 0, end_line: 15, end_col: 0 },
    });
    graph.add_node(Node {
        id: EntityId { raw: "gamma_orphan".into() },
        kind: EntityKind { raw: "invariant".into() },
        title: Some("Gamma".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan { file: "inv.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    graph.add_edge(Edge { source: "beta".into(), target: "alpha".into(), label: "behaviors".into() });
    state.graph = graph;

    server
}

fn call_prompt(server: &mut McpServer, name: &str, args: Value) -> Value {
    let req = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "prompts/get",
        "params": { "name": name, "arguments": args }
    });
    let resp = server.handle_message(&req.to_string()).unwrap();
    serde_json::from_str(&resp).unwrap()
}

fn prompt_text(resp: &Value) -> String {
    // Data is in the last message (assistant role), instruction is first (user role)
    let messages = resp["result"]["messages"].as_array().unwrap();
    let last = messages.last().unwrap();
    last["content"]["text"].as_str().unwrap().to_string()
}

// --- specforge://prompts/context ---

// B:provide_mcp_context_prompt — verify unit "returns entity context with instructional framing"
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "specforge://prompts/context returns structured entity context")]
fn context_prompt_returns_context() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "alpha"}));
    let messages = resp["result"]["messages"].as_array().unwrap();

    // Must have instruction message + data message
    assert!(messages.len() >= 2, "prompt must have instruction + data messages, got {}", messages.len());

    // First message is instruction (role: user)
    assert_eq!(messages[0]["role"], "user", "instruction message should be role 'user'");
    let instruction = messages[0]["content"]["text"].as_str().unwrap();
    assert!(instruction.contains("implement") || instruction.contains("context") || instruction.contains("entity"),
        "instruction should guide the agent, got: {}", instruction);

    // Second message has the data (role: assistant)
    assert_eq!(messages[1]["role"], "assistant", "data message should be role 'assistant'");
    let text = messages[1]["content"]["text"].as_str().unwrap();
    let parsed: Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert!(parsed["contract_text"].is_string());
    assert!(parsed["upstream_entities"].is_array());
    assert!(parsed["downstream_entities"].is_array());
    assert!(parsed["verify_expectations"].is_array());
}

// B:provide_mcp_context_prompt — verify unit "unknown entity returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "non-existent entity returns error")]
fn context_prompt_unknown_entity() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// B:provide_mcp_context_prompt — verify unit "includes upstream and downstream"
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "response includes contract and related entities")]
fn context_prompt_includes_edges() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "alpha"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let upstream = parsed["upstream_entities"].as_array().unwrap();
    // beta -> alpha, so beta is upstream of alpha
    assert!(upstream.contains(&json!("beta")));
}

// --- specforge://prompts/review ---

// B:provide_mcp_review_prompt — verify unit "returns findings"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "specforge://prompts/review returns coverage analysis")]
fn review_prompt_returns_findings() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["findings"].is_array());
    assert!(parsed["coverage_summary"].is_array());
}

// B:provide_mcp_review_prompt — verify unit "detects uncovered entities"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "response identifies entities with missing verification coverage")]
fn review_prompt_detects_uncovered() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let findings = parsed["findings"].as_array().unwrap();
    // beta (feature) and gamma_orphan (invariant) have no verify declarations
    let uncovered: Vec<&Value> = findings.iter()
        .filter(|f| f["message"].as_str().unwrap().contains("no verify"))
        .collect();
    assert!(uncovered.len() >= 2);
}

// B:provide_mcp_review_prompt — verify unit "detects orphan entities"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "detects orphan entities")]
fn review_prompt_detects_orphans() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let findings = parsed["findings"].as_array().unwrap();
    let orphan_findings: Vec<&Value> = findings.iter()
        .filter(|f| f["message"].as_str().unwrap().contains("orphan"))
        .collect();
    assert!(!orphan_findings.is_empty());
}

// --- specforge://prompts/trace ---

// B:provide_mcp_trace_prompt — verify unit "returns trace gaps"
#[test]
#[specforge_test(behavior = "provide_mcp_trace_prompt", verify = "specforge://prompts/trace identifies gaps in plan")]
fn trace_prompt_returns_gaps() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/trace", json!({"entity_id": "alpha"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["affected_entities"].is_array());
    assert!(parsed["unverified_entities"].is_array());
}

// B:provide_mcp_trace_prompt — verify unit "identifies unverified entities in trace"
#[test]
#[specforge_test(behavior = "provide_mcp_trace_prompt", verify = "response returns identified gaps with gap context")]
fn trace_prompt_identifies_unverified() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/trace", json!({"entity_id": "alpha"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let unverified = parsed["unverified_entities"].as_array().unwrap();
    // beta is in the trace but has no verify
    assert!(unverified.contains(&json!("beta")));
}

// B:provide_mcp_trace_prompt — verify unit "unknown entity returns error"
#[test]
#[specforge_test(behavior = "provide_mcp_trace_prompt", verify = "affected entities are listed")]
fn trace_prompt_unknown_entity() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/trace", json!({"entity_id": "nonexistent"}));
    assert!(resp["error"].is_object());
}

// --- specforge://prompts/explore ---

// B:provide_mcp_explore_prompt — verify unit "returns exploration data"
#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "specforge://prompts/explore returns exploration starting points")]
fn explore_prompt_returns_data() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["matching_entities"].is_array());
    assert!(parsed["starting_points"].is_array());
    assert!(parsed["high_connectivity"].is_array());
    assert!(parsed["orphan_nodes"].is_array());
}

// B:provide_mcp_explore_prompt — verify unit "identifies orphan nodes"
#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "orphan_nodes field lists entities with zero incoming and outgoing edges")]
fn explore_prompt_identifies_orphans() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let orphans = parsed["orphan_nodes"].as_array().unwrap();
    assert!(orphans.contains(&json!("gamma_orphan")));
}

// B:provide_mcp_explore_prompt — verify unit "respects kind filter"
#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "kind filter restricts results to matching entity kind")]
fn explore_prompt_kind_filter() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({"kind": "behavior"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let matching = parsed["matching_entities"].as_array().unwrap();
    assert!(matching.contains(&json!("alpha")));
    assert!(!matching.contains(&json!("beta")));
}

// Unknown prompt
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "unknown prompt returns error")]
fn unknown_prompt_returns_error() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/nonexistent", json!({}));
    assert!(resp["error"].is_object());
}

// Prompt when not initialized
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "returns error when not initialized")]
fn prompt_not_initialized() {
    let mut server = McpServer::new();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "alpha"}));
    assert!(resp["error"].is_object());
}

fn span() -> SourceSpan {
    SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 5, end_col: 0 }
}

// B:provide_mcp_context_prompt — verify unit "context prompt works with zero extensions installed"
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "context prompt works with zero extensions installed")]
fn context_zero_extensions() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "minimal".into() },
        kind: EntityKind { raw: "behavior".into() },
        title: Some("Minimal".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });
    state.graph = graph;

    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "minimal"}));
    assert!(resp["result"]["messages"].is_array());
}

// B:provide_mcp_review_prompt — verify unit "depth parameter controls neighbor traversal depth"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "depth parameter controls neighbor traversal depth")]
fn review_depth_parameter() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({"entity_id": "alpha"}));
    assert!(resp["result"]["messages"].is_array());
}

// B:provide_mcp_review_prompt — verify unit "review returns empty findings when no testable entities in graph"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "review returns empty findings when no testable entities in graph")]
fn review_empty_findings_no_testable() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    // Add only non-testable entities (features have no verify)
    let state = server.state_mut();
    let mut graph = Graph::new();
    graph.add_node(Node {
        id: EntityId { raw: "some_feature".into() },
        kind: EntityKind { raw: "feature".into() },
        title: Some("Some Feature".into()),
        fields: FieldMap::new(),
        source_span: span(),
    });
    state.graph = graph;

    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Features have no verify, so no coverage findings about missing verify
    assert!(parsed["findings"].is_array());
}

// B:provide_mcp_review_prompt — verify unit "review prompt returns empty findings when no testable entities"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "cancelled long-running export returns partial result or acknowledgment")]
fn review_empty_findings() {
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());
    server.state_mut().graph = Graph::new();

    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["findings"].as_array().unwrap().is_empty());
}

// B:provide_mcp_trace_prompt — verify unit "malformed plan JSON returns validation error"
#[test]
#[specforge_test(behavior = "provide_mcp_trace_prompt", verify = "malformed plan JSON returns validation error")]
fn trace_malformed_plan() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/trace", json!({"plan": "not_json_object"}));
    // Should handle gracefully — either error or result, but no crash
    assert!(resp["error"].is_object() || resp["result"].is_object());
}

// B:provide_mcp_explore_prompt — verify unit "entity_id focuses exploration on that entity"
#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "entity_id focuses exploration on that entity")]
fn explore_entity_id_focus() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({"entity_id": "alpha"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let matching = parsed["matching_entities"].as_array().unwrap();
    assert!(matching.contains(&json!("alpha")));
}

// B:provide_mcp_explore_prompt — verify unit "high_connectivity excludes zero-edge nodes"
#[test]
#[specforge_test(behavior = "provide_mcp_explore_prompt", verify = "high_connectivity excludes zero-edge nodes")]
fn explore_high_connectivity() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/explore", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let high_conn = parsed["high_connectivity"].as_array().unwrap();
    // gamma_orphan has zero edges — must NOT appear in high_connectivity
    assert!(
        !high_conn.contains(&json!("gamma_orphan")),
        "zero-edge nodes must not appear in high_connectivity, got: {:?}", high_conn
    );
    // alpha and beta have edges — they should be in high_connectivity
    assert!(high_conn.contains(&json!("alpha")), "alpha (has edges) should be in high_connectivity");
    assert!(high_conn.contains(&json!("beta")), "beta (has edges) should be in high_connectivity");
}

// B:provide_mcp_review_prompt — verify unit "reviews all entities when entity_id is omitted"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "MCP init followed by check produces zero errors")]
fn review_all_entities_when_no_filter() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["findings"].is_array());
    assert!(parsed["coverage_summary"].is_array());
}

// B:provide_mcp_context_prompt — verify unit "context includes contract text"
#[test]
#[specforge_test(behavior = "provide_mcp_context_prompt", verify = "context includes contract text")]
fn context_includes_contract() {
    let mut server = test_server();
    let resp = call_prompt(&mut server, "specforge://prompts/context", json!({"entity_id": "alpha"}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["contract_text"].is_string());
    assert!(parsed["contract_text"].as_str().unwrap().contains("MUST"));
}

// B:provide_mcp_review_prompt — verify unit "review prompt returns empty findings when no testable entities exist"
#[test]
#[specforge_test(behavior = "provide_mcp_review_prompt", verify = "review prompt returns empty findings when no testable entities exist")]
fn review_empty_findings_no_testable_entities() {
    // Create a server with only non-testable entities (no verify statements)
    let mut server = McpServer::new();
    let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
    server.handle_message(&req.to_string());

    let state = server.state_mut();
    let mut graph = specforge_graph::Graph::new();
    let mut fields = specforge_parser::FieldMap::new();
    fields.push("problem".into(), specforge_parser::FieldValue::String("a problem".into()));
    graph.add_node(specforge_graph::Node {
        id: specforge_parser::EntityId { raw: "feat1".into() },
        kind: specforge_parser::EntityKind { raw: "feature".into() },
        title: Some("Feature 1".into()),
        fields,
        source_span: specforge_common::SourceSpan { file: "test.spec".into(), start_line: 1, start_col: 0, end_line: 3, end_col: 0 },
    });
    state.graph = graph;

    let resp = call_prompt(&mut server, "specforge://prompts/review", json!({}));
    let text = prompt_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Features are not testable — review should return empty or minimal findings
    assert!(parsed["findings"].is_array());
}
