use specforge_graph::{Edge, Graph, Node};
use specforge_common::SourceSpan;
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_test_macros::test as specforge_test;

fn make_node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: "test.spec".to_string(),
            start_line: 1,
            start_col: 1,
            end_line: 1,
            end_col: 1,
        },
    }
}

fn make_edge(source: &str, target: &str, label: &str) -> Edge {
    Edge {
        source: source.to_string(),
        target: target.to_string(),
        label: label.to_string(),
    }
}

// --- build_in_memory_graph ---

#[specforge_test(behavior = "build_in_memory_graph")]
#[test]
fn graph_one_node_per_entity() {
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "behavior"));

    assert_eq!(graph.nodes().len(), 2);
    assert!(graph.node("alpha").is_some());
    assert!(graph.node("beta").is_some());
}

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "maintain_mutable_graph")]
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

#[specforge_test(behavior = "maintain_mutable_graph")]
#[test]
fn removing_node_removes_its_edges() {
    let mut graph = Graph::new();
    graph.add_node(make_node("feat", "feature"));
    graph.add_node(make_node("beh", "behavior"));
    graph.add_edge(make_edge("feat", "beh", "behaviors"));

    graph.remove_node("beh");

    assert!(graph.edges().is_empty(), "edges to removed node should be gone");
}

#[specforge_test(behavior = "maintain_mutable_graph")]
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

#[specforge_test(behavior = "compute_subgraph_for_invalidation")]
#[test]
fn subgraph_for_file() {
    let mut graph = Graph::new();

    let mut node_a = make_node("alpha", "behavior");
    node_a.source_span.file = "a.spec".to_string();
    let mut node_b = make_node("beta", "behavior");
    node_b.source_span.file = "b.spec".to_string();
    let mut node_c = make_node("gamma", "feature");
    node_c.source_span.file = "a.spec".to_string();

    graph.add_node(node_a);
    graph.add_node(node_b);
    graph.add_node(node_c);
    graph.add_edge(make_edge("gamma", "alpha", "behaviors"));

    let sub = graph.nodes_in_file("a.spec");
    assert_eq!(sub.len(), 2);
}

// === build_graph integration ===

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
#[test]
fn build_graph_unresolved_ref_produces_e001() {
    use specforge_graph::build_graph;
    use specforge_parser::parse;

    let source = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    assert_eq!(graph.edge_count(), 1, "only valid ref becomes edge");
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("nonexistent"));
}

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
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

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.as_ref().is_some_and(|s| s.contains("alpha_parser")),
        "should suggest close match, got: {:?}", errors[0].suggestion
    );
}

// === invalidation subgraph ===

#[specforge_test(behavior = "compute_subgraph_for_invalidation")]
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

#[specforge_test(behavior = "compute_subgraph_for_invalidation")]
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

#[specforge_test(behavior = "compute_subgraph_for_invalidation")]
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

#[specforge_test(behavior = "compute_subgraph_for_invalidation")]
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

// === ref blocks as graph nodes ===

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
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

#[specforge_test(behavior = "resolve_external_ref_declarations")]
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

#[specforge_test(behavior = "resolve_external_ref_declarations")]
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

#[specforge_test(behavior = "resolve_external_ref_declarations")]
#[test]
fn ref_with_known_scheme_no_i005() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    let source = r#"ref gh.issue:42 "Support Wasm extensions""#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_provider_schemes: vec!["gh".to_string()].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I005").collect();
    assert!(infos.is_empty(), "known scheme should not produce I005");
}

// === resolve_soft_cross_extension_references ===

#[specforge_test(behavior = "resolve_soft_cross_extension_references")]
#[test]
fn unknown_keyword_matching_known_extension_emits_i004() {
    use specforge_graph::{build_graph_with_config, GraphConfig};
    use specforge_parser::parse;

    // "capability" is not installed but is known to come from @specforge/product
    let source = r#"capability onboarding_flow "Onboarding Flow" { problem "Users need onboarding" }"#;
    let spec_file = parse(source, "main.spec");
    let config = GraphConfig {
        known_extension_keywords: vec![
            ("capability".to_string(), "@specforge/product".to_string()),
        ].into_iter().collect(),
        ..Default::default()
    };
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);

    let infos: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert_eq!(infos.len(), 1, "unknown keyword with known extension should produce I004");
    assert!(infos[0].message.contains("@specforge/product"), "I004 should suggest the extension");
    assert!(infos[0].message.contains("capability"), "I004 should mention the keyword");
}

#[specforge_test(behavior = "resolve_soft_cross_extension_references")]
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

#[specforge_test(behavior = "resolve_soft_cross_extension_references")]
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

#[specforge_test(behavior = "build_in_memory_graph")]
#[test]
fn end_to_end_resolve_and_build() {
    use specforge_graph::build_graph;
    use specforge_resolver::resolve_project;

    let dir = setup_project(&[
        ("types.spec", r#"behavior alpha "A" { contract "first" }"#),
        ("main.spec", "use types\nfeature gamma \"G\" {\n  behaviors [alpha]\n}"),
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

#[specforge_test(behavior = "build_in_memory_graph")]
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
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
}
