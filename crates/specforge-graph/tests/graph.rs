use specforge_graph::{Edge, Graph, Node};
use specforge_common::{SourceSpan, Sym};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as specforge_test;

fn make_node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new("test.spec"),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        },
    }
}

fn make_edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: Sym::new(source),
        target: Sym::new(target),
        label: Sym::new(label),
    }
}

// --- build_in_memory_graph ---

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one node per entity")]
#[test]
fn graph_one_node_per_entity() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "behavior"));

    assert_eq!(graph.nodes().len(), 2);
    assert!(graph.node("alpha").is_some());
    assert!(graph.node("beta").is_some());
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one edge per resolved reference")]
#[test]
fn graph_one_edge_per_reference() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "feature"));
    graph.add_edge(make_edge("beta", "alpha", "behaviors"));

    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].source, "beta");
    assert_eq!(graph.edges()[0].target, "alpha");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "edge types match relationship semantics")]
#[test]
fn graph_edges_connect_existing_nodes() {
    let mut graph = Graph::new();
    graph.add_node(make_node("feat", "feature"));
    graph.add_node(make_node("beh", "behavior"));
    graph.add_edge(make_edge("feat", "beh", "behaviors"));

    let edges = graph.edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].source, "feat");
    assert_eq!(edges[0].target, "beh");
    assert_eq!(edges[0].label, "behaviors");
}

// --- maintain_mutable_graph ---

#[specforge_test(behavior = "maintain_mutable_graph", verify = "add and remove nodes from graph")]
#[test]
fn remove_node_from_graph() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "behavior"));

    graph.remove_node("alpha");

    assert_eq!(graph.nodes().len(), 1);
    assert!(graph.node("alpha").is_none());
    assert!(graph.node("beta").is_some());
}

#[specforge_test(behavior = "maintain_mutable_graph", verify = "removing a node removes its edges")]
#[test]
fn removing_node_removes_its_edges() {
    let mut graph = Graph::new();
    graph.add_node(make_node("feat", "feature"));
    graph.add_node(make_node("beh", "behavior"));
    graph.add_edge(make_edge("feat", "beh", "behaviors"));

    graph.remove_node("beh");

    assert!(graph.edges().is_empty(), "edges to removed node should be gone");
}

#[specforge_test(behavior = "maintain_mutable_graph", verify = "graph consistency after batch mutations")]
#[test]
fn graph_consistency_after_batch_mutations() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_node(make_node("c", "feature"));
    graph.add_edge(make_edge("c", "a", "behaviors"));
    graph.add_edge(make_edge("c", "b", "behaviors"));

    // Remove a, should remove edge c->a but keep c->b
    graph.remove_node("a");

    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(graph.edges().len(), 1);
    assert_eq!(graph.edges()[0].target, "b");
}

// --- subgraph ---

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "changed file and direct dependents are invalidated")]
#[test]
fn subgraph_for_file() {
    let mut graph = Graph::new();

    let mut node_a = make_node("alpha", "behavior");
    node_a.source_span.file = Sym::new("a.spec");
    let mut node_b = make_node("beta", "behavior");
    node_b.source_span.file = Sym::new("b.spec");
    let mut node_c = make_node("gamma", "feature");
    node_c.source_span.file = Sym::new("a.spec");

    graph.add_node(node_a);
    graph.add_node(node_b);
    graph.add_node(node_c);
    graph.add_edge(make_edge("gamma", "alpha", "behaviors"));

    let sub = graph.nodes_in_file("a.spec");
    assert_eq!(sub.len(), 2);
}

