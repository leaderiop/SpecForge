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

/// A -> B -> C, D (disconnected)
fn build_chain_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "invariant"));
    graph.add_node(node("d", "behavior")); // disconnected
    graph.add_edge(Edge {
        source: "a".to_string(),
        target: "b".to_string(),
        label: "behaviors".to_string(),
    });
    graph.add_edge(Edge {
        source: "b".to_string(),
        target: "c".to_string(),
        label: "invariants".to_string(),
    });
    graph
}

#[test]
fn scoped_json_returns_only_reachable_subgraph() {
    let graph = build_chain_graph();
    let json = specforge_emitter::emit_json_scoped(&graph, "b").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();

    // Scoped at "b": reachable = b (root), a (incoming), c (outgoing)
    assert!(ids.contains(&"b"), "root must be included");
    assert!(ids.contains(&"a"), "upstream neighbor must be included");
    assert!(ids.contains(&"c"), "downstream neighbor must be included");
    assert!(!ids.contains(&"d"), "disconnected node must be excluded");
}

#[test]
fn scoped_context_returns_only_reachable_subgraph() {
    let graph = build_chain_graph();
    let json = specforge_emitter::emit_context_scoped(&graph, "a").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();

    // Scoped at "a": reachable = a, b, c (through chain)
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
    assert!(!ids.contains(&"d"));
}

#[test]
fn scoped_export_on_nonexistent_entity_returns_error() {
    let graph = build_chain_graph();
    let result = specforge_emitter::emit_json_scoped(&graph, "nonexistent");
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.contains("E001"), "error should contain E001: {}", err);
}

#[test]
fn scoped_edges_only_between_reachable_nodes() {
    let graph = build_chain_graph();
    let json = specforge_emitter::emit_json_scoped(&graph, "a").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2); // a->b, b->c
}

#[test]
fn scoped_leaf_node_returns_single_node() {
    let graph = build_chain_graph();
    // "d" is disconnected, so scope returns just d
    let json = specforge_emitter::emit_json_scoped(&graph, "d").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "d");

    let edges = parsed["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 0);
}
