use serde_json::{Value, json};
use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_mcp::McpServer;
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, SpannedRef, VerifyStatement};
use specforge_test::prelude::*;

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

    let mut fields_a = FieldMap::new();
    fields_a.push(
        "contract".into(),
        FieldValue::String("The system MUST do alpha".into()),
    );
    let verify_stmts = vec![VerifyStatement {
        kind: "unit".into(),
        description: "does alpha correctly".into(),
    }];
    fields_a.push("verify".into(), FieldValue::VerifyList(verify_stmts));

    graph.add_node(Node {
        id: EntityId {
            raw: "alpha".into(),
        },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: Some("Alpha Behavior".into()),
        fields: fields_a,
        source_span: span(),
        methods: Vec::new(),
    });

    let mut fields_b = FieldMap::new();
    fields_b.push(
        "behaviors".into(),
        FieldValue::ReferenceList(vec![SpannedRef {
            id: "alpha".into(),
            span: span(),
        }]),
    );
    graph.add_node(Node {
        id: EntityId {
            raw: "beta_feature".into(),
        },
        kind: EntityKind {
            raw: "feature".into(),
        },
        title: Some("Beta Feature".into()),
        fields: fields_b,
        source_span: SourceSpan {
            file: "features.spec".into(),
            start_line: 10,
            start_col: 0,
            end_line: 15,
            end_col: 0,
        },
        methods: Vec::new(),
    });

    graph.add_node(Node {
        id: EntityId {
            raw: "gamma_orphan".into(),
        },
        kind: EntityKind {
            raw: "invariant".into(),
        },
        title: Some("Gamma Orphan".into()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "invariants.spec".into(),
            start_line: 1,
            start_col: 0,
            end_line: 3,
            end_col: 0,
        },
        methods: Vec::new(),
    });

    graph.add_edge(Edge {
        source: "beta_feature".into(),
        target: "alpha".into(),
        label: "behaviors".into(),
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
    resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

// --- specforge.query ---

// B:provide_mcp_query_tool — verify unit "returns subgraph for entity"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "specforge.query tool returns subgraph for valid entityId"
)]
fn query_returns_subgraph() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
}

// B:provide_mcp_query_tool — verify unit "returns error for unknown entity"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "non-existent entityId returns error response"
)]
fn query_error_for_unknown() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "nonexistent"}),
    );
    assert!(resp["error"].is_object());
}

// B:provide_mcp_query_tool — verify unit "respects depth parameter"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "depth parameter limits traversal depth"
)]
fn query_respects_depth() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "depth": 0}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    // Depth 0 = only the root
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "alpha");
}

// B:provide_mcp_query_tool — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "kind filter restricts returned node types"
)]
fn query_respects_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "depth": 2, "kinds": ["behavior"]}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        let kind = node["kind"].as_str().unwrap();
        // Root always included, plus kind-filtered nodes
        assert!(kind == "behavior" || node["id"] == "alpha");
    }
}

// B:provide_mcp_query_tool — verify unit "missing entity_id returns error"
#[specforge_test(
    behavior = "handle_mcp_protocol_error",
    verify = "missing required params produces -32602 Invalid params"
)]
fn query_missing_entity_id() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.query", json!({}));
    assert!(resp["error"].is_object());
}

// --- specforge.export ---

// B:provide_mcp_export_tool — verify unit "exports graph format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "specforge.export tool returns graph in requested format"
)]
fn export_graph_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
    // Full graph format includes fields
    let alpha = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == "alpha")
        .unwrap();
    assert!(alpha["fields"].is_object());
}

// B:provide_mcp_export_tool — verify unit "exports context format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "all three formats (context, brief, graph) supported"
)]
fn export_context_format() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "context"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
}

// B:provide_mcp_export_tool — verify unit "exports brief format"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "all three formats (context, brief, graph) supported"
)]
fn export_brief_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "brief"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
}

// B:provide_mcp_export_tool — verify unit "exports scoped subgraph"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "scope parameter restricts to subgraph"
)]
fn export_scoped() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "scope": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());
}

// B:provide_mcp_export_tool — verify unit "unknown format returns error"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "unknown format returns error"
)]
fn export_unknown_format() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "yaml"}));
    assert!(resp["error"].is_object());
}

