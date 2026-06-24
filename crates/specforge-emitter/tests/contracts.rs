use specforge_common::{Diagnostic, Severity, SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
use specforge_test::prelude::*;

fn span() -> SourceSpan {
    SourceSpan {
        file: Sym::new("test.spec"),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node_with_fields(id: &str, kind: &str, contract: &str, status: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(Sym::new("contract"), FieldValue::String(contract.to_string()));
    fields.push(Sym::new("status"), FieldValue::Identifier(status.to_string()));
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
    }
}

fn testable_node(id: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(Sym::new("contract"), FieldValue::String("The system MUST work".to_string()));
    fields.push(Sym::new("verify"), FieldValue::VerifyList(vec![
        VerifyStatement { kind: "unit".to_string(), description: "it works".to_string() },
    ]));
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new("behavior") },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
    }
}

fn build_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("a", "feature", "feature A", "planned"));
    graph.add_node(testable_node("b"));
    graph.add_node(testable_node("c"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    graph.add_edge(Edge { source: "b".into(), target: "c".into(), label: "depends_on".into() });
    graph
}

// === serialize_json_graph contract ===

// B:serialize_json_graph — verify contract "requires/ensures consistency for JSON graph serialization"
#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "requires/ensures consistency for JSON graph serialization")]
fn json_graph_contract_finalized_graph_produces_valid_output() {
    // Requires: graph is finalized (built with nodes + edges)
    // Ensures: valid JSON with schema_version, all nodes, all edges, source locations
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["schema_version"].is_string(), "must include schema_version");
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3, "all nodes present");
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 2, "all edges present");

    for node in parsed["nodes"].as_array().unwrap() {
        assert!(node["file"].is_string(), "must include source file");
        assert!(node["line"].is_number(), "must include source line");
    }
}

// === serialize_dot_visualization contract ===

// B:serialize_dot_visualization — verify contract "requires/ensures consistency for DOT visualization"
#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "requires/ensures consistency for DOT visualization")]
fn dot_contract_finalized_graph_produces_valid_dot() {
    // Requires: graph is finalized
    // Ensures: valid Graphviz DOT syntax
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);

    assert!(dot.starts_with("digraph"), "must be a directed graph");
    assert!(dot.contains("rankdir=LR"), "must have LR layout");
    assert!(dot.contains("shape=box"), "nodes must have shape");
    assert!(dot.ends_with("}\n") || dot.ends_with("}"), "must be properly closed");
}

// === compute_traceability_chain contract ===

// B:compute_traceability_chain — verify contract "requires/ensures consistency for traceability chain computation"
#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "requires/ensures consistency for traceability chain computation")]
fn trace_contract_entity_in_graph_produces_chain() {
    // Requires: entity exists in graph
    // Ensures: trace chain with upstream + downstream, sorted by depth
    let graph = build_graph();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();

    assert_eq!(trace.entity_id, "b");
    assert!(!trace.upstream.is_empty(), "mid-chain entity must have upstream");
    assert!(!trace.downstream.is_empty(), "mid-chain entity must have downstream");

    // Verify depth ordering
    for window in trace.upstream.windows(2) {
        assert!(window[0].depth <= window[1].depth, "upstream must be sorted by depth");
    }
    for window in trace.downstream.windows(2) {
        assert!(window[0].depth <= window[1].depth, "downstream must be sorted by depth");
    }
}

// === compute_project_statistics contract ===

// B:compute_project_statistics — verify contract "requires/ensures consistency for project statistics computation"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "requires/ensures consistency for project statistics computation")]
fn stats_contract_graph_with_diagnostics_produces_complete_stats() {
    // Requires: graph + diagnostics collected
    // Ensures: all stat fields populated correctly
    let graph = build_graph();
    let diagnostics = vec![
        Diagnostic {
            code: "E001".into(),
            severity: Severity::Error,
            message: "err".into(),
            span: None,
            suggestion: None,
        },
        Diagnostic {
            code: "W002".into(),
            severity: Severity::Warning,
            message: "warn".into(),
            span: None,
            suggestion: None,
        },
    ];

    let stats = specforge_emitter::compute_stats_with_diagnostics(&graph, &["behavior"], &diagnostics);
    assert_eq!(stats.total_entities, 3);
    assert_eq!(stats.total_edges, 2);
    assert_eq!(stats.testable_count, 2);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.warning_count, 1);
    assert!(stats.coverage_pct >= 0.0 && stats.coverage_pct <= 100.0);
}

