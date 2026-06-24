use specforge_graph::{build_graph_with_config, Edge, Graph, GraphConfig, Node};
use specforge_common::{SourceSpan, Sym};
use specforge_parser::{parse, EntityId, EntityKind, FieldMap};
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

fn make_node_in_file(id: &str, kind: &str, file: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: Some(id.to_string()),
        fields: FieldMap::new(),
        source_span: SourceSpan {
            file: Sym::new(file),
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

// B:build_in_memory_graph — verify contract "requires/ensures consistency for in-memory graph construction"
#[test]
#[specforge_test(behavior = "build_in_memory_graph", verify = "requires/ensures consistency for in-memory graph construction")]
fn build_in_memory_graph_contract() {
    // Requires: set of nodes and edges
    // Ensures: graph contains all nodes and edges, queryable by ID
    let mut graph = Graph::new();
    graph.add_node(make_node("alpha", "behavior"));
    graph.add_node(make_node("beta", "feature"));
    graph.add_node(make_node("gamma", "type"));
    graph.add_edge(make_edge("beta", "alpha", "behaviors"));
    graph.add_edge(make_edge("beta", "gamma", "types"));

    assert_eq!(graph.node_count(), 3, "all nodes must be present");
    assert_eq!(graph.edge_count(), 2, "all edges must be present");
    assert!(graph.node("alpha").is_some(), "node queryable by ID");
    assert!(graph.node("beta").is_some(), "node queryable by ID");
    assert!(graph.node("gamma").is_some(), "node queryable by ID");
    assert_eq!(graph.edges_from("beta").len(), 2, "edges queryable from source");
    assert_eq!(graph.edges_to("alpha").len(), 1, "edges queryable to target");
}

// B:maintain_mutable_graph — verify contract "requires/ensures consistency for mutable graph maintenance"
#[test]
#[specforge_test(behavior = "maintain_mutable_graph", verify = "requires/ensures consistency for mutable graph maintenance")]
fn maintain_mutable_graph_contract() {
    // Requires: existing graph with nodes and edges
    // Ensures: add/remove mutations reflect correctly, edges cleaned on node removal
    let mut graph = Graph::new();
    graph.add_node(make_node("a", "behavior"));
    graph.add_node(make_node("b", "feature"));
    graph.add_edge(make_edge("b", "a", "behaviors"));

    assert_eq!(graph.node_count(), 2);
    assert_eq!(graph.edge_count(), 1);

    // Add a new node
    graph.add_node(make_node("c", "type"));
    assert_eq!(graph.node_count(), 3, "add_node must increase count");

    // Remove node with edge — edge must be cleaned up
    graph.remove_node("a");
    assert_eq!(graph.node_count(), 2, "remove_node must decrease count");
    assert!(graph.node("a").is_none(), "removed node must not be queryable");
    assert!(graph.edges().is_empty(), "edges to removed node must be cleaned");

    // Remaining nodes intact
    assert!(graph.node("b").is_some());
    assert!(graph.node("c").is_some());
}

// B:compute_subgraph_for_invalidation — verify contract "requires/ensures consistency for subgraph invalidation"
#[test]
#[specforge_test(behavior = "compute_subgraph_for_invalidation", verify = "requires/ensures consistency for subgraph invalidation")]
fn compute_subgraph_for_invalidation_contract() {
    // Requires: graph with nodes across files + import DAG
    // Ensures: invalidation_set returns changed file + transitive dependents, excludes unrelated
    let mut graph = Graph::new();
    graph.add_node(make_node_in_file("a", "behavior", "types.spec"));
    graph.add_node(make_node_in_file("b", "feature", "main.spec"));
    graph.add_node(make_node_in_file("c", "behavior", "unrelated.spec"));
    graph.add_edge(make_edge("b", "a", "behaviors"));

    let import_dag = vec![
        ("types.spec".to_string(), vec![]),
        ("main.spec".to_string(), vec!["types.spec".to_string()]),
        ("unrelated.spec".to_string(), vec![]),
    ];

    let affected = graph.invalidation_set("types.spec", &import_dag);

    assert!(affected.contains("types.spec"), "changed file must be included");
    assert!(affected.contains("main.spec"), "direct dependent must be included");
    assert!(!affected.contains("unrelated.spec"), "unrelated file must be excluded");
}

// B:resolve_external_ref_declarations — verify contract "requires/ensures consistency for external ref resolution"
#[test]
#[specforge_test(behavior = "resolve_external_ref_declarations", verify = "requires/ensures consistency for external ref resolution")]
fn resolve_external_ref_declarations_contract() {
    // Requires: graph with ref nodes having scheme metadata
    // Ensures: ref nodes are in graph with scheme field; known scheme = no I005, unknown = I005
    let source = r#"
ref gh.issue:42 "Wasm support"
ref jira.story:ABC-1 "Backend work"
"#;
    let spec_file = parse(source, "main.spec");

    // Known scheme = "gh", unknown = "jira"
    let config = GraphConfig {
        known_provider_schemes: vec!["gh".to_string()].into_iter().collect(),
        ..Default::default()
    };
    let (graph, diagnostics) = build_graph_with_config(&[spec_file], &config);

    // Both refs are nodes in the graph
    assert!(graph.node("gh.issue:42").is_some(), "ref with known scheme must be in graph");
    assert!(graph.node("jira.story:ABC-1").is_some(), "ref with unknown scheme must be in graph");

    // Known scheme: no I005
    let gh_infos: Vec<_> = diagnostics.iter()
        .filter(|d| d.code == "I005" && d.message.contains("gh"))
        .collect();
    assert!(gh_infos.is_empty(), "known scheme should not produce I005");

    // Unknown scheme: I005
    let jira_infos: Vec<_> = diagnostics.iter()
        .filter(|d| d.code == "I005" && d.message.contains("jira"))
        .collect();
    assert_eq!(jira_infos.len(), 1, "unknown scheme should produce I005");
}

// B:resolve_soft_cross_extension_references — verify contract "requires/ensures consistency for soft cross-extension resolution"
#[test]
#[specforge_test(behavior = "resolve_soft_cross_extension_references", verify = "requires/ensures consistency for soft cross-extension resolution")]
fn resolve_soft_cross_extension_references_contract() {
    // Requires: entity with keyword from uninstalled but known extension
    // Ensures: I004 info diagnostic with extension name suggestion
    let source = r#"journey onboarding "Onboarding" { problem "Users need onboarding" }"#;
    let spec_file = parse(source, "main.spec");

    let config = GraphConfig {
        known_extension_keywords: vec![
            ("journey".to_string(), "@specforge/product".to_string()),
        ].into_iter().collect(),
        ..Default::default()
    };
    let (graph, diagnostics) = build_graph_with_config(&[spec_file], &config);

    // Entity is still added to graph (soft resolution)
    assert!(graph.node("onboarding").is_some(), "entity must be in graph despite uninstalled extension");

    // I004 info diagnostic emitted
    let i004s: Vec<_> = diagnostics.iter().filter(|d| d.code == "I004").collect();
    assert_eq!(i004s.len(), 1, "uninstalled extension keyword must produce I004");
    assert!(i004s[0].message.contains("@specforge/product"), "I004 must mention the extension");
    assert!(i004s[0].message.contains("journey"), "I004 must mention the keyword");
    assert_eq!(i004s[0].severity, specforge_graph::Severity::Info, "I004 must be info severity");
}

// B:provide_did_you_mean_suggestions — verify contract "requires/ensures consistency for did-you-mean suggestions"
#[test]
#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "requires/ensures consistency for did-you-mean suggestions")]
fn provide_did_you_mean_suggestions_contract() {
    // Requires: unresolved reference with entity IDs available
    // Ensures: close match → suggestion, distant match → no suggestion
    let source_close = r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#;
    let spec_file = parse(source_close, "test.spec");
    let config = GraphConfig::default();
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);
    let e003: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(e003.len(), 1);
    assert!(e003[0].suggestion.is_some(), "close match must produce suggestion");

    let source_far = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [zzzzz_completely_different] }
"#;
    let spec_file = parse(source_far, "test.spec");
    let (_, diagnostics) = build_graph_with_config(&[spec_file], &config);
    let e003: Vec<_> = diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert_eq!(e003.len(), 1);
    assert!(e003[0].suggestion.is_none(), "distant match must not produce suggestion");
}