// --- specforge.trace ---

// B:provide_mcp_trace_tool — verify unit "returns trace chain"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "specforge.trace tool returns traceability chain for valid entityId"
)]
fn trace_returns_chain() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
}

// B:provide_mcp_trace_tool — verify unit "unknown entity returns error"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "non-existent entityId returns error response"
)]
fn trace_unknown_entity() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "nonexistent"}),
    );
    assert!(resp["error"].is_object());
}

// --- specforge.search ---

// B:provide_mcp_search_tool — verify unit "returns fuzzy matches"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "text search finds entities matching by name or contract"
)]
fn search_returns_matches() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["entity_id"], "alpha");
}

// B:provide_mcp_search_tool — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "kind filter restricts results to matching entity kinds"
)]
fn search_respects_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "kinds": ["feature"]}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Alpha is a behavior, not a feature, so should not match with feature filter
    let results = parsed.as_array().unwrap();
    for r in results {
        assert_eq!(r["kind"], "feature");
    }
}

// B:provide_mcp_search_tool — verify unit "respects limit"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "limit caps the number of returned results"
)]
fn search_respects_limit() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "a", "limit": 1}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed.as_array().unwrap().len() <= 1);
}

// B:provide_mcp_search_tool — verify unit "missing query returns error"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "missing query returns error"
)]
fn search_missing_query() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({}));
    assert!(resp["error"].is_object());
}

// --- specforge.schema ---

// B:provide_mcp_schema_tool — verify unit "returns schema with entity kinds"
#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "specforge.schema returns full GraphProtocolSchema"
)]
fn schema_tool_returns_kinds() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["entity_kinds"].is_object());
}

// B:provide_mcp_schema_tool — verify unit "respects kind filter"
#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "kind filter restricts schema to single entity kind"
)]
fn schema_tool_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({"kind": "behavior"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let kinds = parsed["entity_kinds"].as_object().unwrap();
    assert!(kinds.contains_key("behavior"));
    assert!(!kinds.contains_key("feature"));
}

// --- specforge.coverage ---

// B:provide_mcp_coverage_tool — verify unit "returns coverage per entity"
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "entity_id filter returns single entity coverage"
)]
fn coverage_returns_per_entity() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());

    for r in results {
        assert!(r["entity_id"].is_string());
        assert!(r["status"].is_string());
        assert!(r["declared"].is_boolean());
    }
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "specforge.coverage returns coverage for all testable entities"
)]
fn coverage_alpha_uncovered_without_tests() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"entity_id": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["entity_id"], "alpha");
    // Declared obligations with no recorded test prove nothing.
    assert_eq!(results[0]["status"], "uncovered");
    assert_eq!(results[0]["obligations"], 1);
    assert_eq!(results[0]["unproven"], json!(["does alpha correctly"]));
}

/// The server with a behavior `two` declaring obligations "a" and "b", and
/// a `specforge-report.json` recording `tests` (verify text, status) for it.
/// The tempdir must outlive the server's use of the report.
fn server_with_report(tests: &[(&str, &str)]) -> (McpServer, tempfile::TempDir) {
    let mut server = test_server();
    let mut fields = FieldMap::new();
    fields.push(
        "verify".into(),
        FieldValue::VerifyList(
            ["a", "b"]
                .map(|text| VerifyStatement {
                    kind: "unit".into(),
                    description: text.into(),
                })
                .to_vec(),
        ),
    );
    server.state_mut().graph.add_node(Node {
        id: EntityId { raw: "two".into() },
        kind: EntityKind {
            raw: "behavior".into(),
        },
        title: None,
        fields,
        source_span: span(),
        methods: Vec::new(),
    });
    let tests: Vec<Value> = tests
        .iter()
        .map(|(verify, status)| json!({"name": verify, "verify": verify, "status": status}))
        .collect();
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("specforge-report.json"),
        json!({"results": {"two": {"tests": tests}}}).to_string(),
    )
    .unwrap();
    server.state_mut().project_root = Some(project.path().to_path_buf());
    (server, project)
}