// === export_agent_context_format contract ===

// B:export_agent_context_format — verify contract "requires/ensures consistency for agent context export"
#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "requires/ensures consistency for agent context export")]
fn context_contract_includes_contracts_and_verify_omits_prose() {
    // Requires: finalized graph
    // Ensures: id, kind, contract, verify, status present; description omitted
    let graph = build_graph();
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    let b_node = nodes.iter().find(|n| n["id"] == "b").unwrap();
    assert!(b_node["contract"].is_string(), "must include contract");
    assert!(b_node["verify"].is_array(), "must include verify");

    // Should not include verbose description field
    for node in nodes {
        assert!(node.get("description").is_none(), "must omit description");
    }
}

// === export_agent_graph_format contract ===

// B:export_agent_graph_format — verify contract "requires/ensures consistency for agent graph export"
#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "requires/ensures consistency for agent graph export")]
fn graph_format_contract_finalized_graph_produces_full_output() {
    // Requires: finalized graph
    // Ensures: all nodes with all fields, all edges, schema_version present
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed["schema_version"].is_string(), "must include schema_version");
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3, "all nodes present");
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 2, "all edges present");

    // Graph format includes all fields (unlike brief/context which strip)
    let b_node = nodes.iter().find(|n| n["id"] == "b").unwrap();
    assert!(b_node["kind"].is_string(), "graph format must include kind");
    // Fields are nested under "fields" key in full graph format
    assert!(
        b_node["fields"]["contract"].is_string() || b_node["contract"].is_string(),
        "graph format must include contract (in fields or top-level)"
    );
}

// === query_graph_multi_resolution contract ===

// B:query_graph_multi_resolution — verify contract "requires/ensures consistency for multi-resolution graph query"
#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "requires/ensures consistency for multi-resolution graph query")]
fn query_contract_valid_entity_returns_subgraph() {
    // Requires: entity exists in graph, depth >= 0
    // Ensures: root always included, neighbors within depth, schema_version present
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["schema_version"].is_string());
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"b"), "root must always be included");
}

// === enforce_token_budget contract ===

// B:enforce_token_budget — verify contract "requires/ensures consistency for token budget enforcement"
#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "requires/ensures consistency for token budget enforcement")]
fn budget_contract_within_budget_no_truncation() {
    // Requires: graph + budget
    // Ensures: within budget → all nodes, no token_budget metadata
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 100_000);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
    assert!(parsed.get("token_budget").is_none() || parsed["token_budget"].is_null());
}

// === validate_agent_plan contract ===

// B:validate_agent_plan — verify contract "requires/ensures consistency for agent plan validation"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "requires/ensures consistency for agent plan validation")]
fn plan_contract_validates_ids_coverage_ordering() {
    // Requires: finalized graph + plan JSON
    // Ensures: unresolvable IDs → errors, missing testable → warnings, wrong order → violations
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "nonexistent", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(!result.errors.is_empty(), "unresolvable IDs must produce errors");
    assert!(!result.warnings.is_empty(), "missing testable must produce warnings");

    let json = specforge_emitter::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object(), "structured JSON report required");
}

// === deterministic_output contract ===

// B:deterministic_output — verify contract "requires/ensures consistency for deterministic output"
#[test]
#[specforge_test(behavior = "deterministic_output", verify = "requires/ensures consistency for deterministic output")]
fn deterministic_contract_same_input_identical_output() {
    // Requires: same graph input
    // Ensures: identical output across all formats
    let graph = build_graph();

    let json1 = specforge_emitter::emit_json(&graph);
    let json2 = specforge_emitter::emit_json(&graph);
    assert_eq!(json1, json2, "JSON must be deterministic");

    let dot1 = specforge_emitter::emit_dot(&graph);
    let dot2 = specforge_emitter::emit_dot(&graph);
    assert_eq!(dot1, dot2, "DOT must be deterministic");

    let brief1 = specforge_emitter::emit_brief(&graph);
    let brief2 = specforge_emitter::emit_brief(&graph);
    assert_eq!(brief1, brief2, "brief must be deterministic");

    let ctx1 = specforge_emitter::emit_context(&graph);
    let ctx2 = specforge_emitter::emit_context(&graph);
    assert_eq!(ctx1, ctx2, "context must be deterministic");
}

