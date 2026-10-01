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

/// A node with a description, so tests can check prose is left out.
fn described_node(id: &str) -> Node {
    let mut node = testable_node(id);
    node.fields.push(
        Sym::new("description"),
        FieldValue::String("Long prose that explains the entity at length".to_string()),
    );
    node
}

/// The IDs of the `nodes` array of a JSON export, in output order.
fn node_ids(parsed: &serde_json::Value) -> Vec<&str> {
    parsed["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect()
}

/// Each line of a DOT document with every quoted string replaced by `Q`.
/// Panics on a quote that is not closed on its line or an escape that is
/// not one DOT allows, so a statement split by a stray quote or newline
/// cannot pass.
fn dot_statement_shapes(dot: &str) -> Vec<String> {
    dot.lines()
        .map(|line| {
            let mut shape = String::new();
            let mut chars = line.trim().chars();
            while let Some(ch) = chars.next() {
                if ch != '"' {
                    shape.push(ch);
                    continue;
                }
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('"' | '\\' | 'n') => {}
                            other => panic!("bad escape {other:?} in: {line}"),
                        },
                        Some(_) => {}
                        None => panic!("unterminated quoted string in: {line}"),
                    }
                }
                shape.push('Q');
            }
            shape
        })
        .collect()
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
#[specforge_test(
    behavior = "serialize_json_graph",
    verify = "Serialize JSON Graph: JSON graph serialization holds — validation_complete_fired, all_nodes_serialized, all_edges_serialized, schema_version_present, valid_json_produced, render_complete_emitted"
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
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "Serialize DOT Visualization: DOT visualization holds — validation_complete_fired, valid_dot_produced, nodes_labeled, edges_labeled, render_complete_emitted"
)]
fn dot_contract_finalized_graph_produces_valid_dot() {
    // Requires: graph is finalized
    // Ensures: valid DOT; every node labeled with its ID and title; every
    // edge labeled with its type.
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());

    assert_eq!(
        dot,
        concat!(
            "digraph specforge {\n",
            "  rankdir=LR;\n",
            "  node [shape=box];\n",
            "  \"a\" [label=\"a\\nTitle a\"];\n",
            "  \"b\" [label=\"b\\nTitle b\"];\n",
            "  \"c\" [label=\"c\\nTitle c\"];\n",
            "  \"a\" -> \"b\" [label=\"behaviors\"];\n",
            "  \"b\" -> \"c\" [label=\"depends_on\"];\n",
            "}\n",
        )
    );
    // valid_dot_produced: every line is a statement of the DOT grammar.
    assert_eq!(
        dot_statement_shapes(&dot),
        vec![
            "digraph specforge {",
            "rankdir=LR;",
            "node [shape=box];",
            "Q [label=Q];",
            "Q [label=Q];",
            "Q [label=Q];",
            "Q -> Q [label=Q];",
            "Q -> Q [label=Q];",
            "}",
        ]
    );
}

// === compute_traceability_chain contract ===

/// Expected edges from the test registries: behaviors expect `invariants`
/// and `features` (the latter also declared by a feature's `behaviors`),
/// features expect `behaviors`.
fn trace_expectations() -> specforge_emitter::TraceExpectations {
    let (fields, kinds) = crate::trace_support::registries();
    specforge_emitter::TraceExpectations::from_registries(&fields, &kinds)
}