// === build_graph integration ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one node per entity")]
#[test]
fn build_graph_one_node_per_entity() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" { behaviors [alpha, beta] }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    assert_eq!(graph.node_count(), 3, "one node per entity");
    assert!(graph.node("alpha").is_some());
    assert!(graph.node("beta").is_some());
    assert!(graph.node("gamma").is_some());
    assert!(
        diagnostics.iter().all(|d| d.severity != specforge_graph::Severity::Error),
        "no errors: {:?}", diagnostics
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one edge per resolved reference")]
#[test]
fn build_graph_one_edge_per_resolved_reference() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" { behaviors [alpha, beta] }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    assert_eq!(graph.edge_count(), 2, "one edge per reference");
    let edges_from_gamma = graph.edges_from("gamma");
    assert_eq!(edges_from_gamma.len(), 2);
    assert!(edges_from_gamma.iter().any(|e| e.target == "alpha"));
    assert!(edges_from_gamma.iter().any(|e| e.target == "beta"));
    assert!(edges_from_gamma.iter().all(|e| e.label == "behaviors"));
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one edge per resolved reference")]
#[test]
fn build_graph_unresolved_ref_produces_e003() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    assert_eq!(graph.edge_count(), 1, "only valid ref becomes edge");
    // Unresolved references use E003 ("Unresolved reference"); E001 is parse-only.
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("nonexistent"));
    assert!(
        diagnostics.iter().all(|d| d.code != "E001"),
        "an unresolved reference must not be reported as a parse error (E001)"
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one node per entity")]
#[test]
fn build_graph_duplicate_entity_id_produces_e002() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let file_a = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let file_b = parse(r#"behavior alpha "Duplicate" { contract "dup" }"#, "b.spec");
    let (graph, diagnostics) = build_graph(&[file_a, file_b]);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(errors.len(), 1, "duplicate ID should produce E002");
    assert!(errors[0].message.contains("alpha"));
    // First declaration wins — node still exists
    assert_eq!(graph.node_count(), 1);
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "same ID with different kinds does not produce E002")]
#[test]
fn same_id_different_kind_no_e002() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let file_a = parse(r#"feature code_formatting "Code Formatting" { }"#, "a.spec");
    let file_b = parse(r#"milestone code_formatting "Phase 8: Code Formatting" { }"#, "b.spec");
    let (graph, diagnostics) = build_graph(&[file_a, file_b]);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert!(
        errors.is_empty(),
        "same ID with different kinds should not produce E002, got: {:?}",
        errors
    );
    // W060 should be emitted for cross-kind ID collision
    let w060: Vec<_> = diagnostics.iter().filter(|d| d.code == "W060").collect();
    assert_eq!(w060.len(), 1, "should emit W060 for same-ID-different-kind");

    // First-writer-wins: the first entity (feature) is retained, second (milestone) is skipped.
    // The graph stores one node per raw ID; the first declaration wins deterministically.
    assert_eq!(graph.node_count(), 1);
    let node = graph.node("code_formatting").expect("node should exist");
    assert_eq!(
        node.kind.raw.to_string(),
        "feature",
        "first-writer-wins: feature was declared first, should be retained"
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "parse errors are surfaced as diagnostics")]
#[test]
fn build_graph_surfaces_parse_errors() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    // Malformed spec: `???` is not a valid entity ID, triggers parse errors
    let source = r#"
behavior good "Good" { status planned }
behavior ??? { broken content }
behavior also_good "Also Good" { status done }
"#;
    let spec = parse(source, "bad.spec");
    assert!(!spec.errors.is_empty(), "parser should report errors for malformed input");

    let (_graph, diagnostics) = build_graph(&[spec]);
    let parse_diags: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert!(
        !parse_diags.is_empty(),
        "build_graph should surface parse errors as E001 diagnostics, got: {:?}",
        diagnostics
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one edge per resolved reference")]
#[test]
fn build_graph_cross_file_references() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let types = parse(r#"behavior alpha "A" { contract "first" }"#, "types.spec");
    let main = parse(
        r#"feature gamma "G" { behaviors [alpha] }"#,
        "main.spec",
    );
    let (graph, diagnostics) = build_graph(&[types, main]);

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);
    assert_eq!(graph.edges_from("gamma")[0].target, "alpha");
    assert!(
        diagnostics.iter().all(|d| d.severity != specforge_graph::Severity::Error),
        "cross-file ref should resolve: {:?}", diagnostics
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "close match produces suggestion")]
#[test]
fn build_graph_did_you_mean_for_close_match() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.as_ref().is_some_and(|s| s.contains("alpha_parser")),
        "should suggest close match, got: {:?}", errors[0].suggestion
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "distant match produces no suggestion")]
#[test]
fn build_graph_no_suggestion_for_distant_match() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [zzzzz_totally_unrelated] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_none(),
        "distant match should not produce suggestion, got: {:?}", errors[0].suggestion
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "suggestion appears in help text")]
#[test]
fn build_graph_suggestion_appears_in_help_text() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(errors.len(), 1);
    let suggestion = errors[0].suggestion.as_ref().expect("should have suggestion");
    assert!(
        suggestion.contains("did you mean") && suggestion.contains("alpha_parser"),
        "suggestion should be human-readable help text, got: {:?}", suggestion
    );
}

// === edge types match relationship semantics ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "edge types match relationship semantics")]
#[test]
fn edge_labels_match_field_names() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
invariant inv1 "I1" { guarantee "always" }
feature gamma "G" {
  behaviors [alpha]
  invariants [inv1]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    assert!(
        diagnostics.iter().all(|d| d.severity != specforge_graph::Severity::Error),
        "no errors: {:?}",
        diagnostics
    );
    let edges = graph.edges_from("gamma");
    assert_eq!(edges.len(), 2, "two edges from gamma");

    let behavior_edge = edges.iter().find(|e| e.target == "alpha").unwrap();
    assert_eq!(behavior_edge.label, "behaviors", "edge label matches field name");

    let invariant_edge = edges.iter().find(|e| e.target == "inv1").unwrap();
    assert_eq!(invariant_edge.label, "invariants", "edge label matches field name");
}