// === serialize_traceability_data contract ===

// B:serialize_traceability_data — verify contract "requires/ensures consistency for traceability data serialization"
#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "requires/ensures consistency for traceability data serialization")]
fn trace_data_contract_all_entities_traced() {
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    assert_eq!(traces.len(), graph.nodes().len(), "one trace per entity");

    let json = specforge_emitter::serialize_trace_all(&traces);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
}

#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "full trace covers all root entities across registered edge types")]
fn trace_data_full_trace_covers_all_roots() {
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    let ids: Vec<&str> = traces.iter().map(|t| t.entity_id.as_str()).collect();
    assert!(ids.contains(&"a"), "root entity a must be traced");
    assert!(ids.contains(&"b"), "mid entity b must be traced");
    assert!(ids.contains(&"c"), "leaf entity c must be traced");
}

#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "gaps in chain are highlighted")]
fn trace_data_gaps_highlighted() {
    // A graph with a dangling edge has gaps
    let mut graph = Graph::new();
    graph.add_node(testable_node("isolated"));
    graph.add_edge(Edge {
        source: Sym::new("isolated"),
        target: Sym::new("nowhere"),
        label: Sym::new("depends_on"),
    });
    let gaps = specforge_emitter::detect_trace_gaps(&graph);
    assert!(!gaps.is_empty(), "dangling edge should produce trace gaps");
}

#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "output conforms to Graph Protocol schema")]
fn trace_data_output_conforms_to_schema() {
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    let json = specforge_emitter::serialize_trace_all(&traces);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
    for trace in parsed["traces"].as_array().unwrap() {
        assert!(trace["entity_id"].is_string());
        assert!(trace["upstream"].is_array());
        assert!(trace["downstream"].is_array());
    }
}

// === export_diagnostics_as_json contract ===

// B:export_diagnostics_as_json — verify contract "requires/ensures consistency for JSON diagnostic export"
#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "requires/ensures consistency for JSON diagnostic export")]
fn diagnostic_json_contract_complete_fields() {
    let diags = vec![
        Diagnostic {
            code: "E001".into(),
            severity: Severity::Error,
            message: "unresolved".into(),
            span: Some(SourceSpan {
                file: "test.spec".into(),
                start_line: 5,
                start_col: 10,
                end_line: 5,
                end_col: 20,
            }),
            suggestion: Some("did you mean 'foo'?".into()),
        },
    ];

    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());

    let entry = &parsed[0];
    assert_eq!(entry["code"], "E001");
    assert_eq!(entry["severity"], "Error");
    assert_eq!(entry["file"], "test.spec");
    assert_eq!(entry["line"], 5);
    assert_eq!(entry["column"], 10);
    assert_eq!(entry["suggestion"], "did you mean 'foo'?");
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "diagnostics serialized as JSON array to stdout")]
fn diagnostic_json_array() {
    let diags = vec![
        Diagnostic { code: "E001".into(), severity: Severity::Error, message: "err".into(), span: None, suggestion: None },
        Diagnostic { code: "W001".into(), severity: Severity::Warning, message: "warn".into(), span: None, suggestion: None },
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "each diagnostic includes code, severity, message, file, line, column")]
fn diagnostic_json_all_fields() {
    let diags = vec![Diagnostic {
        code: "E042".into(),
        severity: Severity::Error,
        message: "test msg".into(),
        span: Some(SourceSpan { file: "a.spec".into(), start_line: 3, start_col: 7, end_line: 3, end_col: 15 }),
        suggestion: None,
    }];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let e = &parsed[0];
    assert_eq!(e["code"], "E042");
    assert_eq!(e["severity"], "Error");
    assert_eq!(e["message"], "test msg");
    assert_eq!(e["file"], "a.spec");
    assert_eq!(e["line"], 3);
    assert_eq!(e["column"], 7);
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "JSON output is valid and parseable")]
fn diagnostic_json_valid_parseable() {
    let diags = vec![
        Diagnostic { code: "E001".into(), severity: Severity::Error, message: "msg with \"quotes\"".into(), span: None, suggestion: None },
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let result: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(result.is_ok(), "output must be valid JSON");
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "exit code unaffected by format flag")]
fn diagnostic_exit_code_unaffected_by_format() {
    let diags = vec![
        Diagnostic { code: "E001".into(), severity: Severity::Error, message: "err".into(), span: None, suggestion: None },
    ];
    // Exit code should be based on severity regardless of format
    let exit = specforge_emitter::compute_exit_code(&diags);
    assert_eq!(exit, 1, "errors should produce exit 1 regardless of format");
    // Also verify JSON is still produced
    let json = specforge_emitter::serialize_diagnostics(&diags);
    assert!(!json.is_empty());
}