// No event bus exists in the codebase: like the other emitter contracts,
// the *_emitted clause is not observable here.
#[specforge_test(
    behavior = "compute_traceability_chain",
    verify = "Compute Traceability Chain: traceability chain computation holds — validation_complete_fired, full_chain_traversed, missing_links_flagged, trace_chain_computed_emitted"
)]
fn trace_contract_entity_in_graph_produces_chain() {
    // Requires (validation_complete_fired): a finalized graph.
    // Ensures (full_chain_traversed): the full chain, both directions,
    // every hop.
    let graph = build_graph();
    let expectations = trace_expectations();
    let links = |links: &[specforge_emitter::TraceLink]| -> Vec<(String, String, usize)> {
        links
            .iter()
            .map(|l| {
                assert_eq!(l.status, specforge_emitter::TraceLinkStatus::Resolved);
                (l.entity_id.clone(), l.edge_label.clone(), l.depth)
            })
            .collect()
    };
    let missing = |chain: &specforge_emitter::TraceChain| -> Vec<(String, String, String)> {
        chain
            .missing
            .iter()
            .map(|m| {
                assert_eq!(m.status, specforge_emitter::TraceLinkStatus::Missing);
                (
                    m.from.clone(),
                    m.edge_label.clone(),
                    m.expected_kind.clone(),
                )
            })
            .collect()
    };

    let trace = specforge_emitter::trace_with_expectations(&graph, "b", &expectations).unwrap();
    assert_eq!(trace.entity_id, "b");
    assert_eq!(trace.entity_kind, "behavior");
    assert_eq!(
        links(&trace.upstream),
        vec![("a".to_string(), "behaviors".to_string(), 1)]
    );
    assert_eq!(
        links(&trace.downstream),
        vec![("c".to_string(), "depends_on".to_string(), 1)]
    );

    // Ensures (missing_links_flagged): b's feature link is declared from
    // a's side; b declares no invariant.
    assert_eq!(
        missing(&trace),
        vec![(
            "b".to_string(),
            "invariants".to_string(),
            "invariant".to_string()
        )]
    );

    // From the root the chain reaches the leaf two hops away, and the root
    // has the one edge its kind expects.
    let root = specforge_emitter::trace_with_expectations(&graph, "a", &expectations).unwrap();
    assert!(root.upstream.is_empty());
    assert_eq!(
        links(&root.downstream),
        vec![
            ("b".to_string(), "behaviors".to_string(), 1),
            ("c".to_string(), "depends_on".to_string(), 2),
        ]
    );
    assert!(root.missing.is_empty(), "{:?}", root.missing);

    // c: no feature lists it and it declares nothing its kind expects.
    let leaf = specforge_emitter::trace_with_expectations(&graph, "c", &expectations).unwrap();
    assert_eq!(
        missing(&leaf),
        vec![
            (
                "c".to_string(),
                "features".to_string(),
                "feature".to_string()
            ),
            (
                "c".to_string(),
                "invariants".to_string(),
                "invariant".to_string()
            ),
        ]
    );

    // The JSON carries the Graph Protocol schema_version and the gaps.
    let json: serde_json::Value =
        serde_json::from_str(&specforge_emitter::serialize_trace(&leaf).unwrap()).unwrap();
    assert_eq!(json["schema_version"], specforge_emitter::SCHEMA_VERSION);
    assert_eq!(json["missing"][0]["status"], "missing");
    assert_eq!(json["upstream"][0]["status"], "resolved");
}

// === compute_project_statistics contract ===

// B:compute_project_statistics — verify contract "requires/ensures consistency for project statistics computation"
#[specforge_test(
    behavior = "compute_project_statistics",
    verify = "Compute Project Statistics: project statistics computation holds — validation_complete_fired, entity_counts_produced, coverage_computed, zero_testable_safe"
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

    // One testable behavior without a verify, and a verified feature: the
    // feature is not testable, so it must not count toward coverage.
    let mut graph = graph;
    graph.add_node(node_with_fields("d", "behavior", "unverified", "planned"));
    let mut verified_feature = testable_node("e");
    verified_feature.kind = EntityKind {
        raw: Sym::new("feature"),
    };
    graph.add_node(verified_feature);

    let stats =
        specforge_emitter::compute_stats_with_diagnostics(&graph, &["behavior"], &diagnostics);
    assert_eq!(stats.total_entities, 5);
    assert_eq!(stats.total_edges, 2);
    // entity_counts_produced: counts grouped by kind.
    assert_eq!(
        stats.entities_by_kind,
        [("behavior".to_string(), 3), ("feature".to_string(), 2)]
            .into_iter()
            .collect()
    );
    // coverage_computed: b and c of the three behaviors are verified.
    assert_eq!(stats.testable_count, 3);
    assert!(
        (stats.coverage_pct - 200.0 / 3.0).abs() < 1e-9,
        "2 of 3 testable verified: {}",
        stats.coverage_pct
    );
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.warning_count, 1);

    // zero_testable_safe: no testable kinds, or a testable kind with no
    // entities, reports 0%, not NaN.
    for testable in [&[][..], &["event"][..]] {
        let none = specforge_emitter::compute_stats_with_diagnostics(&graph, testable, &[]);
        assert_eq!(none.testable_count, 0);
        assert_eq!(none.coverage_pct, 0.0, "testable kinds {testable:?}");
    }
}