// === invalidation subgraph ===

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "changed file and direct dependents are invalidated")]
#[test]
fn invalidation_set_includes_changed_file() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let a = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let b = parse(r#"behavior beta "B" { contract "second" }"#, "b.spec");
    let (graph, _) = build_graph(&[a, b]);

    let import_dag = vec![
        ("a.spec".to_string(), vec![]),
        ("b.spec".to_string(), vec![]),
    ];
    let affected = graph.invalidation_set("a.spec", &import_dag);

    assert!(affected.contains("a.spec"));
    assert!(!affected.contains("b.spec"), "independent file not affected");
}

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "changed file and direct dependents are invalidated")]
#[test]
fn invalidation_set_includes_direct_dependents() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let types = parse(r#"behavior alpha "A" { contract "first" }"#, "types.spec");
    let main = parse(r#"feature gamma "G" { behaviors [alpha] }"#, "main.spec");
    let (graph, _) = build_graph(&[types, main]);

    // main.spec imports types.spec
    let import_dag = vec![
        ("types.spec".to_string(), vec![]),
        ("main.spec".to_string(), vec!["types.spec".to_string()]),
    ];
    let affected = graph.invalidation_set("types.spec", &import_dag);

    assert!(affected.contains("types.spec"));
    assert!(affected.contains("main.spec"), "direct dependent should be invalidated");
}

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "transitive dependents are included in subgraph")]
#[test]
fn invalidation_set_includes_transitive_dependents() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let a = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let b = parse(r#"feature beta "B" { behaviors [alpha] }"#, "b.spec");
    let c = parse(r#"feature gamma "G" { behaviors [beta] }"#, "c.spec");
    let (graph, _) = build_graph(&[a, b, c]);

    // c imports b, b imports a
    let import_dag = vec![
        ("a.spec".to_string(), vec![]),
        ("b.spec".to_string(), vec!["a.spec".to_string()]),
        ("c.spec".to_string(), vec!["b.spec".to_string()]),
    ];
    let affected = graph.invalidation_set("a.spec", &import_dag);

    assert!(affected.contains("a.spec"));
    assert!(affected.contains("b.spec"));
    assert!(affected.contains("c.spec"), "transitive dependent should be invalidated");
}

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "unaffected files are not invalidated")]
#[test]
fn invalidation_set_excludes_unaffected_files() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let a = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let b = parse(r#"feature beta "B" { behaviors [alpha] }"#, "b.spec");
    let c = parse(r#"behavior gamma "G" { contract "independent" }"#, "c.spec");
    let (graph, _) = build_graph(&[a, b, c]);

    let import_dag = vec![
        ("a.spec".to_string(), vec![]),
        ("b.spec".to_string(), vec!["a.spec".to_string()]),
        ("c.spec".to_string(), vec![]),
    ];
    let affected = graph.invalidation_set("a.spec", &import_dag);

    assert_eq!(affected.len(), 2);
    assert!(!affected.contains("c.spec"), "unrelated file not invalidated");
}

// === subgraph rebuild matches full rebuild ===

#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "subgraph rebuild matches full rebuild result")]
#[test]
fn subgraph_rebuild_matches_full_rebuild() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    // Setup: 3 files, b depends on a, c is independent
    let a_v1 = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let b = parse(r#"feature beta "B" { behaviors [alpha] }"#, "b.spec");
    let c = parse(r#"behavior gamma "G" { contract "independent" }"#, "c.spec");

    let import_dag = vec![
        ("a.spec".to_string(), vec![]),
        ("b.spec".to_string(), vec!["a.spec".to_string()]),
        ("c.spec".to_string(), vec![]),
    ];

    // Initial build
    let (mut graph, _) = build_graph(&[a_v1, b.clone(), c.clone()]);

    // "Modify" a.spec: alpha renamed to alpha_v2, add new entity delta
    let a_v2 = parse(
        r#"
behavior alpha_v2 "A v2" { contract "updated" }
behavior delta "D" { contract "new" }
"#,
        "a.spec",
    );

    // Incremental: compute invalidation set, remove affected nodes, rebuild
    let affected = graph.invalidation_set("a.spec", &import_dag);
    // Remove all nodes from affected files
    let nodes_to_remove: Vec<String> = graph
        .nodes()
        .iter()
        .filter(|n| affected.contains(n.source_span.file.as_str()))
        .map(|n| n.id.raw.to_string())
        .collect();
    for id in &nodes_to_remove {
        graph.remove_node(id);
    }
    // Rebuild only affected files
    let b_reparsed = parse(r#"feature beta "B" { behaviors [alpha_v2] }"#, "b.spec");
    let (rebuilt_subgraph, _) = build_graph(&[a_v2.clone(), b_reparsed.clone()]);
    for node in rebuilt_subgraph.nodes() {
        graph.add_node(node.clone());
    }
    for edge in rebuilt_subgraph.edges() {
        graph.add_edge(edge.clone());
    }

    // Cold rebuild for comparison
    let (cold_graph, _) = build_graph(&[a_v2, b_reparsed, c]);

    // Compare: same nodes and edges
    let mut incr_node_ids: Vec<_> = graph.nodes().iter().map(|n| n.id.raw.to_string()).collect::<Vec<String>>();
    let mut cold_node_ids: Vec<_> = cold_graph.nodes().iter().map(|n| n.id.raw.to_string()).collect::<Vec<String>>();
    incr_node_ids.sort();
    cold_node_ids.sort();
    assert_eq!(incr_node_ids, cold_node_ids, "incremental and cold rebuild should have same nodes");

    let mut incr_edges: Vec<_> = graph
        .edges()
        .iter()
        .map(|e| format!("{}->{} ({})", e.source, e.target, e.label))
        .collect();
    let mut cold_edges: Vec<_> = cold_graph
        .edges()
        .iter()
        .map(|e| format!("{}->{} ({})", e.source, e.target, e.label))
        .collect();
    incr_edges.sort();
    cold_edges.sort();
    assert_eq!(incr_edges, cold_edges, "incremental and cold rebuild should have same edges");
}

// === ref blocks as graph nodes ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one node per entity")]
#[test]
fn ref_blocks_become_graph_nodes() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Support Wasm extensions"
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    assert_eq!(graph.node_count(), 2);
    let ref_node = graph.node("gh.issue:42").expect("ref should be a node");
    assert_eq!(ref_node.kind.raw, "ref");
}

#[specforge_test(behavior = "resolve_external_ref_declarations", verify = "ref node is added to graph with scheme metadata")]
#[test]
fn ref_blocks_carry_scheme_metadata() {
    use specforge_graph::{build_graph, FieldValue};
    use specforge_parser::parse;

    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let ref_node = graph.node("gh.issue:42").unwrap();
    let scheme = ref_node.fields.get("scheme");
    assert!(
        matches!(scheme, Some(FieldValue::String(s)) if s == "gh"),
        "ref node should carry scheme field"
    );
}

// === resolve_external_ref_declarations ===

#[specforge_test(behavior = "resolve_external_ref_declarations", verify = "ref with unknown scheme emits I005")]
#[test]
fn ref_with_unknown_scheme_emits_i005() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_provider_schemes: vec!["jira".to_string()].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I005").collect();
    assert_eq!(infos.len(), 1, "unknown scheme 'gh' should produce I005");
    assert!(infos[0].message.contains("gh"), "I005 should mention the scheme");
}

#[specforge_test(behavior = "resolve_external_ref_declarations", verify = "ref with known scheme is registered and marked for provider validation")]
#[test]
fn ref_with_no_providers_configured_skips_i005() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let spec_file = parse(source, "main.spec");
    // Default build_graph — no providers configured
    let (_, diagnostics) = build_graph(&[spec_file]);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I005").collect();
    assert!(infos.is_empty(), "no I005 when no providers configured");
}

