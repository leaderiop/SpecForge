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
        title: None,
        fields: FieldMap::new(),
        source_span: span(),
    }
}

fn node_with_verify(id: &str, kind: &str) -> Node {
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
        kind: EntityKind { raw: Sym::new(kind) },
        title: None,
        fields,
        source_span: span(),
    }
}

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports correct entity counts")]
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

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
// (covers edge count reporting)
#[test]
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_reports_edge_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature"));
    graph.add_node(node("b", "behavior"));
    graph.add_edge(Edge {
        source: Sym::new("a"),
        target: Sym::new("b"),
        label: Sym::new("behaviors"),
    });

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.total_edges, 1);
}

// B:compute_project_statistics — verify unit "stats reports orphan count"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports orphan count")]
fn stats_reports_orphan_count() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "behavior")); // orphan — no edges
    graph.add_node(node("b", "feature"));
    graph.add_node(node("c", "behavior"));
    graph.add_edge(Edge {
        source: Sym::new("b"),
        target: Sym::new("c"),
        label: Sym::new("behaviors"),
    });

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.orphan_count, 1); // only "a" is orphan
}

// B:compute_project_statistics — verify unit "stats reports coverage percentage"
// (covers verified entity counting for coverage computation)
#[test]
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_reports_verified_count() {
    let mut graph = Graph::new();
    graph.add_node(node_with_verify("a", "behavior"));
    graph.add_node(node("b", "behavior")); // no verify

    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.verified_count, 1);
}

// B:compute_project_statistics — verify unit "stats reports correct entity counts"
// (edge case: empty graph)
#[test]
#[specforge_test(behavior = "compute_project_statistics")]
fn stats_on_empty_graph() {
    let graph = Graph::new();
    let stats = specforge_emitter::compute_stats(&graph);
    assert_eq!(stats.total_entities, 0);
    assert_eq!(stats.total_edges, 0);
    assert_eq!(stats.orphan_count, 0);
    assert_eq!(stats.verified_count, 0);
}

// B:compute_project_statistics — verify unit "stats reports coverage percentage"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports coverage percentage")]
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

// B:compute_project_statistics — verify unit "coverage is 0% when testable_entity_count is zero"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "coverage is 0% when testable_entity_count is zero")]
fn stats_coverage_zero_when_no_testable_entities() {
    let mut graph = Graph::new();
    graph.add_node(node("a", "feature")); // not testable

    let testable: &[&str] = &["behavior"]; // no behaviors in graph
    let stats = specforge_emitter::compute_stats_with_testable(&graph, testable);
    assert_eq!(stats.testable_count, 0);
    assert_eq!(stats.coverage_pct, 0.0);
}

// B:compute_project_statistics — verify unit "stats reports diagnostic summary"
#[test]
#[specforge_test(behavior = "compute_project_statistics", verify = "stats reports diagnostic summary")]
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