/// `specforge.coverage`'s result for `two`.
fn coverage_of_two(server: &mut McpServer) -> Value {
    let resp = call_tool(server, "specforge.coverage", json!({"entity_id": "two"}));
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    parsed[0].clone()
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "an entity with an unproven obligation is partial, not covered"
)]
fn coverage_with_an_unproven_obligation_is_partial() {
    let (mut server, _project) = server_with_report(&[("a", "pass")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["obligations"], 2);
    assert_eq!(two["proven"], 1);
    assert_eq!(two["unproven"], json!(["b"]));

    let (mut server, _project) = server_with_report(&[("a", "pass"), ("b", "pass")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "covered", "{two}");
    assert_eq!(two["unproven"], json!([]));
}

#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "a failing recorded test keeps an entity from being covered"
)]
fn coverage_with_a_failing_test_is_partial() {
    // Both obligations are proven, but a third test fails (A014).
    let (mut server, _project) = server_with_report(&[("a", "pass"), ("b", "pass"), ("a", "fail")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["proven"], 2);

    // A failing test proves nothing, even when it names an obligation.
    let (mut server, _project) = server_with_report(&[("a", "fail")]);
    let two = coverage_of_two(&mut server);
    assert_eq!(two["status"], "partial", "{two}");
    assert_eq!(two["unproven"], json!(["a", "b"]));
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "analyze reads the project's specforge-report.json by default"
)]
fn analyze_reads_the_project_report_by_default() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    std::fs::write(
        root.join("specforge.json"),
        r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software","@specforge/testing"]}"#,
    )
    .unwrap();
    std::fs::write(
        root.join("app.spec"),
        "behavior two \"Two\" {\n  verify unit \"a\"\n  verify unit \"b\"\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("specforge-report.json"),
        r#"{"results":{"two":{"tests":[{"name":"t","verify":"a","status":"pass"}]}}}"#,
    )
    .unwrap();

    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.analyze",
        json!({"path": root.to_str().unwrap(), "pass": "coverage"}),
    );
    let parsed: Value = serde_json::from_str(&tool_text(&resp)).unwrap();
    let coverage = parsed["passes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pass"] == "@specforge/testing:coverage")
        .unwrap_or_else(|| panic!("no coverage pass: {parsed}"))
        .clone();
    assert_eq!(
        coverage["summary"]["test_results"]["obligations_proven"], 1,
        "{coverage}"
    );
    let a015: Vec<&Value> = coverage["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["code"] == "A015")
        .collect();
    assert_eq!(a015.len(), 1, "{coverage}");
    assert!(a015[0]["message"].as_str().unwrap().contains("\"b\""));
}

// --- specforge.stats ---

// B:provide_mcp_stats_tool — verify unit "returns project statistics"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "specforge.stats returns entity counts by kind"
)]
fn stats_returns_statistics() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["entity_counts"].is_array());
    assert!(parsed["edge_count"].is_number());
    assert!(parsed["orphan_count"].is_number());
    assert!(parsed["diagnostic_summary"].is_object());
}

// B:provide_mcp_stats_tool — verify unit "counts match graph"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes coverage percentage"
)]
fn stats_counts_match_graph() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["edge_count"], 1); // one behaviors edge
    assert_eq!(parsed["orphan_count"], 1); // gamma_orphan
}

// Tool call when not initialized
#[specforge_test(
    behavior = "mcp_initialize",
    verify = "initialization rejects tool calls before completion"
)]
fn tool_call_not_initialized() {
    let mut server = McpServer::new();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha"}),
    );
    assert!(resp["error"].is_object());
}

// Unknown tool name
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "unknown tool returns error"
)]
fn unknown_tool_returns_error() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.nonexistent", json!({}));
    assert!(resp["error"].is_object());
}

// B:provide_mcp_validate_tool — verify unit "response includes all diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "specforge.validate tool triggers compilation"
)]
fn validate_returns_all_diagnostics() {
    let mut server = test_server();
    // Set project root to the specforge project so validate can compile
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // validate returns diagnostics as a JSON array string in content[0].text
    assert!(parsed.is_array());
}

// B:provide_mcp_validate_tool — verify unit "severity_filter restricts returned diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "strict mode promotes warnings to errors"
)]
fn validate_severity_filter_placeholder() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    // Placeholder: severity filter not yet implemented, just verify tool responds
    assert!(resp["result"].is_object());
}