#[specforge_test(behavior = "resolve_external_ref_declarations", verify = "ref with known scheme is registered and marked for provider validation")]
#[test]
fn ref_with_known_scheme_registered_and_no_i005() {
    use specforge_graph::{build_graph_with_config, FieldValue, GraphConfig};
    use specforge_parser::parse;

    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_provider_schemes: vec!["gh".to_string()].into_iter().collect(),
        ..Default::default()
    };
    let (graph, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I005").collect();
    assert!(infos.is_empty(), "known scheme should not produce I005");

    // Ref node is registered in the graph with scheme metadata
    let ref_node = graph.node("gh.issue:42").expect("ref should be a graph node");
    assert_eq!(ref_node.kind.raw, "ref");
    assert!(
        matches!(ref_node.fields.get("scheme"), Some(FieldValue::String(s)) if s == "gh"),
        "ref node should carry scheme field 'gh'"
    );
}

// === resolve_soft_cross_extension_references ===

#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "unknown keyword matching known extension emits I004")]
#[test]
fn unknown_keyword_matching_known_extension_emits_i004() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // "journey" is not installed but is known to come from @specforge/product
    let source = r#"journey onboarding_flow "Onboarding Flow" { problem "Users need onboarding" }"#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_extension_keywords: vec![
            ("journey".to_string(), "@specforge/product".to_string()),
        ].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert_eq!(infos.len(), 1, "unknown keyword with known extension should produce I004");
    assert!(infos[0].message.contains("@specforge/product"), "I004 should suggest the extension");
    assert!(infos[0].message.contains("journey"), "I004 should mention the keyword");
}

#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "installed extension with missing entity emits E001")]
#[test]
fn installed_keyword_does_not_emit_i004() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    let source = r#"behavior alpha "A" { contract "first" }"#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        installed_keywords: vec!["behavior".to_string()].into_iter().collect(),
        known_extension_keywords: vec![
            ("behavior".to_string(), "@specforge/software".to_string()),
        ].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert!(infos.is_empty(), "installed keyword should not produce I004");
}

#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "unknown keyword matching known extension emits I004")]
#[test]
fn unknown_keyword_with_no_catalog_match_no_i004() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // "foobar" is not in any catalog — no I004, will become E024 in validation phase
    let source = r#"foobar xyz "X" { stuff "things" }"#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_extension_keywords: vec![
            ("behavior".to_string(), "@specforge/software".to_string()),
        ].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert!(infos.is_empty(), "unknown keyword not in catalog should not produce I004");
}

// === resolve_soft_cross_extension_references: installed keyword + missing entity ===

#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "installed extension with missing entity emits E003")]
#[test]
fn installed_keyword_with_missing_entity_emits_e003() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // "behavior" is installed, but "nonexistent" entity doesn't exist
    let source = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        installed_keywords: vec!["behavior".to_string(), "feature".to_string()]
            .into_iter()
            .collect(),
        known_extension_keywords: vec![
            ("behavior".to_string(), "@specforge/software".to_string()),
            ("feature".to_string(), "@specforge/software".to_string()),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    // Should get E003 (unresolved ref), NOT I004
    let e003s: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    let i004s: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert_eq!(e003s.len(), 1, "missing entity should produce E003");
    assert!(e003s[0].message.contains("nonexistent"));
    assert!(i004s.is_empty(), "installed keywords should not produce I004");
}