// === export_agent_context_format contract ===

// B:export_agent_context_format — verify contract "requires/ensures consistency for agent context export"
#[specforge_test(
    behavior = "export_agent_context_format",
    verify = "Export Agent Context Format: agent context export holds — validation_complete_fired, token_optimized_output, schema_version_present, scope_enforced, invalid_scope_diagnosed, export_complete_emitted"
)]
fn context_contract_includes_contracts_and_verify_omits_prose() {
    // Requires: finalized graph
    let mut graph = build_graph();
    graph.add_node(described_node("d")); // disconnected from a -> b -> c
    let json = specforge_emitter::emit_context(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // schema_version_present
    assert_eq!(parsed["schema_version"], "0.1.0");

    // token_optimized_output: contract, status and verify kept; the prose
    // description and source locations left out.
    let nodes = parsed["nodes"].as_array().unwrap();
    let d = nodes.iter().find(|n| n["id"] == "d").unwrap();
    assert_eq!(d["contract"], "The system MUST work");
    assert_eq!(
        d["verify"],
        serde_json::json!([{ "kind": "unit", "description": "it works" }])
    );
    let a = nodes.iter().find(|n| n["id"] == "a").unwrap();
    assert_eq!(a["status"], "planned");
    assert!(
        !json.contains("Long prose"),
        "description text must not appear: {json}"
    );
    for node in nodes {
        assert!(node.get("description").is_none(), "{node}");
        assert!(node.get("file").is_none(), "{node}");
    }

    // scope_enforced: scoped at a, only the connected a, b, c.
    let scoped = specforge_emitter::emit_context_scoped(&graph, "a").unwrap();
    let scoped: serde_json::Value = serde_json::from_str(&scoped).unwrap();
    assert_eq!(node_ids(&scoped), vec!["a", "b", "c"]);
    assert_eq!(scoped["edges"].as_array().unwrap().len(), 2);

    // invalid_scope_diagnosed: E003 naming the entity, exit code 1 — through
    // the call `specforge export --format context --scope` makes.
    let err = specforge_emitter::emit(
        &graph,
        &specforge_emitter::EmitOptions {
            format: specforge_emitter::EmitFormat::Context,
            scope: Some("ghost"),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "E003: unresolved scope entity 'ghost' — entity not found in graph"
    );
    assert_eq!(err.exit_code(), 1);
}

// === export_agent_graph_format contract ===

// B:export_agent_graph_format — verify contract "requires/ensures consistency for agent graph export"
#[specforge_test(
    behavior = "export_agent_graph_format",
    verify = "Export Agent Graph Format: agent graph export holds — validation_complete_fired, full_fidelity_output, schema_version_present, scope_enforced, invalid_scope_diagnosed, export_complete_emitted"
)]
fn graph_format_contract_finalized_graph_produces_full_output() {
    // Requires: finalized graph
    let mut graph = build_graph();
    graph.add_node(described_node("d")); // disconnected from a -> b -> c
    let json = specforge_emitter::emit_graph(&graph);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // schema_version_present
    assert_eq!(parsed["schema_version"], "0.1.0");

    // full_fidelity_output: every node with every field and its location,
    // every edge.
    assert_eq!(node_ids(&parsed), vec!["a", "b", "c", "d"]);
    let d = &parsed["nodes"][3];
    assert_eq!(d["kind"], "behavior");
    assert_eq!(d["title"], "Title d");
    assert_eq!(d["file"], "test.spec");
    assert_eq!(d["line"], 1);
    assert_eq!(
        d["fields"],
        serde_json::json!({
            "contract": "The system MUST work",
            "verify": [{ "kind": "unit", "description": "it works" }],
            "description": "Long prose that explains the entity at length",
        })
    );
    assert_eq!(parsed["nodes"][0]["fields"]["status"], "planned");
    assert_eq!(
        parsed["edges"],
        serde_json::json!([
            { "source": "a", "target": "b", "label": "behaviors" },
            { "source": "b", "target": "c", "label": "depends_on" },
        ])
    );

    // scope_enforced: scoped at c, only the connected a, b, c.
    let scoped = specforge_emitter::emit_json_scoped(&graph, "c").unwrap();
    let scoped: serde_json::Value = serde_json::from_str(&scoped).unwrap();
    assert_eq!(node_ids(&scoped), vec!["a", "b", "c"]);
    assert_eq!(scoped["edges"].as_array().unwrap().len(), 2);

    // invalid_scope_diagnosed: E003 naming the entity, exit code 1 — through
    // the call `specforge export --format graph --scope` makes.
    let err = specforge_emitter::emit(
        &graph,
        &specforge_emitter::EmitOptions {
            scope: Some("ghost"),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "E003: unresolved scope entity 'ghost' — entity not found in graph"
    );
    assert_eq!(err.exit_code(), 1);
}

// === query_graph_multi_resolution contract ===

// B:query_graph_multi_resolution — verify contract "requires/ensures consistency for multi-resolution graph query"
#[specforge_test(
    behavior = "query_graph_multi_resolution",
    verify = "Query Graph at Multiple Resolutions: multi-resolution graph query holds — validation_complete_fired, depth_respected, kind_filter_applied, graph_protocol_conformance, graph_queried_emitted"
)]
fn query_contract_valid_entity_returns_subgraph() {
    // Requires: entity exists in graph, depth >= 0
    // a(feature) -> b(behavior) -> c(behavior) -> x(invariant)
    let mut graph = build_graph();
    graph.add_node(node_with_fields("x", "invariant", "holds", "active"));
    graph.add_edge(Edge {
        source: "c".into(),
        target: "x".into(),
        label: "invariants".into(),
    });
    let query = |depth: usize, kinds: &[&str]| -> serde_json::Value {
        let out = specforge_emitter::query(&graph, "a", depth, kinds).unwrap();
        serde_json::from_str(&out).unwrap()
    };

    // depth_respected: exactly the entities within N hops.
    assert_eq!(node_ids(&query(0, &[])), vec!["a"]);
    assert_eq!(node_ids(&query(1, &[])), vec!["a", "b"]);
    assert_eq!(node_ids(&query(2, &[])), vec!["a", "b", "c"]);
    assert_eq!(node_ids(&query(3, &[])), vec!["a", "b", "c", "x"]);

    // kind_filter_applied: only the listed kinds, plus the queried root.
    assert_eq!(node_ids(&query(3, &["behavior"])), vec!["a", "b", "c"]);
    assert_eq!(node_ids(&query(3, &["invariant"])), vec!["a", "x"]);

    // graph_protocol_conformance: schema_version, and edges only between
    // returned nodes.
    let result = query(2, &[]);
    assert_eq!(result["schema_version"], "0.1.0");
    assert_eq!(
        result["edges"],
        serde_json::json!([
            { "source": "a", "target": "b", "label": "behaviors" },
            { "source": "b", "target": "c", "label": "depends_on" },
        ])
    );
}

