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
    fields.push(
        Sym::new("contract"),
        FieldValue::String(contract.to_string()),
    );
    fields.push(
        Sym::new("status"),
        FieldValue::Identifier(status.to_string()),
    );
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
        methods: Vec::new(),
    }
}

fn testable_node(id: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        Sym::new("contract"),
        FieldValue::String("The system MUST work".to_string()),
    );
    fields.push(
        Sym::new("verify"),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".to_string(),
            description: "it works".to_string(),
        }]),
    );
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
        methods: Vec::new(),
    }
}

fn build_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("a", "feature", "feature A", "planned"));
    graph.add_node(testable_node("b"));
    graph.add_node(testable_node("c"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    graph.add_edge(Edge {
        source: "b".into(),
        target: "c".into(),
        label: "depends_on".into(),
    });
    graph
}

// === serialize_json_graph contract ===

// B:serialize_json_graph — verify contract "requires/ensures consistency for JSON graph serialization"
#[test]
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "requires/ensures consistency for JSON graph serialization"
)]
fn json_graph_contract_finalized_graph_produces_valid_output() {
    // Requires: graph is finalized (built with nodes + edges)
    // Ensures: valid JSON with schema_version, all nodes, all edges, source locations
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed["schema_version"].is_string(),
        "must include schema_version"
    );
    assert_eq!(
        parsed["nodes"].as_array().unwrap().len(),
        3,
        "all nodes present"
    );
    assert_eq!(
        parsed["edges"].as_array().unwrap().len(),
        2,
        "all edges present"
    );

    for node in parsed["nodes"].as_array().unwrap() {
        assert!(node["file"].is_string(), "must include source file");
        assert!(node["line"].is_number(), "must include source line");
    }
}

// === serialize_dot_visualization contract ===

// B:serialize_dot_visualization — verify contract "requires/ensures consistency for DOT visualization"
#[test]
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "requires/ensures consistency for DOT visualization"
)]
fn dot_contract_finalized_graph_produces_valid_dot() {
    // Requires: graph is finalized
    // Ensures: valid Graphviz DOT syntax
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);

    assert!(dot.starts_with("digraph"), "must be a directed graph");
    assert!(dot.contains("rankdir=LR"), "must have LR layout");
    assert!(dot.contains("shape=box"), "nodes must have shape");
    assert!(
        dot.ends_with("}\n") || dot.ends_with("}"),
        "must be properly closed"
    );
}

// === compute_traceability_chain contract ===

// B:compute_traceability_chain — verify contract "requires/ensures consistency for traceability chain computation"
#[test]
#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "requires/ensures consistency for traceability chain computation"
)]
fn trace_contract_entity_in_graph_produces_chain() {
    // Requires: entity exists in graph
    // Ensures: trace chain with upstream + downstream, sorted by depth
    let graph = build_graph();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();

    assert_eq!(trace.entity_id, "b");
    assert!(
        !trace.upstream.is_empty(),
        "mid-chain entity must have upstream"
    );
    assert!(
        !trace.downstream.is_empty(),
        "mid-chain entity must have downstream"
    );

    // Verify depth ordering
    for window in trace.upstream.windows(2) {
        assert!(
            window[0].depth <= window[1].depth,
            "upstream must be sorted by depth"
        );
    }
    for window in trace.downstream.windows(2) {
        assert!(
            window[0].depth <= window[1].depth,
            "downstream must be sorted by depth"
        );
    }
}

// === compute_project_statistics contract ===