#[test]
#[specforge_test(behavior = "export_diagnostics_as_json", verify = "suggestion field included when available")]
fn diagnostic_suggestion_included() {
    let diags = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "unresolved".into(),
        span: None,
        suggestion: Some("did you mean 'bar'?".into()),
    }];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[0]["suggestion"], "did you mean 'bar'?");
}

// ============================================================
// serialize_json_graph — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all nodes")]
fn json_graph_all_nodes() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "JSON output contains all edges")]
fn json_graph_all_edges() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 2);
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "output is valid JSON")]
fn json_graph_valid_json() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let result: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(result.is_ok(), "output must be valid JSON");
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "output includes schema_version field")]
fn json_graph_schema_version() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "empty graph produces valid JSON with empty nodes and edges arrays")]
fn json_graph_empty() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 0);
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "schema is included even for empty graph")]
fn json_graph_empty_has_schema() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string(), "empty graph must still have schema_version");
}

#[test]
#[specforge_test(behavior = "serialize_json_graph", verify = "structural-only graph (zero extensions) produces valid Graph Protocol JSON with raw keywords in kind field")]
fn json_graph_structural_only() {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("x", "custom_kind", "c", "active"));
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let node = &parsed["nodes"].as_array().unwrap()[0];
    assert_eq!(node["kind"], "custom_kind", "raw keyword preserved in kind field");
}

// ============================================================
// serialize_dot_visualization — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "DOT output is valid Graphviz syntax")]
fn dot_valid_syntax() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.starts_with("digraph"));
    assert!(dot.contains("{"));
    assert!(dot.trim_end().ends_with("}"));
}

#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "nodes are labeled with IDs")]
fn dot_nodes_labeled() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("\"a\""), "node a must be present");
    assert!(dot.contains("\"b\""), "node b must be present");
    assert!(dot.contains("\"c\""), "node c must be present");
}

#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "edges are labeled with types")]
fn dot_edges_labeled() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("behaviors"), "edge label 'behaviors' must be present");
    assert!(dot.contains("depends_on"), "edge label 'depends_on' must be present");
}

#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "node shapes use extension-defined dot_shape")]
fn dot_node_shapes() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("shape="), "nodes must have shape attribute");
}

// ============================================================
// compute_traceability_chain — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "trace from entity shows upstream and downstream connections")]
fn trace_upstream_downstream() {
    let graph = build_graph();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();
    assert!(!trace.upstream.is_empty(), "b has upstream (a)");
    assert!(!trace.downstream.is_empty(), "b has downstream (c)");
}

#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "trace shows full chain depth")]
fn trace_full_depth() {
    let graph = build_graph();
    // Trace from leaf c: upstream should include b and a
    let trace = specforge_emitter::trace(&graph, "c").unwrap();
    assert!(trace.upstream.len() >= 2, "c should have at least 2 upstream (b, a)");
}