// === enforce_token_budget contract ===

// B:enforce_token_budget — verify contract "requires/ensures consistency for token budget enforcement"
#[specforge_test(
    behavior = "enforce_token_budget",
    verify = "Enforce Token Budget: token budget enforcement holds — validation_complete_fired, budget_respected, truncation_metadata_produced, valid_subgraph_after_truncation, token_budget_applied_emitted"
)]
fn budget_contract_within_budget_no_truncation() {
    // Requires: graph + budget
    let graph = build_graph(); // a -> b -> c: b is the most central
    let emit = |budget: usize| -> serde_json::Value {
        serde_json::from_str(&specforge_emitter::emit_json_with_budget(&graph, budget).unwrap())
            .unwrap()
    };

    // Within budget: everything, and no truncation metadata.
    let roomy = emit(100_000);
    assert_eq!(node_ids(&roomy), vec!["a", "b", "c"]);
    assert!(roomy.get("token_budget").is_none(), "{roomy}");

    // Over budget: the least central entity (a, degree 1, before c by id)
    // goes first.
    let tight = emit(180);
    let meta = &tight["token_budget"];
    // truncation_metadata_produced
    assert_eq!(meta["strategy"], "prioritize");
    assert_eq!(meta["budget_tokens"], 180);
    assert_eq!(meta["truncated_entities"], serde_json::json!(["a"]));
    // Kept entities are listed most central last; compare as sets.
    let kept = |v: &serde_json::Value| -> Vec<String> {
        let mut ids: Vec<String> = node_ids(v).iter().map(|s| s.to_string()).collect();
        ids.sort();
        ids
    };
    assert_eq!(kept(&tight), vec!["b", "c"]);
    // budget_respected
    let estimated = meta["estimated_tokens"].as_u64().unwrap() as usize;
    assert!(estimated <= 180, "estimated {estimated} over budget 180");
    // The estimate is the output's real cost: a budget of exactly that keeps
    // the same entities, one token less has to drop another.
    assert_eq!(kept(&emit(estimated)), vec!["b", "c"]);
    assert_eq!(kept(&emit(estimated - 1)), vec!["b"]);
    // valid_subgraph_after_truncation: no edge to or from a.
    assert_eq!(
        tight["edges"],
        serde_json::json!([{ "source": "b", "target": "c", "label": "depends_on" }])
    );
}

