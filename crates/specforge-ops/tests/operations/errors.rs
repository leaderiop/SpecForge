use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_ops::trace::TraceError;
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

// M2: trace for non-existent entity returns TraceError::EntityNotFound
#[test]
fn trace_nonexistent_returns_entity_not_found() {
    let project = crate::view_support::Project::of_graph(build_graph(), Default::default());
    let err = specforge_ops::trace::trace(
        &project.view(),
        specforge_ops::trace::Target::Entity("nonexistent"),
    )
    .unwrap_err();
    assert!(
        matches!(err, TraceError::EntityNotFound { .. }),
        "expected EntityNotFound, got: {err:?}"
    );
}

#[specforge_test_macros::test(
    behavior = "compute_traceability_chain",
    verify = "tracing an entity the graph lacks is E003 naming the closest entity"
)]
fn tracing_an_unknown_entity_is_e003_with_a_suggestion() {
    let project = crate::view_support::Project::of_graph(build_graph(), Default::default());
    let trace = |id| {
        specforge_ops::trace::trace(&project.view(), specforge_ops::trace::Target::Entity(id))
            .unwrap_err()
    };
    let near = trace("bb");
    assert_eq!(
        near,
        TraceError::EntityNotFound {
            entity_id: "bb".into(),
            near: Some("b".into()),
        }
    );
    let error = specforge_ops::OpError::from(near);
    assert_eq!(error.code, "E003");
    assert_eq!(error.message, "unresolved entity 'bb' — not found in graph");
    assert_eq!(error.suggestion.as_deref(), Some("did you mean 'b'?"));
    // Nothing close: no suggestion.
    let far = specforge_ops::OpError::from(trace("zzzzzzzz"));
    assert_eq!(far.suggestion, None);
}