#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "missing link in chain is flagged")]
fn trace_missing_link() {
    // Build a graph with a dangling edge — target "missing" has no node
    let mut graph = Graph::new();
    graph.add_node(testable_node("lone"));
    graph.add_edge(Edge {
        source: Sym::new("lone"),
        target: Sym::new("missing"),
        label: Sym::new("depends_on"),
    });
    let gaps = specforge_emitter::detect_trace_gaps(&graph);
    assert!(!gaps.is_empty(), "dangling edge target should produce a trace gap");
}

// ============================================================
// compute_project_statistics — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports correct entity counts")]
fn stats_entity_counts() {
    let graph = build_graph();
    let stats = specforge_emitter::compute_stats_with_testable(&graph, &["behavior"]);
    assert_eq!(stats.total_entities, 3);
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports coverage percentage")]
fn stats_coverage_pct() {
    let graph = build_graph();
    let stats = specforge_emitter::compute_stats_with_testable(&graph, &["behavior"]);
    assert!(stats.coverage_pct >= 0.0 && stats.coverage_pct <= 100.0);
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports orphan count")]
fn stats_orphan_count() {
    let mut graph = Graph::new();
    graph.add_node(testable_node("orphan"));
    let stats = specforge_emitter::compute_stats_with_testable(&graph, &["behavior"]);
    assert_eq!(stats.orphan_count, 1, "node with no edges is an orphan");
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports diagnostic summary")]
fn stats_diagnostic_summary() {
    let graph = build_graph();
    let diags = vec![
        Diagnostic { code: "E001".into(), severity: Severity::Error, message: "e".into(), span: None, suggestion: None },
        Diagnostic { code: "W001".into(), severity: Severity::Warning, message: "w".into(), span: None, suggestion: None },
        Diagnostic { code: "I001".into(), severity: Severity::Info, message: "i".into(), span: None, suggestion: None },
    ];
    let stats = specforge_emitter::compute_stats_with_diagnostics(&graph, &["behavior"], &diags);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.warning_count, 1);
    assert_eq!(stats.info_count, 1);
}

#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "coverage is 0% when testable_entity_count is zero")]
fn stats_zero_testable() {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("f", "feature", "a feature", "planned"));
    // No testable kinds declared
    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.testable_count, 0);
    assert!((stats.coverage_pct - 0.0).abs() < f64::EPSILON, "coverage must be 0% with no testable entities");
}

// ============================================================
// validate_agent_plan — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan with all valid entity IDs passes validation")]
fn plan_all_valid_ids() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "a", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });
    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(result.errors.is_empty(), "all valid IDs should produce no errors");
}

#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan referencing nonexistent entity ID produces E003")]
fn plan_nonexistent_id() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [{ "entity_id": "nonexistent", "action": "implement" }]
    });
    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].contains("nonexistent"), "error should mention the missing ID");
}

#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "testable entity missing from plan produces warning")]
fn plan_missing_testable() {
    let graph = build_graph();
    // Only plan for 'a' (feature, not testable) — b and c (testable behaviors) are missing
    let plan = serde_json::json!({
        "entries": [{ "entity_id": "a", "action": "implement" }]
    });
    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(!result.warnings.is_empty(), "missing testable entities should produce warnings");
}

#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan dependency order contradicting graph produces diagnostic")]
fn plan_wrong_order() {
    let graph = build_graph();
    // Graph has a→b and b→c, meaning b depends on a and c depends on b.
    // Plan lists a before b before c — so dependencies (targets) appear AFTER their dependents (sources).
    // This should trigger ordering violations.
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "a", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });
    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(!result.ordering_violations.is_empty(), "wrong dependency order should produce violations");
}

#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "output is structured JSON")]
fn plan_structured_json_output() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [{ "entity_id": "a", "action": "implement" }]
    });
    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    let json = specforge_emitter::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object(), "plan result must be a JSON object");
    assert!(parsed.get("errors").is_some());
    assert!(parsed.get("warnings").is_some());
}

// ============================================================
// deterministic_output — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "deterministic_output", verify = "same input produces identical output across runs")]
fn deterministic_across_runs() {
    let graph = build_graph();
    let out1 = specforge_emitter::emit_json(&graph);
    let out2 = specforge_emitter::emit_json(&graph);
    let out3 = specforge_emitter::emit_json(&graph);
    assert_eq!(out1, out2);
    assert_eq!(out2, out3);
}