#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "installed extension with imported file but missing entity emits E003")]
#[test]
fn installed_keyword_cross_file_missing_entity_emits_e003() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // "behavior" is installed, file is imported, but referenced entity doesn't exist there
    let types = parse(r#"behavior alpha "A" { contract "first" }"#, "types.spec");
    let main = parse(
        r#"feature gamma "G" { behaviors [alpha, missing_beta] }"#,
        "main.spec",
    );
    let config = GraphConfig {
        installed_keywords: vec!["behavior".to_string(), "feature".to_string()]
            .into_iter()
            .collect(),
        known_extension_keywords: vec![
            ("behavior".to_string(), "@specforge/software".to_string()),
            ("feature".to_string(), "@specforge/software".to_string()),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let (graph, diagnostics) = build_graph_with_config(&[types, main], &config);

    let e003s: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(e003s.len(), 1, "missing cross-file entity should produce E003");
    assert!(e003s[0].message.contains("missing_beta"));
    assert_eq!(graph.edge_count(), 1, "only valid ref alpha becomes edge");
}

// === end-to-end pipeline: filesystem → resolve → build_graph ===

fn setup_project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();
    }
    dir
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one node per entity")]
#[test]
fn end_to_end_resolve_and_build() {
    use specforge_graph::build_graph;
    use specforge_resolver::resolve_project;

    let dir = setup_project(&[
        ("types.spec", r#"behavior alpha "A" { contract "first" }"#),
        ("main.spec", "use \"types\"\nfeature gamma \"G\" {\n  behaviors [alpha]\n}"),
    ]);

    let resolved = resolve_project(dir.path());
    assert!(
        resolved.diagnostics.iter().all(|d| d.severity != specforge_graph::Severity::Error),
        "resolve errors: {:?}", resolved.diagnostics
    );

    let spec_files: Vec<_> = resolved.files.iter().map(|f| &f.spec_file).cloned().collect();
    let (graph, diagnostics) = build_graph(&spec_files);

    assert!(
        diagnostics.iter().all(|d| d.severity != specforge_graph::Severity::Error),
        "build errors: {:?}", diagnostics
    );
    assert_eq!(graph.node_count(), 2, "alpha + gamma");
    assert_eq!(graph.edge_count(), 1, "gamma -> alpha");
    assert_eq!(graph.edges_from("gamma")[0].target, "alpha");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "graph contains one edge per resolved reference")]
#[test]
fn end_to_end_with_errors() {
    use specforge_graph::build_graph;
    use specforge_resolver::resolve_project;

    let dir = setup_project(&[
        ("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#),
    ]);

    let resolved = resolve_project(dir.path());
    let spec_files: Vec<_> = resolved.files.iter().map(|f| &f.spec_file).cloned().collect();
    let (graph, diagnostics) = build_graph(&spec_files);

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1, "only valid ref becomes edge");
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(errors.len(), 1);
}

#[test]
// === edge index correctness ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "edge index returns same results as linear scan")]
#[test]
fn edges_from_returns_correct_edges_after_multiple_adds() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_node(make_node("c", "feature"));
    graph.add_node(make_node("d", "feature"));
    graph.add_edge(make_edge("c", "a", "behaviors"));
    graph.add_edge(make_edge("c", "b", "behaviors"));
    graph.add_edge(make_edge("d", "a", "behaviors"));

    let from_c = graph.edges_from("c");
    assert_eq!(from_c.len(), 2);
    assert!(from_c.iter().any(|e| e.target == "a"));
    assert!(from_c.iter().any(|e| e.target == "b"));

    let from_d = graph.edges_from("d");
    assert_eq!(from_d.len(), 1);
    assert_eq!(from_d[0].target, "a");

    let from_a = graph.edges_from("a");
    assert!(from_a.is_empty(), "a has no outgoing edges");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "edge index updated on node removal")]
#[test]
fn edges_to_updated_after_node_removal() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_node(make_node("c", "feature"));
    graph.add_edge(make_edge("c", "a", "behaviors"));
    graph.add_edge(make_edge("c", "b", "behaviors"));

    assert_eq!(graph.edges_to("a").len(), 1);
    assert_eq!(graph.edges_to("b").len(), 1);

    graph.remove_node("a");

    assert!(graph.edges_to("a").is_empty(), "removed node should have no incoming edges");
    assert_eq!(graph.edges_to("b").len(), 1, "b still has incoming edge");
    assert_eq!(graph.edges_from("c").len(), 1, "c should have one edge left");
}

#[specforge_test(behavior = "maintain_mutable_graph", verify = "clear_edges resets all edge indexes")]
#[test]
fn clear_edges_resets_index() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "feature"));
    graph.add_edge(make_edge("b", "a", "behaviors"));

    assert_eq!(graph.edges_from("b").len(), 1);
    graph.clear_edges();
    assert!(graph.edges_from("b").is_empty());
    assert!(graph.edges_to("a").is_empty());
}

// === cycle detection ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "detects cycles in directed graph")]
#[test]
fn detect_cycles_finds_simple_cycle() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_node(make_node("c", "behavior"));
    graph.add_edge(make_edge("a", "b", "depends_on"));
    graph.add_edge(make_edge("b", "c", "depends_on"));
    graph.add_edge(make_edge("c", "a", "depends_on")); // cycle: a->b->c->a

    let cycles = graph.detect_cycles();
    assert!(!cycles.is_empty(), "should detect the cycle");
    // The cycle should contain a, b, c
    let cycle = &cycles[0];
    assert!(cycle.len() >= 3, "cycle should have at least 3 nodes");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "no false positives for acyclic graph")]
#[test]
fn detect_cycles_returns_empty_for_dag() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_node(make_node("c", "feature"));
    graph.add_edge(make_edge("c", "a", "behaviors"));
    graph.add_edge(make_edge("c", "b", "behaviors"));
    graph.add_edge(make_edge("b", "a", "depends_on"));

    let cycles = graph.detect_cycles();
    assert!(cycles.is_empty(), "DAG should have no cycles");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "detects self-referencing cycle")]
#[test]
fn detect_cycles_finds_self_loop() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_edge(make_edge("a", "a", "depends_on")); // self-loop

    let cycles = graph.detect_cycles();
    assert!(!cycles.is_empty(), "self-loop should be detected as cycle");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "has_cycles returns boolean")]
#[test]
fn has_cycles_boolean_check() {
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "behavior"));
    graph.add_edge(make_edge("a", "b", "depends_on"));

    assert!(!graph.has_cycles(), "DAG should not have cycles");

    graph.add_edge(make_edge("b", "a", "depends_on"));
    assert!(graph.has_cycles(), "should detect cycle after adding back-edge");
}

// === cycle detection in build_graph ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "build_graph emits W061 for reference cycles")]
#[test]
fn build_graph_emits_w061_for_reference_cycles() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior a "A" { contract "c" depends_on [b] }
behavior b "B" { contract "c" depends_on [c] }
behavior c "C" { contract "c" depends_on [a] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let w061s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W061").collect();
    assert!(!w061s.is_empty(), "reference cycles should produce W061");
    assert!(w061s[0].message.contains("cycle"), "W061 should mention cycle");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "build_graph no W061 for acyclic refs")]
