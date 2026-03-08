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

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: kind.to_string() },
        title: title.map(|s| s.to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

#[test]
fn empty_graph_produces_valid_dot() {
    let graph = Graph::new();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.starts_with("digraph"));
    assert!(dot.contains('{'));
    assert!(dot.trim_end().ends_with('}'));
}

#[test]
fn dot_nodes_labeled_with_id_and_title() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha Behavior")));

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("alpha"), "node ID in DOT output");
    assert!(dot.contains("Alpha Behavior"), "node title in DOT output");
}

#[test]
fn dot_edges_labeled_with_type() {
    let mut graph = Graph::new();
    graph.add_node(node("feat_a", "feature", Some("Feature A")));
    graph.add_node(node("beh_b", "behavior", None));
    graph.add_edge(Edge {
        source: "feat_a".to_string(),
        target: "beh_b".to_string(),
        label: "behaviors".to_string(),
    });

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("feat_a") && dot.contains("beh_b"), "edge endpoints in DOT");
    assert!(dot.contains("behaviors"), "edge label in DOT");
}

#[test]
fn dot_node_default_shape_is_box() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("box"), "default shape is box");
}