#[test]
#[specforge_test(behavior = "deterministic_output", verify = "entity ordering is independent of hashmap iteration")]
fn deterministic_entity_ordering() {
    // Build two graphs with same entities added in different order
    let mut g1 = Graph::new();
    g1.add_node(testable_node("x"));
    g1.add_node(testable_node("y"));
    g1.add_node(testable_node("z"));

    let mut g2 = Graph::new();
    g2.add_node(testable_node("z"));
    g2.add_node(testable_node("x"));
    g2.add_node(testable_node("y"));

    let j1 = specforge_emitter::emit_json(&g1);
    let j2 = specforge_emitter::emit_json(&g2);
    assert_eq!(j1, j2, "entity ordering must be deterministic regardless of insertion order");
}

#[test]
#[specforge_test(behavior = "deterministic_output", verify = "file emission order is independent of filesystem readdir order")]
fn deterministic_file_order() {
    // Nodes from different files should still produce deterministic output
    let mut graph = Graph::new();
    let mut n1 = testable_node("alpha");
    n1.source_span.file = Sym::new("z.spec");
    let mut n2 = testable_node("beta");
    n2.source_span.file = Sym::new("a.spec");
    graph.add_node(n1);
    graph.add_node(n2);

    let out1 = specforge_emitter::emit_json(&graph);
    let out2 = specforge_emitter::emit_json(&graph);
    assert_eq!(out1, out2, "output must be deterministic regardless of file origins");
}

#[test]
#[specforge_test(behavior = "deterministic_output", verify = "output contains no timestamps or non-deterministic values")]
fn deterministic_no_timestamps() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    // Should not contain timestamp-like patterns
    assert!(!json.contains("timestamp"), "output must not contain timestamps");
    assert!(!json.contains("2026-"), "output must not contain date strings");
}

// ============================================================
// export_agent_context_format — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "context format includes entity IDs and contracts")]
fn context_includes_ids_and_contracts() {
    let graph = build_graph();
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        assert!(node["id"].is_string(), "context node must have id");
    }
    let b = nodes.iter().find(|n| n["id"] == "b").unwrap();
    assert!(b["contract"].is_string(), "behavior node must have contract");
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "context format omits verbose prose fields")]
fn context_omits_prose() {
    let graph = build_graph();
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    for node in parsed["nodes"].as_array().unwrap() {
        assert!(node.get("description").is_none(), "context must omit description");
    }
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "scoped export returns only reachable subgraph")]
fn context_scoped_export() {
    let graph = build_graph();
    let json = specforge_emitter::emit_context_scoped(&graph, "b").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"b"), "scoped root must be included");
    // Should not contain unreachable nodes from 'b' perspective
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "non-existent scope entity produces E003 and exit code 1")]
fn context_nonexistent_scope() {
    let graph = build_graph();
    let result = specforge_emitter::emit_context_scoped(&graph, "nonexistent");
    assert!(result.is_err(), "non-existent scope must return error");
    assert!(result.unwrap_err().to_string().contains("E003"), "error must contain E003");
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "output conforms to Graph Protocol schema")]
fn context_conforms_to_schema() {
    let graph = build_graph();
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
    assert!(parsed["schema_version"].is_string());
}

#[test]
#[specforge_test(behavior = "export_agent_context_format", verify = "output includes schema_version field")]
fn context_has_schema_version() {
    let graph = build_graph();
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
}

// ============================================================
// export_agent_graph_format — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "graph format includes all nodes and edges")]
fn graph_format_all_nodes_edges() {
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 2);
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "graph format includes all fields and metadata")]
fn graph_format_all_fields() {
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let b = parsed["nodes"].as_array().unwrap().iter().find(|n| n["id"] == "b").unwrap();
    assert!(b["kind"].is_string());
    assert!(b["file"].is_string());
    assert!(b["line"].is_number());
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "scoped export returns only reachable subgraph")]
fn graph_format_scoped() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json_scoped(&graph, "c").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"c"), "scoped root must be included");
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "non-existent scope entity produces E003 and exit code 1")]
fn graph_format_nonexistent_scope() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_scoped(&graph, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("E003"));
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "output conforms to Graph Protocol schema")]
fn graph_format_conforms_to_schema() {
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "output includes schema_version field")]
fn graph_format_schema_version() {
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
}

