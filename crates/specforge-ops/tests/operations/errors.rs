use specforge_common::{SourceSpan, Sym};
use specforge_emitter::EmitterError;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};

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
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("Title {}", id)),
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
    }
}

fn build_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    graph
}

// M2: trace for non-existent entity returns EmitterError::EntityNotFound
#[test]
fn trace_nonexistent_returns_entity_not_found() {
    let graph = build_graph();
    let result = specforge_ops::trace::trace(&graph, "nonexistent");
    let err = result.unwrap_err();
    assert!(
        matches!(err, EmitterError::EntityNotFound(_)),
        "expected EntityNotFound, got: {:?}",
        err
    );
}
