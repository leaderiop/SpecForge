use specforge_common::{Diagnostic, Severity, SourceSpan};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".to_string(),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node_with_fields(id: &str, kind: &str, contract: &str, status: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push("contract".to_string(), FieldValue::String(contract.to_string()));
    fields.push("status".to_string(), FieldValue::Identifier(status.to_string()));
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
    }
}

fn testable_node(id: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push("contract".to_string(), FieldValue::String("The system MUST work".to_string()));
    fields.push("verify".to_string(), FieldValue::VerifyList(vec![
        VerifyStatement { kind: "unit".to_string(), description: "it works".to_string() },
    ]));
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
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

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
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

// === query_graph_multi_resolution contract ===

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
fn trace_data_contract_all_entities_traced() {
    // Requires: finalized graph
    // Ensures: trace_all produces one trace per entity, serialized with schema_version
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    assert_eq!(traces.len(), graph.nodes().len(), "one trace per entity");

    let json = specforge_emitter::serialize_trace_all(&traces);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
}

// === export_diagnostics_as_json contract ===

#[test]
fn diagnostic_json_contract_complete_fields() {
    // Requires: diagnostics collected
    // Ensures: JSON array, each entry has code/severity/message/file/line/column
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