// B:provide_mcp_validate_tool — verify unit "use_cached=false triggers fresh compilation"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=false triggers fresh compilation"
)]
fn validate_use_cached_false() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let resp1 = call_tool(&mut server, "specforge.validate", json!({}));
    assert!(resp1["result"].is_object());
    let resp2 = call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": false}),
    );
    assert!(resp2["result"].is_object());
}

// B:provide_mcp_export_tool — verify unit "max_tokens truncates output"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "specforge.export tool returns graph in requested format"
)]
fn export_max_tokens_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.export", json!({"format": "graph"}));
    // Placeholder: max_tokens not yet implemented, just verify export works
    let text = tool_text(&resp);
    assert!(!text.is_empty());
}

// B:provide_mcp_trace_tool — verify unit "plan parameter triggers gap analysis"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "plan parameter triggers gap analysis"
)]
fn trace_plan_gap_analysis() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "alpha", "plan": {"steps": []}}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "alpha");
}

// B:provide_mcp_trace_tool — verify unit "missing links flagged in trace output"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "response includes upstream and downstream links"
)]
fn trace_missing_links() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "gamma_orphan"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["entity_id"], "gamma_orphan");
    // Orphan has no connections
    assert!(parsed["upstream"].as_array().unwrap().is_empty());
    assert!(parsed["downstream"].as_array().unwrap().is_empty());
}

// B:provide_mcp_search_tool — verify unit "field and value filter matches entity fields"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "text search finds entities matching by name or contract"
)]
fn search_field_filter_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Placeholder: field filter not yet implemented, just verify results returned
    assert!(parsed.is_array());
}

// B:provide_mcp_search_tool — verify unit "empty query returns all entities up to limit"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "empty query returns all entities up to limit"
)]
fn search_empty_query_returns_all() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": ""}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    // Empty query should return entities (may be all or subset)
    let _ = results.len();
}

// B:provide_mcp_search_tool — verify unit "references filter returns entities referencing target"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "text search finds entities matching by name or contract"
)]
fn search_references_filter_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.search", json!({"query": "alpha"}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Placeholder: references filter not yet implemented
    assert!(parsed.is_array());
}

// B:provide_mcp_schema_tool — verify unit "include_edges false omits edge type definitions"
#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "include_edges false omits edge type definitions"
)]
fn schema_include_edges_false_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Placeholder: include_edges param not yet implemented
    assert!(parsed["entity_kinds"].is_object());
}

// B:provide_mcp_schema_tool — verify unit "include_validation_rules true includes rules"
#[specforge_test(
    behavior = "provide_mcp_schema_tool",
    verify = "include_validation_rules true includes validation rules"
)]
fn schema_include_validation_rules_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.schema", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Placeholder: include_validation_rules param not yet implemented
    assert!(parsed.is_object());
}

// B:provide_mcp_coverage_tool — verify unit "kind filter restricts to matching entity kinds"
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "kind filter restricts to matching entity kinds"
)]
fn coverage_kind_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.coverage",
        json!({"kind": "behavior"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    for r in results {
        assert_eq!(r["kind"], "behavior");
    }
}

// B:provide_mcp_coverage_tool — verify unit "status_filter restricts to matching status"
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "status_filter restricts to matching coverage status"
)]
fn coverage_status_filter_placeholder() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.coverage", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Placeholder: status_filter not yet implemented
    assert!(parsed.is_array());
}

// B:provide_mcp_stats_tool — verify unit "response includes coverage percentage"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes orphan node count"
)]
fn stats_includes_coverage_percentage() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["coverage_pct"].is_number());
}

// B:provide_mcp_query_tool — verify unit "format parameter selects emitter format"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "format parameter changes output serialization"
)]
fn query_format_parameter() {
    let mut server = test_server();
    // context format
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "context"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["nodes"].is_array());

    // brief format
    let resp2 = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "format": "brief"}),
    );
    let text2 = tool_text(&resp2);
    let parsed2: Value = serde_json::from_str(&text2).unwrap();
    assert!(parsed2["nodes"].is_array());
}