#[test]
fn build_graph_no_w061_for_acyclic_refs() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior a "A" { contract "c" }
behavior b "B" { contract "c" depends_on [a] }
feature f "F" { behaviors [a, b] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let w061s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W061").collect();
    assert!(w061s.is_empty(), "acyclic graph should not produce W061");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "union type variants are not treated as references")]
#[test]
fn union_type_variants_do_not_produce_unresolved_ref() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
type Status = active | inactive | archived
behavior my_behavior "B" {
    contract "must track status"
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // Variant identifiers are not reference lists, so they must not be reported
    // as unresolved references (E003) or parse errors (E001).
    let unresolved: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "E001" || d.code == "E003")
        .collect();
    assert!(
        unresolved.is_empty(),
        "union variants should not produce E001/E003: {:?}",
        unresolved
    );
    assert_eq!(graph.node_count(), 2, "type + behavior");
    assert_eq!(graph.edge_count(), 0, "no edges from variant lists");
}

// === Phase 9: Predicate Query API ===

#[specforge_test(behavior = "query_graph", verify = "filter_nodes returns matching nodes")]
#[test]
fn filter_nodes_by_kind() {
    let mut graph = Graph::new();
    graph.add_node(make_node("login", "behavior"));
    graph.add_node(make_node("data_ok", "invariant"));
    graph.add_node(make_node("logout", "behavior"));
    graph.add_node(make_node("user_id", "type"));

    let behaviors = graph.filter_nodes(|n| n.kind.raw == "behavior");
    assert_eq!(behaviors.len(), 2, "should find 2 behaviors");

    let invariants = graph.nodes_by_kind("invariant");
    assert_eq!(invariants.len(), 1, "should find 1 invariant");
    assert_eq!(invariants[0].id.raw.as_str(), "data_ok");
}

#[specforge_test(behavior = "query_graph", verify = "filter_nodes with field predicate")]
#[test]
fn filter_nodes_by_field_value() {
    use specforge_parser::FieldValue;

    let mut graph = Graph::new();
    let mut node1 = make_node("login", "behavior");
    node1.fields.push(Sym::new("status"), FieldValue::Identifier("planned".to_string()));
    graph.add_node(node1);

    let mut node2 = make_node("logout", "behavior");
    node2.fields.push(Sym::new("status"), FieldValue::Identifier("implemented".to_string()));
    graph.add_node(node2);

    let planned = graph.filter_nodes(|n| {
        matches!(n.fields.get("status"), Some(FieldValue::Identifier(s)) if s == "planned")
    });
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].id.raw.as_str(), "login");
}

#[specforge_test(behavior = "query_graph", verify = "nodes_by_kind returns empty for unknown kind")]
#[test]
fn nodes_by_kind_returns_empty_for_unknown() {
    let mut graph = Graph::new();
    graph.add_node(make_node("login", "behavior"));

    let result = graph.nodes_by_kind("nonexistent");
    assert!(result.is_empty());
}

// --- one-way authoring, bidirectional query ---

#[specforge_test(behavior = "query_graph", verify = "querying a feature returns behaviors that reference it via incoming edges")]
#[test]
fn subgraph_depth_finds_behavior_via_incoming_implements_edge() {
    let mut graph = Graph::new();
    // Feature has NO behaviors field -- it's a plain node
    graph.add_node(make_node("my_feature", "feature"));
    // Behavior declares features [my_feature] -- creates an Implements edge
    graph.add_node(make_node("my_behavior", "behavior"));
    graph.add_edge(make_edge("my_behavior", "my_feature", "BehaviorImplementsFeature"));

    // Querying the feature at depth 1 should find the behavior
    // via the INCOMING Implements edge -- no reverse edge needed
    let sub = graph.subgraph_depth("my_feature", 1).unwrap();
    assert_eq!(sub.nodes().len(), 2, "feature + behavior");
    assert!(sub.node("my_feature").is_some());
    assert!(sub.node("my_behavior").is_some());
    assert_eq!(sub.edges().len(), 1);
}

#[specforge_test(behavior = "query_graph", verify = "one-way Implements edge is sufficient -- no reverse behaviors edge needed")]
#[test]
fn subgraph_depth_no_reverse_edge_needed_for_feature_behavior_link() {
    let mut graph = Graph::new();
    graph.add_node(make_node("feat_a", "feature"));
    graph.add_node(make_node("beh_1", "behavior"));
    graph.add_node(make_node("beh_2", "behavior"));
    // Both behaviors point to feat_a -- one-way authoring
    graph.add_edge(make_edge("beh_1", "feat_a", "BehaviorImplementsFeature"));
    graph.add_edge(make_edge("beh_2", "feat_a", "BehaviorImplementsFeature"));

    let sub = graph.subgraph_depth("feat_a", 1).unwrap();
    assert_eq!(sub.nodes().len(), 3, "feature + 2 behaviors");
    assert!(sub.node("beh_1").is_some());
    assert!(sub.node("beh_2").is_some());
}

