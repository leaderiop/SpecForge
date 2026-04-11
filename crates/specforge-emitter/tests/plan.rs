use specforge_common::{SourceSpan, Sym};
use specforge_graph::{Edge, Graph, Node};
use specforge_parser::{EntityId, EntityKind, FieldMap, FieldValue, VerifyStatement};
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

fn testable_node(id: &str) -> Node {
    let mut fields = FieldMap::new();
    fields.push(
        Sym::new("verify"),
        FieldValue::VerifyList(vec![VerifyStatement {
            kind: "unit".to_string(),
            description: "it works".to_string(),
        }]),
    );
    Node {
        id: EntityId { raw: Sym::new(id) },
        kind: EntityKind { raw: Sym::new("behavior") },
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

// B:validate_agent_plan — verify unit "plan with all valid entity IDs passes validation"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan with all valid entity IDs passes validation")]
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

// B:validate_agent_plan — verify unit "plan referencing nonexistent entity ID produces E001"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan referencing nonexistent entity ID produces E001")]
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

// B:validate_agent_plan — verify unit "testable entity missing from plan produces warning"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "testable entity missing from plan produces warning")]
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

// B:validate_agent_plan — verify unit "plan dependency order contradicting graph produces diagnostic"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "plan dependency order contradicting graph produces diagnostic")]
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

// B:validate_agent_plan — verify unit "output is structured JSON"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "output is structured JSON")]
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

// B:validate_agent_plan — verify contract "requires/ensures consistency for agent plan validation"
#[test]
#[specforge_test(behavior = "validate_agent_plan", verify = "requires/ensures consistency for agent plan validation")]
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