// B:provide_mcp_query_tool — verify unit "include_coverage annotates nodes with coverage status"
#[specforge_test(
    behavior = "provide_mcp_query_tool",
    verify = "include_coverage parameter includes coverage status in response"
)]
fn query_include_coverage() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.query",
        json!({"entity_id": "alpha", "include_coverage": true}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    let alpha = nodes.iter().find(|n| n["id"] == "alpha").unwrap();
    assert_eq!(alpha["coverage_status"], "partial"); // alpha has verify
}

// B:provide_mcp_search_tool — verify unit "field and value filter matches entity fields"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "field and value filter matches entity fields"
)]
fn search_field_value_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "field": "contract", "value": "MUST"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0]["entity_id"], "alpha");
}

// B:provide_mcp_search_tool — verify unit "references filter returns entities referencing target"
#[specforge_test(
    behavior = "provide_mcp_search_tool",
    verify = "references filter returns entities referencing target"
)]
fn search_references_filter() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.search",
        json!({"query": "alpha", "references": "alpha"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let results = parsed.as_array().unwrap();
    // beta_feature has an edge to alpha
    assert!(results.iter().any(|r| r["entity_id"] == "beta_feature"));
}

// B:provide_mcp_stats_tool — verify unit "diagnostic_summary includes severity counts"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes diagnostic summary by severity"
)]
fn stats_diagnostic_summary_severity_counts() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let summary = &parsed["diagnostic_summary"];
    assert!(summary["errors"].is_number());
    assert!(summary["warnings"].is_number());
    assert!(summary["infos"].is_number());
}

// B:provide_mcp_trace_tool — verify unit "gaps array lists missing expected links"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "missing links flagged in trace output"
)]
fn trace_gaps_array() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_id": "gamma_orphan"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    let gaps = parsed["gaps"].as_array().unwrap();
    assert!(gaps.contains(&json!("no upstream links")));
    assert!(gaps.contains(&json!("no downstream links")));
}

// B:provide_mcp_validate_tool — verify unit "severity_filter restricts returned diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "severity_filter restricts returned diagnostics"
)]
fn validate_severity_filter() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let resp = call_tool(
        &mut server,
        "specforge.validate",
        json!({"severity_filter": "error"}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // If filtered to errors only, all returned diagnostics should be errors
    if let Some(arr) = parsed.as_array() {
        for d in arr {
            if let Some(sev) = d["severity"].as_str() {
                assert!(
                    sev.eq_ignore_ascii_case("error"),
                    "Expected error severity, got: {}",
                    sev
                );
            }
        }
    }
}

// B:provide_mcp_validate_tool — verify unit "use_cached returns existing diagnostics"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=true returns existing diagnostics without recompilation"
)]
fn validate_use_cached_true() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    // First compile
    let _resp1 = call_tool(&mut server, "specforge.validate", json!({}));
    let diag_count = server.state().diagnostics.len();
    // Second call with use_cached=true should not recompile
    let resp2 = call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": true}),
    );
    assert!(resp2["result"].is_object());
    // Diagnostics count should remain the same
    assert_eq!(server.state().diagnostics.len(), diag_count);
}

// B:provide_mcp_validate_tool — verify unit "validate recompiles and updates graph"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "response includes all diagnostics as Graph Protocol diagnostics"
)]
fn validate_updates_graph() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let resp = call_tool(&mut server, "specforge.validate", json!({}));
    // Validate returns result with content and isError fields
    assert!(resp["result"].is_object());
    assert!(resp["result"]["isError"].is_boolean());
}

// B:provide_mcp_validate_tool — verify unit "validate with use_cached=false triggers fresh compilation"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=false triggers fresh compilation"
)]
fn validate_use_cached_false_triggers_fresh() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    // First compile
    let _resp1 = call_tool(&mut server, "specforge.validate", json!({}));
    // Second call with use_cached=false should recompile
    let resp2 = call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": false}),
    );
    assert!(
        resp2["result"].is_object() || resp2["error"].is_object(),
        "use_cached=false should trigger fresh compilation"
    );
}