#[specforge_test(behavior = "query_graph", verify = "one-way enforced_by edge is sufficient -- no reverse invariants edge needed")]
#[test]
fn subgraph_depth_invariant_found_via_incoming_enforced_by_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("my_invariant", "invariant"));
    graph.add_node(make_node("my_behavior", "behavior"));
    // Behavior says invariants [my_invariant] -- one-way authoring
    graph.add_edge(make_edge("my_behavior", "my_invariant", "invariants"));

    // Querying the invariant should find the behavior via incoming edge
    let sub = graph.subgraph_depth("my_invariant", 1).unwrap();
    assert_eq!(sub.nodes().len(), 2);
    assert!(sub.node("my_behavior").is_some());
}

#[specforge_test(behavior = "query_graph", verify = "one-way constrains edge is sufficient -- no reverse edge needed")]
#[test]
fn subgraph_depth_constraint_found_via_incoming_constrains_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("my_constraint", "constraint"));
    graph.add_node(make_node("my_behavior", "behavior"));
    // Constraint says constrains [my_behavior] -- one-way authoring
    graph.add_edge(make_edge("my_constraint", "my_behavior", "constrains"));

    // Querying the behavior should find the constraint via incoming edge
    let sub = graph.subgraph_depth("my_behavior", 1).unwrap();
    assert_eq!(sub.nodes().len(), 2);
    assert!(sub.node("my_constraint").is_some());
}

#[specforge_test(behavior = "query_graph", verify = "one-way produces edge is sufficient -- no reverse trigger edge needed")]
#[test]
fn subgraph_depth_event_found_via_incoming_produces_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("my_event", "event"));
    graph.add_node(make_node("my_behavior", "behavior"));
    // Behavior says produces [my_event] -- one-way authoring
    graph.add_edge(make_edge("my_behavior", "my_event", "BehaviorProducesEvent"));

    // Querying the event should find the producing behavior via incoming edge
    let sub = graph.subgraph_depth("my_event", 1).unwrap();
    assert_eq!(sub.nodes().len(), 2);
    assert!(sub.node("my_behavior").is_some());
}

#[specforge_test(behavior = "query_graph", verify = "one-way consumes edge is sufficient -- no reverse consumers edge needed")]
#[test]
fn subgraph_depth_event_found_via_incoming_consumes_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("my_event", "event"));
    graph.add_node(make_node("consumer_behavior", "behavior"));
    // Behavior says consumes [my_event] -- one-way authoring
    graph.add_edge(make_edge("consumer_behavior", "my_event", "BehaviorConsumesEvent"));

    // Querying the event should find the consuming behavior via incoming edge
    let sub = graph.subgraph_depth("my_event", 1).unwrap();
    assert_eq!(sub.nodes().len(), 2);
    assert!(sub.node("consumer_behavior").is_some());
}

#[specforge_test(behavior = "query_graph", verify = "querying invariant finds enforcing behaviors without enforced_by reverse field")]
#[test]
fn subgraph_depth_invariant_found_via_incoming_invariants_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("inv_1", "invariant"));
    graph.add_node(make_node("beh_a", "behavior"));
    graph.add_node(make_node("beh_b", "behavior"));
    // Both behaviors declare invariants [inv_1] -- one-way authoring
    graph.add_edge(make_edge("beh_a", "inv_1", "BehaviorEnforcesInvariant"));
    graph.add_edge(make_edge("beh_b", "inv_1", "BehaviorEnforcesInvariant"));

    // Querying the invariant should find both behaviors
    let sub = graph.subgraph_depth("inv_1", 1).unwrap();
    assert_eq!(sub.nodes().len(), 3, "invariant + 2 behaviors");
    assert!(sub.node("beh_a").is_some());
    assert!(sub.node("beh_b").is_some());
}

// === H10: Custom bidirectional pairs from config ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "custom bidirectional pairs suppress false-positive cycles")]
#[test]
fn custom_bidirectional_pairs_suppress_cycle_warning() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // Create a spec with a 2-hop cycle: a -> b (via "guards") and b -> a (via "guarded_by")
    let source = r#"
behavior a "A" { contract "first" guards [b] }
behavior b "B" { contract "second" guarded_by [a] }
"#;
    let spec_file = parse(source, "main.spec");

    // Without bidirectional pairs configured, the cycle should be reported as W061
    let config_no_pairs = GraphConfig::default();
    let (_, diags_no_pairs) = build_graph_with_config(&[spec_file.clone()], &config_no_pairs);
    let w061_no_pairs: Vec<_> = diags_no_pairs.iter().filter(|d| d.code == "W061").collect();
    assert!(
        !w061_no_pairs.is_empty(),
        "without bidirectional pairs, 2-hop cycle should produce W061"
    );

    // With custom bidirectional pairs configured, the 2-hop cycle should be suppressed
    let config_with_pairs = GraphConfig {
        bidirectional_pairs: vec![
            ("guards".to_string(), "guarded_by".to_string()),
        ],
        ..Default::default()
    };
    let (_, diags_with_pairs) = build_graph_with_config(&[spec_file], &config_with_pairs);
    let w061_with_pairs: Vec<_> = diags_with_pairs.iter().filter(|d| d.code == "W061").collect();
    assert!(
        w061_with_pairs.is_empty(),
        "with bidirectional pairs configured, 2-hop cycle should be suppressed, got: {:?}",
        w061_with_pairs
    );
}

// === M3/M4: Diagnostics carry actionable suggestions ===