// === validate_agent_plan contract ===

// B:validate_agent_plan — verify contract "requires/ensures consistency for agent plan validation"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "Validate Agent Implementation Plan: agent plan validation holds — validation_complete_fired, unresolvable_ids_diagnosed, missing_entries_warned, ordering_validated, structured_report_produced, plan_validated_emitted"
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
#[specforge_test(
    behavior = "deterministic_output",
    verify = "Deterministic Output: deterministic output holds — validation_complete_fired, byte_identical_output, no_nondeterministic_values"
)]
fn deterministic_contract_same_input_identical_output() {
    // Requires: same graph input
    // Ensures: identical output across all formats
    let graph = build_graph();

    let json1 = specforge_emitter::emit_json(&graph);
    let json2 = specforge_emitter::emit_json(&graph);
    assert_eq!(json1, json2, "JSON must be deterministic");

    let dot1 = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());
    let dot2 = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());
    assert_eq!(dot1, dot2, "DOT must be deterministic");

    let brief1 = specforge_emitter::emit_brief(&graph);
    let brief2 = specforge_emitter::emit_brief(&graph);
    assert_eq!(brief1, brief2, "brief must be deterministic");

    let ctx1 = specforge_emitter::emit_context(&graph);
    let ctx2 = specforge_emitter::emit_context(&graph);
    assert_eq!(ctx1, ctx2, "context must be deterministic");
}

// === serialize_traceability_data contract ===