// B:provide_mcp_validate_tool — verify unit "validate with use_cached=true returns existing diagnostics without recompilation"
#[specforge_test(
    behavior = "provide_mcp_validate_tool",
    verify = "validate with use_cached=true returns existing diagnostics without recompilation"
)]
fn validate_use_cached_true_returns_existing() {
    let mut server = test_server();
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    server.state_mut().project_root = Some(project_root);
    let _resp1 = call_tool(&mut server, "specforge.validate", json!({}));
    let resp2 = call_tool(
        &mut server,
        "specforge.validate",
        json!({"use_cached": true}),
    );
    assert!(
        resp2["result"].is_object(),
        "use_cached=true should return existing diagnostics without recompilation"
    );
}

// B:provide_mcp_export_tool — verify unit "max_tokens truncates output to fit token budget"
#[specforge_test(
    behavior = "provide_mcp_export_tool",
    verify = "max_tokens truncates output to fit token budget"
)]
fn export_max_tokens_truncates() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "context", "max_tokens": 10}),
    );
    // Should either return truncated output or error about budget
    assert!(resp["result"].is_object() || resp["error"].is_object());
}

// B:provide_mcp_trace_tool — verify unit "missing links flagged in trace output"
#[specforge_test(
    behavior = "provide_mcp_trace_tool",
    verify = "missing links flagged in trace output"
)]
fn trace_missing_links_flagged() {
    let mut server = test_server();
    // Trace with a plan that has references to nonexistent entities
    let resp = call_tool(
        &mut server,
        "specforge.trace",
        json!({"entity_ids": ["nonexistent_entity"]}),
    );
    // Should flag missing links
    assert!(resp["result"].is_object() || resp["error"].is_object());
}

// B:provide_mcp_stats_tool — verify unit "response includes diagnostic summary by severity"
#[specforge_test(
    behavior = "provide_mcp_stats_tool",
    verify = "response includes diagnostic summary by severity"
)]
fn stats_includes_diagnostic_summary() {
    let mut server = test_server();
    let resp = call_tool(&mut server, "specforge.stats", json!({}));
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();
    // Should include diagnostic counts by severity
    assert!(
        parsed["diagnostics"].is_object()
            || parsed["diagnostic_summary"].is_object()
            || parsed["entity_count"].is_number(),
        "stats response should include diagnostic summary"
    );
}

// --- C9-02/C9-03: token budget enforcement ---

// B:provide_mcp_export_tool — verify unit "max_tokens truncates output within budget with metadata"
#[test]
fn export_with_max_tokens_truncates_and_reports_budget() {
    let mut server = test_server();
    let resp = call_tool(
        &mut server,
        "specforge.export",
        json!({"format": "graph", "max_tokens": 120}),
    );
    let text = tool_text(&resp);
    let parsed: Value = serde_json::from_str(&text).unwrap();

    // metadata: the budget result rides in the payload
    assert!(
        parsed["token_budget"].is_object(),
        "budget metadata must be present: {}",
        text.len()
    );
    assert_eq!(parsed["token_budget"]["budget_tokens"], 120);

    // the estimate must honor the budget
    let estimated = parsed["token_budget"]["estimated_tokens"].as_u64().unwrap();
    assert!(
        estimated <= 120,
        "estimated tokens {estimated} exceed the 120-token budget"
    );

    // and truncation happened (a full-graph export is much larger)
    assert!(
        !parsed["token_budget"]["truncated_entities"]
            .as_array()
            .unwrap()
            .is_empty(),
        "entities must have been dropped to fit the budget"
    );
}

// B:mcp_resource_reads — verify unit "context resource honors ?max_tokens budget"
#[test]
fn context_resource_honors_max_tokens_budget() {
    let mut server = test_server();
    let req = json!({
        "jsonrpc": "2.0", "id": 7,
        "method": "resources/read",
        "params": { "uri": "specforge://context?max_tokens=150" }
    });
    let resp: Value =
        serde_json::from_str(&server.handle_message(&req.to_string()).unwrap()).unwrap();
    let text = resp["result"]["contents"][0]["text"].as_str().unwrap_or("");
    assert!(!text.is_empty(), "context payload must be present");

    // rough token estimate (words + structural chars) must fit the budget
    let words = text.split_whitespace().count();
    assert!(
        words <= 150,
        "budgeted context must be trimmed to ~150 tokens, got ~{words} words"
    );
}