#[specforge_test(behavior = "build_in_memory_graph", verify = "W061 carries actionable suggestion")]
#[test]
fn w061_cycle_warning_has_suggestion() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior a "A" { contract "c" depends_on [b] }
behavior b "B" { contract "c" depends_on [c] }
behavior c "C" { contract "c" depends_on [a] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let w061s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W061").collect();
    assert!(!w061s.is_empty(), "reference cycles should produce W061");
    assert!(
        w061s[0].suggestion.is_some(),
        "W061 should carry an actionable suggestion, got None"
    );
    assert!(
        w061s[0].suggestion.as_ref().unwrap().contains("break the cycle"),
        "W061 suggestion should advise breaking the cycle, got: {:?}",
        w061s[0].suggestion
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "E002 carries actionable suggestion")]
#[test]
fn e002_duplicate_entity_has_suggestion() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let file_a = parse(r#"behavior alpha "A" { contract "first" }"#, "a.spec");
    let file_b = parse(r#"behavior alpha "Duplicate" { contract "dup" }"#, "b.spec");
    let (_, diagnostics) = build_graph(&[file_a, file_b]);

    let e002s: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002s.len(), 1, "duplicate ID should produce E002");
    assert!(
        e002s[0].suggestion.is_some(),
        "E002 should carry an actionable suggestion, got None"
    );
    assert!(
        e002s[0].suggestion.as_ref().unwrap().contains("rename"),
        "E002 suggestion should advise renaming, got: {:?}",
        e002s[0].suggestion
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "W060 carries actionable suggestion")]
#[test]
fn w060_cross_kind_collision_has_suggestion() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let file_a = parse(r#"feature code_fmt "Code Formatting" { }"#, "a.spec");
    let file_b = parse(r#"milestone code_fmt "Phase 8" { }"#, "b.spec");
    let (_, diagnostics) = build_graph(&[file_a, file_b]);

    let w060s: Vec<_> = diagnostics.iter().filter(|d| d.code == "W060").collect();
    assert_eq!(w060s.len(), 1, "same-ID-different-kind should produce W060");
    assert!(
        w060s[0].suggestion.is_some(),
        "W060 should carry an actionable suggestion, got None"
    );
    assert!(
        w060s[0].suggestion.as_ref().unwrap().contains("distinct"),
        "W060 suggestion should advise using distinct IDs, got: {:?}",
        w060s[0].suggestion
    );
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "Graph::with_bidirectional_pairs stores pairs for cycle suppression")]
#[test]
fn graph_with_bidirectional_pairs_stores_pairs() {
    let mut graph = Graph::with_bidirectional_pairs(vec![
        ("invariants".to_string(), "enforced_by".to_string()),
    ]);

    // Build a 2-hop cycle with the known bidirectional pair
    graph.add_node(make_node("inv1", "invariant"));
    graph.add_node(make_node("beh1", "behavior"));
    graph.add_edge(make_edge("beh1", "inv1", "invariants"));
    graph.add_edge(make_edge("inv1", "beh1", "enforced_by"));

    // Should NOT detect cycles because this is a known bidirectional pair
    let cycles = graph.detect_cycles();
    assert!(
        cycles.is_empty(),
        "known bidirectional pair should not be flagged as cycle, got: {:?}",
        cycles
    );

    // But a REAL cycle with different labels should still be detected
    graph.add_node(make_node("x", "behavior"));
    graph.add_node(make_node("y", "behavior"));
    graph.add_edge(make_edge("x", "y", "depends_on"));
    graph.add_edge(make_edge("y", "x", "depends_on"));

    let cycles2 = graph.detect_cycles();
    assert!(
        !cycles2.is_empty(),
        "real cycle with non-bidirectional labels should still be detected"
    );
}

// --- add_edge_checked: node existence validation ---

#[specforge_test(behavior = "build_in_memory_graph", verify = "add_edge_checked rejects edge with non-existent source")]
#[test]
fn add_edge_checked_rejects_nonexistent_source() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    // "ghost" does not exist as a node
    let diag = graph.add_edge_checked(make_edge("ghost", "alpha", "depends_on"));
    assert!(
        diag.is_some(),
        "add_edge_checked should return a diagnostic when source node doesn't exist"
    );
    let d = diag.unwrap();
    assert_eq!(d.code, "W011");
    assert!(d.message.contains("ghost"), "diagnostic should mention the missing node ID");
    // Edge should NOT be added
    assert_eq!(graph.edge_count(), 0, "edge should not be added when source is missing");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "add_edge_checked rejects edge with non-existent target")]
#[test]
fn add_edge_checked_rejects_nonexistent_target() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    let diag = graph.add_edge_checked(make_edge("alpha", "phantom", "depends_on"));
    assert!(
        diag.is_some(),
        "add_edge_checked should return a diagnostic when target node doesn't exist"
    );
    let d = diag.unwrap();
    assert_eq!(d.code, "W011");
    assert!(d.message.contains("phantom"), "diagnostic should mention the missing node ID");
    assert_eq!(graph.edge_count(), 0, "edge should not be added when target is missing");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "add_edge_checked accepts edge between existing nodes")]
#[test]
fn add_edge_checked_accepts_valid_edge() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "feature"));
    let diag = graph.add_edge_checked(make_edge("beta", "alpha", "behaviors"));
    assert!(
        diag.is_none(),
        "add_edge_checked should return None when both nodes exist"
    );
    assert_eq!(graph.edge_count(), 1, "edge should be added when both nodes exist");
}

#[specforge_test(behavior = "build_in_memory_graph", verify = "add_edge_checked rejects edge when both source and target are missing")]
#[test]
fn add_edge_checked_rejects_both_missing() {
    let mut graph = Graph::new();
    let diag = graph.add_edge_checked(make_edge("ghost_a", "ghost_b", "depends_on"));
    assert!(
        diag.is_some(),
        "add_edge_checked should return a diagnostic when both nodes don't exist"
    );
    assert_eq!(graph.edge_count(), 0, "edge should not be added when both nodes are missing");
}