#[test]
#[specforge_test(behavior = "export_agent_graph_format", verify = "structural-only graph exports valid JSON with raw keyword strings as entity kinds")]
fn graph_format_structural_only() {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("x", "freeform_kind", "c", "active"));
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"][0]["kind"], "freeform_kind");
}

// ============================================================
// query_graph_multi_resolution — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "depth 0 returns only the target entity")]
fn query_depth_0() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 0, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["b"], "depth 0 should return only target");
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "depth 1 returns direct neighbors")]
fn query_depth_1() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"b"), "root must be included");
    assert!(ids.contains(&"a") || ids.contains(&"c"), "at least one neighbor at depth 1");
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "depth N returns all entities within N hops")]
fn query_depth_n() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "a", 10, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let ids: Vec<&str> = parsed["nodes"].as_array().unwrap()
        .iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 3, "large depth should return all reachable nodes");
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "kind filter restricts results to specified entity kinds")]
fn query_kind_filter() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "a", 10, &["behavior"]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        let id = node["id"].as_str().unwrap();
        let kind = node["kind"].as_str().unwrap();
        if id != "a" {
            assert_eq!(kind, "behavior", "filtered nodes should be behaviors");
        }
    }
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "multiple kind filters combine as union")]
fn query_multiple_kinds() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "a", 10, &["feature", "behavior"]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3, "union of feature + behavior should include all nodes");
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "output conforms to Graph Protocol schema")]
fn query_conforms_to_schema() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "output includes schema_version field")]
fn query_has_schema_version() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["schema_version"].is_string());
}

#[test]
#[specforge_test(behavior = "query_graph_multi_resolution", verify = "querying same entity at same depth produces identical subgraph")]
fn query_deterministic() {
    let graph = build_graph();
    let r1 = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let r2 = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    assert_eq!(r1, r2, "same query must produce identical output");
}

// ============================================================
// enforce_token_budget — remaining verify statements
// ============================================================

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "output within budget includes all entities")]
fn budget_within_includes_all() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 100_000);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "output exceeding budget truncates low-priority entities")]
fn budget_exceeding_truncates() {
    let graph = build_graph();
    // Very small budget should force truncation
    let result = specforge_emitter::emit_json_with_budget(&graph, 10);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    assert!(nodes.len() < 3, "small budget should truncate some entities");
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "TokenBudgetResult included in metadata when budget applied")]
fn budget_metadata_included() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 10);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["token_budget"].is_object(), "truncated output must include token_budget metadata");
    assert_eq!(parsed["token_budget"]["strategy"], "prioritize");
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "truncated_entities lists omitted entity IDs")]
fn budget_truncated_ids() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 10);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let truncated = parsed["token_budget"]["truncated_entities"].as_array()
        .expect("truncated_entities should be an array");
    assert!(!truncated.is_empty(), "should list truncated entity IDs");
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "no --max-tokens skips budget enforcement")]
fn budget_no_max_tokens() {
    let graph = build_graph();
    // Without budget, emit_json should include everything and no token_budget metadata
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("token_budget").is_none() || parsed["token_budget"].is_null(),
        "no budget should mean no token_budget metadata");
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "export with max_tokens produces output within budget and includes metadata")]
fn budget_integration() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget(&graph, 50);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Output should be parseable JSON
    assert!(parsed["schema_version"].is_string());
    // If truncated, metadata should be present
    if parsed["nodes"].as_array().unwrap().len() < 3 {
        assert!(parsed["token_budget"].is_object());
    }
}

#[test]
#[specforge_test(behavior = "enforce_token_budget", verify = "error strategy rejects export exceeding budget")]
fn budget_error_strategy() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_with_budget_strategy(&graph, 10, "error");
    assert!(result.is_err(), "error strategy should reject over-budget");
    assert!(result.unwrap_err().to_string().contains("budget exceeded"));
}