// No event bus exists in the codebase: like the other emitter contracts,
// the *_emitted clause is not observable here.
#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "Serialize Traceability Data: traceability data serialization holds — validation_complete_fired, full_trace_serialized, gaps_included, graph_protocol_conformance, render_complete_emitted"
)]
fn trace_data_contract_all_entities_traced() {
    // Requires (validation_complete_fired): a finalized graph.
    let graph = build_graph();
    let traces = specforge_emitter::trace_all_with_expectations(&graph, &trace_expectations());
    let json = specforge_emitter::serialize_trace_all(&traces).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    // graph_protocol_conformance: the protocol's schema_version, and a
    // traces array.
    assert_eq!(parsed["schema_version"], specforge_emitter::SCHEMA_VERSION);
    let traces = parsed["traces"].as_array().unwrap();

    // full_trace_serialized: one chain per entity, in ID order, each with
    // its links from root to leaf.
    let ids: Vec<&str> = traces
        .iter()
        .map(|t| t["entity_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
    let hops = |trace: &serde_json::Value, direction: &str| -> Vec<(String, u64, String)> {
        trace[direction]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                (
                    l["entity_id"].as_str().unwrap().to_string(),
                    l["depth"].as_u64().unwrap(),
                    l["status"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(
        hops(&traces[0], "downstream"),
        vec![
            ("b".to_string(), 1, "resolved".to_string()),
            ("c".to_string(), 2, "resolved".to_string()),
        ]
    );
    assert_eq!(
        hops(&traces[2], "upstream"),
        vec![
            ("b".to_string(), 1, "resolved".to_string()),
            ("a".to_string(), 2, "resolved".to_string()),
        ]
    );

    // gaps_included: every missing link, with a missing status, from the
    // entity that lacks it to the kind it should reach.
    let gaps: Vec<(String, String, String, String)> = traces
        .iter()
        .flat_map(|t| t["missing"].as_array().unwrap().iter())
        .map(|m| {
            (
                m["from"].as_str().unwrap().to_string(),
                m["edge_label"].as_str().unwrap().to_string(),
                m["expected_kind"].as_str().unwrap().to_string(),
                m["status"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let gap = |from: &str, label: &str, kind: &str| {
        (
            from.to_string(),
            label.to_string(),
            kind.to_string(),
            "missing".to_string(),
        )
    };
    assert_eq!(
        gaps,
        vec![
            gap("b", "invariants", "invariant"),
            gap("c", "features", "feature"),
            gap("c", "invariants", "invariant"),
        ]
    );
}

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

#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "gaps in chain are highlighted"
)]
fn trace_data_gaps_highlighted() {
    // A behavior linked to nothing: both edges its kind expects are gaps
    // in the full trace, with the registered edge type each would be.
    let mut graph = Graph::new();
    graph.add_node(testable_node("isolated"));
    let traces = specforge_emitter::trace_all_with_expectations(&graph, &trace_expectations());
    let json = specforge_emitter::serialize_trace_all(&traces).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed["traces"][0]["missing"],
        serde_json::json!([
            {
                "from": "isolated",
                "from_kind": "behavior",
                "edge_label": "features",
                "edge_type": "behavior_features",
                "expected_kind": "feature",
                "depth": 1,
                "required": false,
                "status": "missing"
            },
            {
                "from": "isolated",
                "from_kind": "behavior",
                "edge_label": "invariants",
                "edge_type": "behavior_invariants",
                "expected_kind": "invariant",
                "depth": 1,
                "required": false,
                "status": "missing"
            }
        ])
    );

    // Without expectations nothing is missing.
    let bare = specforge_emitter::trace_all(&graph);
    assert!(bare[0].missing.is_empty());
}

#[specforge_test(
    behavior = "serialize_traceability_data",
    verify = "output conforms to Graph Protocol schema"
)]
fn trace_data_output_conforms_to_schema() {
    let graph = build_graph();
    let traces = specforge_emitter::trace_all(&graph);
    let json = specforge_emitter::serialize_trace_all(&traces).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
    for trace in parsed["traces"].as_array().unwrap() {
        assert!(trace["entity_id"].is_string());
        assert!(trace["upstream"].is_array());
        assert!(trace["downstream"].is_array());
    }
}

// === present_diagnostics_as_json contract ===

// B:present_diagnostics_as_json — verify contract
#[specforge_test(
    behavior = "present_diagnostics_as_json",
    verify = "Present Diagnostics as JSON: JSON diagnostic presentation holds — diagnostics_collected, one_shape_everywhere, location_both_ways"
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

#[specforge_test(
    behavior = "present_diagnostics_as_json",
    verify = "diagnostics are presented as one JSON array"
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

#[specforge_test(
    behavior = "present_diagnostics_as_json",
    verify = "each diagnostic carries code, severity, message, file, line and column"
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

#[specforge_test(
    behavior = "present_diagnostics_as_json",
    verify = "the presented JSON is valid and parseable"
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

#[specforge_test(
    behavior = "present_diagnostics_as_json",
    verify = "suggestion is included when available"
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

#[specforge_test(behavior = "serialize_json_graph", verify = "output is valid JSON")]
fn json_graph_valid_json() {
    let graph = build_graph();
    let json = specforge_emitter::emit_json(&graph);
    let result: Result<serde_json::Value, _> = serde_json::from_str(&json);
    assert!(result.is_ok(), "output must be valid JSON");
}

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

#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "DOT output is valid Graphviz syntax"
)]
fn dot_valid_syntax() {
    // IDs and titles with quotes, backslashes and newlines must stay inside
    // their quoted strings.
    let mut graph = build_graph();
    let mut hostile = testable_node("say \"hi\"");
    hostile.title = Some("back\\slash }\n\"; digraph x {".to_string());
    graph.add_node(hostile);
    graph.add_edge(Edge {
        source: "a".into(),
        target: "say \"hi\"".into(),
        label: "behaviors".into(),
    });
    let dot = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());

    assert_eq!(
        dot_statement_shapes(&dot),
        vec![
            "digraph specforge {",
            "rankdir=LR;",
            "node [shape=box];",
            "Q [label=Q];",
            "Q [label=Q];",
            "Q [label=Q];",
            "Q [label=Q];",
            "Q -> Q [label=Q];",
            "Q -> Q [label=Q];",
            "Q -> Q [label=Q];",
            "}",
        ],
        "{dot}"
    );
    assert!(
        dot.contains(
            "  \"say \\\"hi\\\"\" [label=\"say \\\"hi\\\"\\nback\\\\slash }\\n\\\"; digraph x {\"];\n"
        ),
        "{dot}"
    );
    assert!(
        dot.contains("  \"a\" -> \"say \\\"hi\\\"\" [label=\"behaviors\"];\n"),
        "{dot}"
    );
}

#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "nodes are labeled with IDs"
)]
fn dot_nodes_labeled() {
    let mut graph = build_graph();
    // An untitled node with no edges: labeled with its ID alone.
    let mut bare = testable_node("d");
    bare.title = None;
    graph.add_node(bare);
    let dot = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());

    // Each node has its own statement, labeled with its ID (and title).
    let statements: Vec<&str> = dot
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('"') && !l.contains("->"))
        .collect();
    assert_eq!(
        statements,
        vec![
            "\"a\" [label=\"a\\nTitle a\"];",
            "\"b\" [label=\"b\\nTitle b\"];",
            "\"c\" [label=\"c\\nTitle c\"];",
            "\"d\" [label=\"d\"];",
        ]
    );
}

