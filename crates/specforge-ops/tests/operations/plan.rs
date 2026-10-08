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
        kind: EntityKind {
            raw: Sym::new(kind),
        },
        title: Some(format!("Title {}", id)),
        fields: FieldMap::new(),
        source_span: span(),
        methods: Vec::new(),
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
        kind: EntityKind {
            raw: Sym::new("behavior"),
        },
        title: Some(format!("Title {}", id)),
        fields,
        source_span: span(),
        methods: Vec::new(),
    }
}

/// a(feature) -> b(behavior, testable) -> c(behavior, testable)
fn build_graph() -> Graph {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(testable_node("b"));
    graph.add_node(testable_node("c"));
    graph.add_edge(Edge {
        source: "a".into(),
        target: "b".into(),
        label: "behaviors".into(),
    });
    graph.add_edge(Edge {
        source: "b".into(),
        target: "c".into(),
        label: "depends_on".into(),
    });
    graph
}

// B:validate_agent_plan — verify unit "plan with all valid entity IDs passes validation"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "plan with all valid entity IDs passes validation"
)]
fn plan_with_all_valid_entity_ids_passes() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
    assert!(
        result.errors.is_empty(),
        "no errors expected: {:?}",
        result.errors
    );
}

// B:validate_agent_plan — verify unit "plan referencing nonexistent entity ID produces E003"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "plan referencing nonexistent entity ID produces E003"
)]
fn plan_referencing_nonexistent_entity_produces_error() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "nonexistent", "action": "implement" },
        ]
    });

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("E003") && e.contains("nonexistent")),
        "should report E003 for nonexistent: {:?}",
        result.errors
    );
}

// B:validate_agent_plan — verify unit "testable entity missing from plan produces warning"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "testable entity missing from plan produces warning"
)]
fn testable_entity_missing_from_plan_produces_warning() {
    let graph = build_graph();
    // Plan only covers "b", missing "c"
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
    // One warning, for c; none for the covered b or the untestable a.
    assert_eq!(
        result.warnings,
        vec!["testable entity 'c' (behavior) is not covered by the plan".to_string()]
    );
    assert_eq!(result.gaps.len(), 1);
    assert_eq!(
        result.gaps[0].kind,
        specforge_ops::plan::PlanGapKind::MissingPlanEntry
    );
    assert_eq!(result.gaps[0].target, "c");
    // A plan covering both behaviors warns about nothing.
    let full = serde_json::json!({
        "entries": [
            { "entity_id": "c", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
        ]
    });
    let covered = crate::view_support::plan_check(&graph, &["behavior"], &full);
    assert!(covered.warnings.is_empty(), "{:?}", covered.warnings);
}

// B:validate_agent_plan — verify unit "plan dependency order contradicting graph produces diagnostic"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "plan dependency order contradicting graph produces diagnostic"
)]
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

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
    assert!(
        result
            .ordering_violations
            .iter()
            .any(|v| v.contains("b") && v.contains("c")),
        "should flag ordering violation: {:?}",
        result.ordering_violations
    );
}

// B:validate_agent_plan — verify unit "output is structured JSON"
#[specforge_test(behavior = "validate_agent_plan", verify = "output is structured JSON")]
fn plan_validation_output_is_structured_json() {
    let graph = build_graph();
    let plan = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
        ]
    });

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
    let json = specforge_ops::plan::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

    assert!(parsed["errors"].is_array());
    assert!(parsed["warnings"].is_array());
    assert!(parsed["ordering_violations"].is_array());
    assert!(parsed["validated_entries"].is_array());
    assert!(parsed["schema_version"].is_string());
}

// B:validate_agent_plan — verify contract "requires/ensures consistency for agent plan validation"
#[specforge_test(
    behavior = "validate_agent_plan",
    verify = "Validate Agent Implementation Plan: agent plan validation holds — validation_complete_fired, unresolvable_ids_diagnosed, missing_entries_warned, ordering_validated, structured_report_produced, plan_validated_emitted"
)]
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

    let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);

    // unresolvable_ids_diagnosed: E003 for the unknown ID.
    assert_eq!(
        result.errors,
        vec!["E003: unresolved entity 'nonexistent' in plan — not found in graph".to_string()]
    );
    // missing_entries_warned: c is testable but not in the plan.
    assert_eq!(
        result.warnings,
        vec!["testable entity 'c' (behavior) is not covered by the plan".to_string()]
    );
    assert!(result.ordering_violations.is_empty());

    // ordering_validated: b references c, so c must come first.
    let misordered = serde_json::json!({
        "entries": [
            { "entity_id": "b", "action": "implement" },
            { "entity_id": "c", "action": "implement" },
        ]
    });
    let ordered = crate::view_support::plan_check(&graph, &["behavior"], &misordered);
    assert_eq!(
        ordered.ordering_violations,
        vec!["'b' depends on 'c' (via depends_on), but 'c' appears later in the plan".to_string()]
    );
    let fixed = serde_json::json!({
        "entries": [
            { "entity_id": "c", "action": "implement" },
            { "entity_id": "b", "action": "implement" },
        ]
    });
    let fine = crate::view_support::plan_check(&graph, &["behavior"], &fixed);
    assert!(
        fine.ordering_violations.is_empty(),
        "{:?}",
        fine.ordering_violations
    );

    // structured_report_produced: validated entries, gaps and ordering
    // violations as JSON.
    let json = specforge_ops::plan::serialize_plan_result(&result);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!({
            "schema_version": "0.1.0",
            "errors": ["E003: unresolved entity 'nonexistent' in plan — not found in graph"],
            "warnings": ["testable entity 'c' (behavior) is not covered by the plan"],
            "ordering_violations": [],
            "validated_entries": ["b"],
        })
    );
    let report: serde_json::Value =
        serde_json::from_str(&specforge_ops::plan::serialize_plan_result(&ordered)).unwrap();
    assert_eq!(
        report["ordering_violations"],
        serde_json::json!([
            "'b' depends on 'c' (via depends_on), but 'c' appears later in the plan"
        ])
    );
}

/// Pin (plan 14 P5): each gap's `context` is the text of its entry in the
/// outcome's string lists. T8 deletes the lists and this test with them.
#[test]
fn every_gap_carries_the_text_of_its_list_entry() {
    use specforge_ops::plan::PlanGapKind;

    let graph = build_graph();
    for plan in [
        serde_json::json!({
            "entries": [
                { "entity_id": "nonexistent", "action": "implement" },
                { "entity_id": "b", "action": "implement" },
            ]
        }),
        serde_json::json!({
            "entries": [
                { "entity_id": "b", "action": "implement" },
                { "entity_id": "c", "action": "implement" },
            ]
        }),
    ] {
        let result = crate::view_support::plan_check(&graph, &["behavior"], &plan);
        let texts = |kind: PlanGapKind| -> Vec<String> {
            result
                .gaps
                .iter()
                .filter(|gap| gap.kind == kind)
                .map(|gap| gap.context.clone())
                .collect()
        };
        assert_eq!(texts(PlanGapKind::UnresolvedEntity), result.errors);
        assert_eq!(texts(PlanGapKind::MissingPlanEntry), result.warnings);
        assert_eq!(texts(PlanGapKind::Ordering), result.ordering_violations);
    }
}
