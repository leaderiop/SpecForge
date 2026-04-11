use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
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

fn node(id: &str, kind: &str) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
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

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "trace from entity shows upstream and downstream connections")]
fn trace_shows_upstream_and_downstream() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "b").unwrap();

    assert!(trace.upstream.iter().any(|l| l.entity_id == "a"), "upstream should include a");
    assert!(trace.downstream.iter().any(|l| l.entity_id == "c"), "downstream should include c");
}

// B:compute_traceability_chain — verify unit "trace shows full chain depth"
#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "trace shows full chain depth")]
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

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (error case: nonexistent entity returns error)
#[test]
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_nonexistent_entity_returns_error() {
    let graph = build_chain();
    let result = specforge_emitter::trace(&graph, "nonexistent");
    assert!(result.is_err());
}

// B:serialize_traceability_data — verify unit "output conforms to Graph Protocol schema"
#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "output conforms to Graph Protocol schema")]
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

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (edge case: leaf has upstream only)
#[test]
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_on_leaf_has_upstream_only() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "c").unwrap();

    assert!(!trace.upstream.is_empty(), "leaf should have upstream");
    assert!(trace.downstream.is_empty(), "leaf should have no downstream");
}

// B:compute_traceability_chain — verify unit "trace from entity shows upstream and downstream connections"
// (edge case: root has downstream only)
#[test]
#[specforge_test(behavior = "compute_traceability_chain")]
fn trace_on_root_has_downstream_only() {
    let graph = build_chain();
    let trace = specforge_emitter::trace(&graph, "a").unwrap();

    assert!(trace.upstream.is_empty(), "root should have no upstream");
    assert!(!trace.downstream.is_empty(), "root should have downstream");
}

// B:serialize_traceability_data — verify unit "full trace covers all root entities across registered edge types"
#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "full trace covers all root entities across registered edge types")]
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

// B:serialize_traceability_data — verify unit "output conforms to Graph Protocol schema"
#[test]
#[specforge_test(behavior = "serialize_traceability_data")]
fn trace_all_serializes_as_json_array() {
    let graph = build_chain();
    let traces = specforge_emitter::trace_all(&graph);
    let json = specforge_emitter::serialize_trace_all(&traces);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["schema_version"].is_string());
    assert!(parsed["traces"].is_array());
    assert_eq!(parsed["traces"].as_array().unwrap().len(), 3);
}

// B:compute_traceability_chain — verify unit "missing link in chain is flagged"
#[test]
#[specforge_test(behavior = "compute_traceability_chain", verify = "missing link in chain is flagged")]
fn trace_missing_link_flagged() {
    // A graph with an edge pointing to a non-existent node represents a missing link
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    graph.add_edge(Edge { source: "b".into(), target: "phantom".into(), label: "invariants".into() });

    let gaps = specforge_emitter::detect_trace_gaps(&graph);
    assert!(gaps.iter().any(|g| g.contains("phantom")),
        "missing link to phantom must be flagged: {:?}", gaps);

    // Trace from "b" should work (b exists) but not include phantom in downstream
    let trace = specforge_emitter::trace(&graph, "b").unwrap();
    assert!(!trace.downstream.iter().any(|l| l.entity_id == "phantom"),
        "phantom node should not appear in trace (it doesn't exist in graph)");
}

// B:serialize_traceability_data — verify unit "gaps in chain are highlighted"
#[test]
#[specforge_test(behavior = "serialize_traceability_data", verify = "gaps in chain are highlighted")]
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