#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "edges are labeled with types"
)]
fn dot_edges_labeled() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());
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
fn dot_node_shapes() {
    let graph = build_graph();
    let dot = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());
    assert!(dot.contains("shape="), "nodes must have shape attribute");
}

// C13-00: registry-declared dot_shape/dot_color/dot_fillcolor must reach the
// emitted DOT; kinds without declarations keep the default.
#[specforge_test(
    behavior = "serialize_dot_visualization",
    verify = "node shapes use extension-defined dot_shape"
)]
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

    let dot = specforge_emitter::emit_dot(
        &graph,
        &specforge_emitter::DotOptions {
            kind_registry: Some(&registry),
            ..Default::default()
        },
    );
    assert!(
        dot.contains("shape=\"hexagon\""),
        "registry shape emitted: {dot}"
    );
    assert!(
        dot.contains("color=\"firebrick\""),
        "registry color emitted"
    );
    assert!(
        dot.contains("fillcolor=\"#ffeeee\""),
        "registry fillcolor emitted"
    );

    // No registry: default emission (no per-node shape overrides).
    let plain = specforge_emitter::emit_dot(&graph, &specforge_emitter::DotOptions::default());
    assert!(
        !plain.contains("shape=\"hexagon\""),
        "default emission must not invent registry styles"
    );
}

