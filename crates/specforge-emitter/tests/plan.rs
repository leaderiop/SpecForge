use specforge_common::SourceSpan;
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};

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

fn testable_node(id: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        "verify".to_string(),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".to_string(),
            description: "it works".to_string(),
        }]),
    );
    Node {
        id: EntityId { raw: id.to_string() },
        kind: EntityKind { raw: "behavior".to_string() },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
    }
}

/// a(feature) -> b(behavior, testable) -> c(behavior, testable)
fn build_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(testable_node("b"));
    graph.add_node(testable_node("c"));
    graph.add_edge(Edge { source: "a".into(), target: "b".into(), label: "behaviors".into() });
    graph.add_edge(Edge { source: "b".into(), target: "c".into(), label: "depends_on".into() });
    graph
}

#[test]
fn plan_with_all_valid_entity_ids_passes() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(result.errors.is_empty(), "no errors expected: {:?}", result.errors);
}

#[test]
fn plan_referencing_nonexistent_entity_produces_error() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "nonexistent", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(
        result.errors.iter().any(|e| e.contains("E001") && e.contains("nonexistent")),
        "should report E001 for nonexistent: {:?}", result.errors
    );
}

#[test]
fn testable_entity_missing_from_plan_produces_warning() {
    let graph = build_graph();
    // Plan only covers "b", missing "c"
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(
        result.warnings.iter().any(|w| w.contains("c")),
        "should warn about missing testable entity 'c': {:?}", result.warnings
    );
}

#[test]
fn plan_dependency_order_contradicting_graph_produces_diagnostic() {
    let graph = build_graph();
    // Graph has edge b -> c (b references c, so c should be implemented before b).
    // Plan lists b before c — wrong order.
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    assert!(
        result.ordering_violations.iter().any(|v| v.contains("b") && v.contains("c")),
        "should flag ordering violation: {:?}", result.ordering_violations
    );
}

#[test]
fn plan_validation_output_is_structured_json() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);
    let json = specforge_emitter::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(parsed["errors"].is_array());
    assert!(parsed["warnings"].is_array());
    assert!(parsed["ordering_violations"].is_array());
    assert!(parsed["validated_entries"].is_array());
    assert!(parsed["schema_version"].is_string());
}

#[test]
fn plan_validation_contract_consistency() {
    // Requires: graph is finalized (we pass a built graph)
    // Ensures: unresolvable IDs diagnosed, missing entries warned, ordering validated, structured report
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "nonexistent", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = specforge_emitter::validate_plan(&graph, &plan, &["behavior"]);

    // Unresolvable IDs diagnosed
    assert!(!result.errors.is_empty());
    // Missing entries warned (c is testable but not in plan)
    assert!(!result.warnings.is_empty());
    // Structured report produced
    let json = specforge_emitter::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
}
