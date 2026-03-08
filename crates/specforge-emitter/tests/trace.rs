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

/// a -> b -> c (linear chain)
fn build_chain() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "invariant"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    graph.add_edge(Edge { source: "b".into(), target: "c".into(), label: "invariants".into() });
    graph
}

#[test]
fn trace_shows_upstream_and_downstream() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();

    assert!(trace.upstream.iter().any(|l| l.entity_id == "a"), "upstream should include a");
    assert!(trace.downstream.iter().any(|l| l.entity_id == "c"), "downstream should include c");
}

#[test]
fn trace_shows_full_chain_depth() {
    let mut graph = build_chain();
    graph.add_node(node("d", "event"));
    graph.add_edge(Edge { source: "c".into(), target: "d".into(), label: "produces".into() });

    let trace = specforge_emitter::trace(&graph, "a").unwrap();
    // a is root, so no upstream, downstream = b, c, d
    assert!(trace.upstream.is_empty());
    let ids: Vec<&str> = trace.downstream.iter().map(|l| l.entity_id.as_str()).collect();
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
    assert!(ids.contains(&"d"));
}

#[test]
fn trace_nonexistent_entity_returns_error() {
    let graph = build_chain();
    let result = specforge_emitter::trace(&graph, "nonexistent");
    assert!(result.is_err());
}

#[test]
fn trace_serializes_to_json() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();
    let json = specforge_emitter::serialize_trace(&trace);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["entity_id"], "b");
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
    assert!(parsed["schema_version"].is_string());
}

#[test]
fn trace_on_leaf_has_upstream_only() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "c").unwrap();

    assert!(!trace.upstream.is_empty(), "leaf should have upstream");
    assert!(trace.downstream.is_empty(), "leaf should have no downstream");
}

#[test]
fn trace_on_root_has_downstream_only() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "a").unwrap();

    assert!(trace.upstream.is_empty(), "root should have no upstream");
    assert!(!trace.downstream.is_empty(), "root should have downstream");
}

#[test]
fn trace_all_covers_all_root_entities() {
    let graph = build_chain();
    let traces = specforge_emitter::trace_all(&graph);
    // All 3 entities should have a trace
    assert_eq!(traces.len(), 3);
    let ids: Vec<&str> = traces.iter().map(|t| t.entity_id.as_str()).collect();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"c"));
}

#[test]
fn trace_all_serializes_as_json_array() {
    let graph = build_chain();
    let traces = specforge_emitter::trace_all(&graph);
    let json = specforge_emitter::serialize_trace_all(&traces);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
    assert_eq!(parsed["traces"].as_array().unwrap().len(), 3);
}

#[test]
fn trace_detects_dangling_edge_as_gap() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    // Dangling edge: b -> nonexistent (target not in graph)
    graph.add_edge(Edge { source: "b".into(), target: "missing_entity".into(), label: "depends_on".into() });

    let gaps = specforge_emitter::detect_trace_gaps(&graph);
    assert!(!gaps.is_empty(), "should detect dangling edge");
    assert!(gaps.iter().any(|g| g.contains("missing_entity")),
        "gap should mention missing entity: {:?}", gaps);
}