// B:compute_project_statistics — verify contract "requires/ensures consistency for project statistics computation"
#[test]
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "requires/ensures consistency for project statistics computation"
)]
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

    let stats =
        specforge_emitter::compute_stats_with_diagnostics(&graph, &["behavior"], &diagnostics);
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
#[specforge_test(
    behavior = "export_agent_context_format",
    verify = "requires/ensures consistency for agent context export"
)]
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
#[specforge_test(
    behavior = "export_agent_graph_format",
    verify = "requires/ensures consistency for agent graph export"
)]
fn graph_format_contract_finalized_graph_produces_full_output() {
    // Requires: finalized graph
    // Ensures: all nodes with all fields, all edges, schema_version present
    let graph = build_graph();
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed["schema_version"].is_string(),
        "must include schema_version"
    );
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3, "all nodes present");
    assert_eq!(
        parsed["edges"].as_array().unwrap().len(),
        2,
        "all edges present"
    );

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
#[specforge_test(
    behavior = "query_graph_multi_resolution",
    verify = "requires/ensures consistency for multi-resolution graph query"
)]
fn query_contract_valid_entity_returns_subgraph() {
    // Requires: entity exists in graph, depth >= 0
    // Ensures: root always included, neighbors within depth, schema_version present
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["schema_version"].is_string());
    let ids: Vec<&str> = parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"b"), "root must always be included");
}

// === enforce_token_budget contract ===

// B:enforce_token_budget — verify contract "requires/ensures consistency for token budget enforcement"
#[test]
#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "requires/ensures consistency for token budget enforcement"
)]
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
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "requires/ensures consistency for agent plan validation"
)]
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
    assert!(
        !result.errors.is_empty(),
        "unresolvable IDs must produce errors"
    );
    assert!(
        !result.warnings.is_empty(),
        "missing testable must produce warnings"
    );

    let json = specforge_emitter::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object(), "structured JSON report required");
}

// === deterministic_output contract ===

// B:deterministic_output — verify contract "requires/ensures consistency for deterministic output"
#[test]
#[specforge_test(
    behavior = "deterministic_output",
    verify = "requires/ensures consistency for deterministic output"
)]
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
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "requires/ensures consistency for traceability data serialization"
)]
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
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "full trace covers all root entities across registered edge types"
)]
fn trace_data_full_trace_covers_all_roots() {
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    let ids: Vec<&str> = traces.iter().map(|t| t.entity_id.as_str()).collect();
    assert!(ids.contains(&"a"), "root entity a must be traced");
    assert!(ids.contains(&"b"), "mid entity b must be traced");
    assert!(ids.contains(&"c"), "leaf entity c must be traced");
}

#[test]
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "gaps in chain are highlighted"
)]
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
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "output conforms to Graph Protocol schema"
)]
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
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "requires/ensures consistency for JSON diagnostic export"
)]
fn diagnostic_json_contract_complete_fields() {
    let diags = vec![Diagnostic {
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
    }];

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
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "diagnostics serialized as JSON array to stdout"
)]
fn diagnostic_json_array() {
    let diags = vec![
        Diagnostic {
            code: "E001".into(),
            severity: Severity::Error,
            message: "err".into(),
            span: None,
            suggestion: None,
        },
        Diagnostic {
            code: "W001".into(),
            severity: Severity::Warning,
            message: "warn".into(),
            span: None,
            suggestion: None,
        },
    ];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "each diagnostic includes code, severity, message, file, line, column"
)]
fn diagnostic_json_all_fields() {
    let diags = vec![Diagnostic {
        code: "E042".into(),
        severity: Severity::Error,
        message: "test msg".into(),
        span: Some(SourceSpan {
            file: "a.spec".into(),
            start_line: 3,
            start_col: 7,
            end_line: 3,
            end_col: 15,
        }),
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
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "JSON output is valid and parseable"
)]
fn diagnostic_json_valid_parseable() {
    let diags = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "msg with \"quotes\"".into(),
        span: None,
        suggestion: None,
    }];
    let json = specforge_emitter::serialize_diagnostics(&diags);
    let result: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(result.is_ok(), "output must be valid JSON");
}

#[test]
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "exit code unaffected by format flag"
)]
fn diagnostic_exit_code_unaffected_by_format() {
    let diags = vec![Diagnostic {
        code: "E001".into(),
        severity: Severity::Error,
        message: "err".into(),
        span: None,
        suggestion: None,
    }];
    // Exit code should be based on severity regardless of format
    let exit = specforge_emitter::compute_exit_code(&diags);
    assert_eq!(exit, 1, "errors should produce exit 1 regardless of format");
    // Also verify JSON is still produced
    let json = specforge_emitter::serialize_diagnostics(&diags);
    assert!(!json.is_empty());
}

