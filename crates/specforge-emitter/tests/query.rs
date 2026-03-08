use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};

fn span() -> SourceSpan {
    SourceSpan {
        file: "test.spec".to_string(),
        start_line: 1,
        start_col: 0,
        end_line: 1,
        end_col: 0,
    }
}

fn node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: Some(format!("Title {}", id)),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

/// a(feature) -> b(behavior) -> c(invariant) -> d(event)
fn build_linear_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "invariant"));
    graph.add_node(node("d", "event"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    graph.add_edge(Edge { source: "b".into(), target: "c".into(), label: "invariants".into() });
    graph.add_edge(Edge { source: "c".into(), target: "d".into(), label: "produces".into() });
    graph
}

#[test]
fn depth_0_returns_only_target_entity() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "b", 0, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "b");
}

#[test]
fn depth_1_returns_direct_neighbors() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), 3); // a, b, c
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
    assert!(!ids.contains(&"d"));
}

#[test]
fn depth_n_returns_all_within_n_hops() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "a", 3, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 4); // all nodes reachable within 3 hops
}

#[test]
fn kind_filter_restricts_results() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "b", 2, &["invariant"]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    // b is the root (always included), and invariant "c" is within 1 hop
    // kind filter keeps only behavior and invariant nodes
    for node in nodes {
        let kind = node["kind"].as_str().unwrap();
        assert!(
            kind == "behavior" || kind == "invariant",
            "unexpected kind: {} (node {})", kind, node["id"]
        );
    }
}

#[test]
fn multiple_kind_filters_combine_as_union() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "b", 2, &["feature", "event"]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    // Root "b" (behavior) is always included even when kind-filtered
    // Plus "a" (feature at depth 1) and "d" (event at depth 2)
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"b"), "root always included");
    assert!(ids.contains(&"a"), "feature within range");
}

#[test]
fn query_nonexistent_entity_returns_error() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "nonexistent", 1, &[]);
    assert!(result.is_err());
}

#[test]
fn query_includes_schema_version() {
    let graph = build_linear_graph();
    let result = specforge_emitter::query(&graph, "b", 1, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(parsed["schema_version"].is_string());
}

#[test]
fn query_same_entity_same_depth_is_deterministic() {
    let graph = build_linear_graph();
    let r1 = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    let r2 = specforge_emitter::query(&graph, "b", 1, &[]).unwrap();
    assert_eq!(r1, r2);
}
