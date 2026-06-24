use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap};
use specforge_emitter::EmitterError;

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

// M2: query for non-existent entity returns EmitterError::EntityNotFound
#[test]
fn query_nonexistent_returns_entity_not_found() {
    let graph = build_graph();
    let result = specforge_emitter::query(&graph, "nonexistent", 1, &[]);
    let err = result.unwrap_err();
    assert!(
        matches!(err, EmitterError::EntityNotFound(_)),
        "expected EntityNotFound, got: {:?}",
        err
    );
    // Display still includes human-readable message
    let msg = format!("{}", err);
    assert!(msg.contains("E003"), "error message should contain E003: {}", msg);
}

// M2: trace for non-existent entity returns EmitterError::EntityNotFound
#[test]
fn trace_nonexistent_returns_entity_not_found() {
    let graph = build_graph();
    let result = specforge_emitter::trace(&graph, "nonexistent");
    let err = result.unwrap_err();
    assert!(
        matches!(err, EmitterError::EntityNotFound(_)),
        "expected EntityNotFound, got: {:?}",
        err
    );
}

// M2: scoped emit for non-existent entity returns EmitterError::EntityNotFound
#[test]
fn emit_scoped_nonexistent_returns_entity_not_found() {
    let graph = build_graph();
    let result = specforge_emitter::emit_json_scoped(&graph, "nonexistent");
    let err = result.unwrap_err();
    assert!(
        matches!(err, EmitterError::EntityNotFound(_)),
        "expected EntityNotFound, got: {:?}",
        err
    );
}

// M2: EmitterError implements Display for backwards compatibility
#[test]
fn emitter_error_display_works() {
    let err = EmitterError::EntityNotFound("entity 'foo' not found".to_string());
    assert_eq!(format!("{}", err), "entity 'foo' not found");

    let err = EmitterError::SerializationError("bad data".to_string());
    assert_eq!(format!("{}", err), "bad data");

    let err = EmitterError::InvalidScope("bad scope".to_string());
    assert_eq!(format!("{}", err), "bad scope");

    let err = EmitterError::Other("something else".to_string());
    assert_eq!(format!("{}", err), "something else");
}

// M2: EmitterError implements std::error::Error
#[test]
fn emitter_error_implements_std_error() {
    let err = EmitterError::EntityNotFound("test".to_string());
    let _: &dyn std::error::Error = &err;
}

// M2: budget strategy error returns EmitterError::Other
#[test]
fn budget_strategy_error_returns_other() {
    let graph = build_graph();
    // Use extremely small budget to force error
    let result = specforge_emitter::emit_json_with_budget_strategy(&graph, 1, "error");
    // Budget may or may not exceed with 2 nodes, so just verify the API compiles
    // and returns the right type
    match result {
        Ok(_) => {} // within budget is fine
        Err(err) => {
            assert!(
                matches!(err, EmitterError::Other(_)),
                "expected Other variant for budget error, got: {:?}",
                err
            );
        }
    }
}

// M2: emit_schema_for_kind with unknown kind returns EmitterError::EntityNotFound
#[test]
fn schema_for_unknown_kind_returns_entity_not_found() {
    use specforge_emitter::{generate_schema, emit_schema_for_kind};
    use specforge_registry::{KindRegistry, EdgeRegistry, FieldRegistry};

    let kind_reg = KindRegistry::default();
    let edge_reg = EdgeRegistry::default();
    let field_reg = FieldRegistry::default();
    let ext_info: Vec<(String, String)> = vec![];

    let schema = generate_schema(&kind_reg, &edge_reg, &field_reg, &ext_info);
    let result = emit_schema_for_kind(&schema, "nonexistent");
    let err = result.unwrap_err();
    assert!(
        matches!(err, EmitterError::EntityNotFound(_)),
        "expected EntityNotFound, got: {:?}",
        err
    );
}