// C9-04: machine formats serialize compact — no pretty-print whitespace.
#[test]
fn machine_formats_serialize_compact() {
    let graph = build_graph();
    let options = specforge_emitter::EmitOptions {
        format: specforge_emitter::EmitFormat::Json,
        ..Default::default()
    };
    let json = specforge_emitter::emit(&graph, &options).unwrap();
    assert!(
        !json.contains("\n  \""),
        "machine JSON must not contain pretty-print indentation"
    );
    let pretty_bytes = serde_json::to_string_pretty(
        &serde_json::from_slice::<serde_json::Value>(json.as_bytes()).unwrap(),
    )
    .unwrap()
    .len();
    assert!(
        json.len() < pretty_bytes,
        "compact output must be smaller than pretty output"
    );

    let ctx = specforge_emitter::EmitOptions {
        format: specforge_emitter::EmitFormat::Context,
        ..Default::default()
    };
    let context = specforge_emitter::emit(&graph, &ctx).unwrap();
    assert!(!context.contains("\n  \""), "context must be compact too");

    let brief = specforge_emitter::EmitOptions {
        format: specforge_emitter::EmitFormat::Brief,
        ..Default::default()
    };
    let brief_out = specforge_emitter::emit(&graph, &brief).unwrap();
    assert!(!brief_out.contains("\n  \""), "brief must be compact too");
}

// C1-10: token budget applies to the agent formats (context/brief), not just
// schemaless JSON. A tight budget must shrink the output to a subgraph.
#[test]
fn budget_truncates_context_and_brief() {
    let graph = build_graph(); // 3 nodes, 2 edges
    for format in [
        specforge_emitter::EmitFormat::Json,
        specforge_emitter::EmitFormat::Context,
        specforge_emitter::EmitFormat::Brief,
    ] {
        let full = specforge_emitter::emit(
            &graph,
            &specforge_emitter::EmitOptions {
                format,
                ..Default::default()
            },
        )
        .unwrap();
        // Far below the full render; the graph export gets just under it,
        // since it can't shrink past its envelope (E062 below that).
        let budget = match format {
            specforge_emitter::EmitFormat::Json => specforge_emitter::estimate_tokens(&full) - 1,
            _ => 20,
        };
        let truncated = specforge_emitter::emit(
            &graph,
            &specforge_emitter::EmitOptions {
                format,
                token_budget: Some(budget),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            truncated.len() < full.len(),
            "{format:?}: budgeted output must be strictly smaller ({} vs {})",
            truncated.len(),
            full.len()
        );
        // Budgeted output stays valid JSON for the JSON family.
        if matches!(format, specforge_emitter::EmitFormat::Json) {
            serde_json::from_str::<serde_json::Value>(&truncated)
                .expect("budgeted JSON still parses");
        }
    }

    // No budget: unchanged full output.
    let full = specforge_emitter::emit(
        &graph,
        &specforge_emitter::EmitOptions {
            format: specforge_emitter::EmitFormat::Brief,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(full.contains("\"edges\":"), "full brief keeps edges");
}
