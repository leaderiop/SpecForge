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

fn node(id: &str, kind: &str, title: Option<&str>) -> Node {
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new(kind) },
        title: title.map(|s| s.to_string()),
        fields: FieldMap::new(),
        source_span: span(),
    }
}

// B:serialize_dot_visualization — verify unit "DOT output is valid Graphviz syntax"
#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "DOT output is valid Graphviz syntax")]
fn empty_graph_produces_valid_dot() {
    let graph = Graph::new();
    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.starts_with("digraph"));
    assert!(dot.contains('{'));
    assert!(dot.trim_end().ends_with('}'));
}

// B:serialize_dot_visualization — verify unit "nodes are labeled with IDs"
#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "nodes are labeled with IDs")]
fn dot_nodes_labeled_with_id_and_title() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha Behavior")));

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("alpha"), "node ID in DOT output");
    assert!(dot.contains("Alpha Behavior"), "node title in DOT output");
}

// B:serialize_dot_visualization — verify unit "edges are labeled with types"
#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "edges are labeled with types")]
fn dot_edges_labeled_with_type() {
    let mut graph = Graph::new();
    graph.add_node(node("feat_a", "feature", Some("Feature A")));
    graph.add_node(node("beh_b", "behavior", None));
    graph.add_edge(Edge {
        source: Sym::new("feat_a"),
        target: Sym::new("beh_b"),
        label: Sym::new("behaviors"),
    });

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("feat_a") && dot.contains("beh_b"), "edge endpoints in DOT");
    assert!(dot.contains("behaviors"), "edge label in DOT");
}

// B:serialize_dot_visualization — verify unit "node shapes use extension-defined dot_shape"
#[test]
#[specforge_test(behavior = "serialize_dot_visualization", verify = "node shapes use extension-defined dot_shape")]
fn dot_node_default_shape_is_box() {
    let mut graph = Graph::new();
    graph.add_node(node("alpha", "behavior", Some("Alpha")));

    let dot = specforge_emitter::emit_dot(&graph);
    assert!(dot.contains("box"), "default shape is box");
}
