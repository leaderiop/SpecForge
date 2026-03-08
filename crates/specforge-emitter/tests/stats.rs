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
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
    }
}

fn node_with_verify(id: &str, kind: &str) -> Node {
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
        kind: EntityKind { raw: kind.to_string() },
        title: None,
        fields,
        source_span: span(),
    }
}

#[test]
fn stats_reports_correct_entity_counts() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior"));
    graph.add_node(node("b", "behavior"));
    graph.add_node(node("c", "feature"));

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.total_entities, 3);
    assert_eq!(stats.entities_by_kind["behavior"], 2);
    assert_eq!(stats.entities_by_kind["feature"], 1);
}

#[test]
fn stats_reports_edge_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: "a".to_string(),
        target: "b".to_string(),
        label: "behaviors".to_string(),
    });

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.total_edges, 1);
}

#[test]
fn stats_reports_orphan_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior")); // orphan — no edges
    graph.add_node(node("b", "feature"));
    graph.add_node(node("c", "behavior"));
    graph.add_edge(Edge {
        source: "b".to_string(),
        target: "c".to_string(),
        label: "behaviors".to_string(),
    });

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.orphan_count, 1); // only "a" is orphan
}

#[test]
fn stats_reports_verified_count() {
    let mut graph = Graph::new();
    graph.add_node(node_with_verify("a", "behavior"));
    graph.add_node(node("b", "behavior")); // no verify

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.verified_count, 1);
}

#[test]
fn stats_on_empty_graph() {
    let graph = Graph::new();
    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.total_entities, 0);
    assert_eq!(stats.total_edges, 0);
    assert_eq!(stats.orphan_count, 0);
    assert_eq!(stats.verified_count, 0);
}

#[test]
fn stats_coverage_with_testable_kinds() {
    let mut graph = Graph::new();
    graph.add_node(node_with_verify("a", "behavior")); // testable + verified
    graph.add_node(node("b", "behavior")); // testable, not verified
    graph.add_node(node("c", "feature")); // not testable

    let testable = &["behavior"];
    let stats = specforge_emitter::compute_stats_with_testable(&graph, testable);
    // 1 verified out of 2 testable = 50%
    assert_eq!(stats.testable_count, 2);
    assert_eq!(stats.verified_count, 1);
    assert!((stats.coverage_pct - 50.0).abs() < 0.01);
}

#[test]
fn stats_coverage_zero_when_no_testable_entities() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature")); // not testable

    let testable: &[&str] = &["behavior"]; // no behaviors in graph
    let stats = specforge_emitter::compute_stats_with_testable(&graph, testable);
    assert_eq!(stats.testable_count, 0);
    assert_eq!(stats.coverage_pct, 0.0);
}

#[test]
fn stats_includes_diagnostic_summary() {
    let graph = Graph::new();
    let diagnostics = vec![
        specforge_common::Diagnostic {
            code: "E001".to_string(),
            severity: specforge_common::Severity::Error,
            message: "bad ref".to_string(),
            span: None,
            suggestion: None,
        },
        specforge_common::Diagnostic {
            code: "W012".to_string(),
            severity: specforge_common::Severity::Warning,
            message: "orphan".to_string(),
            span: None,
            suggestion: None,
        },
    ];
    let stats = specforge_emitter::compute_stats_with_diagnostics(&graph, &[], &diagnostics);
    assert_eq!(stats.error_count, 1);
    assert_eq!(stats.warning_count, 1);
    assert_eq!(stats.info_count, 0);
}