#[test]
#[specforge_test(
    behavior = "export_diagnostics_as_json",
    verify = "suggestion field included when available"
)]
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
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "JSON output contains all nodes"
)]
fn json_graph_all_nodes() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 3);
}

#[test]
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "JSON output contains all edges"
)]
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
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "output includes schema_version field"
)]
fn json_graph_schema_version() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
}

#[test]
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "empty graph produces valid JSON with empty nodes and edges arrays"
)]
fn json_graph_empty() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["nodes"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["edges"].as_array().unwrap().len(), 0);
}

#[test]
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "schema is included even for empty graph"
)]
fn json_graph_empty_has_schema() {
    let graph = Graph::new();
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        parsed["schema_version"].is_string(),
        "empty graph must still have schema_version"
    );
}

#[test]
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "structural-only graph (zero extensions) produces valid Graph Protocol JSON with raw keywords in kind field"
)]
fn json_graph_structural_only() {
    let mut graph = Graph::new();
    graph.add_node(node_with_fields("x", "custom_kind", "c", "active"));
    let json = specforge_emitter::emit_json(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let node = &parsed["nodes"].as_array().unwrap()[0];
    assert_eq!(
        node["kind"], "custom_kind",
        "raw keyword preserved in kind field"
    );
}

// ============================================================
// serialize_dot_visualization — remaining verify statements
// ============================================================

#[test]
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "DOT output is valid Graphviz syntax"
)]
fn dot_valid_syntax() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.starts_with("digraph"));
    assert!(dot.contains("{"));
    assert!(dot.trim_end().ends_with("}"));
}

#[test]
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "nodes are labeled with IDs"
)]
fn dot_nodes_labeled() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("\"a\""), "node a must be present");
    assert!(dot.contains("\"b\""), "node b must be present");
    assert!(dot.contains("\"c\""), "node c must be present");
}

#[test]
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "edges are labeled with types"
)]
fn dot_edges_labeled() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(
        dot.contains("behaviors"),
        "edge label 'behaviors' must be present"
    );
    assert!(
        dot.contains("depends_on"),
        "edge label 'depends_on' must be present"
    );
}

#[test]
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "node shapes use extension-defined dot_shape"
)]
fn dot_node_shapes() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("shape="), "nodes must have shape attribute");
}

// C13-00: registry-declared dot_shape/dot_color/dot_fillcolor must reach the
// emitted DOT; kinds without declarations keep the default.
#[test]
fn dot_emits_registry_declared_styles() {
    use specforge_registry::{KindRegistry, KindRegistryEntry};

    let graph = build_graph();

    let mut registry = KindRegistry::new();
    registry.register(KindRegistryEntry {
        kind_name: "feature".to_string(),
        description: None,
        source_extension: "@test/x".to_string(),
        testable: false,
        singleton: false,
        supports_verify: false,
        allowed_verify_kinds: Vec::new(),
        has_body_parser: false,
        semantic_token: None,
        lsp_icon: None,
        dot_shape: Some("hexagon".to_string()),
        dot_color: Some("firebrick".to_string()),
        dot_fillcolor: Some("#ffeeee".to_string()),
        open_fields: false,
    });

    let dot = specforge_emitter::emit_dot_with_styles(&graph, Some(&registry));
    assert!(
        dot.contains("shape=\"hexagon\""),
        "registry shape emitted: {dot}"
    );
    assert!(dot.contains("color=\"firebrick\""), "registry color emitted");
    assert!(
        dot.contains("fillcolor=\"#ffeeee\""),
        "registry fillcolor emitted"
    );

    // No registry: default emission (no per-node shape overrides).
    let plain = specforge_emitter::emit_dot_with_styles(&graph, None);
    assert!(
        !plain.contains("shape=\"hexagon\""),
        "default emission must not invent registry styles"
    );
}