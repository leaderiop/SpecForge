use crate::types::{CoverageStats, DiagnosticSummary, ProjectStats};
use specforge_common::{Diagnostic, EdgeType, EntityKind, FieldValue, Severity};
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;
use std::collections::BTreeMap;
use std::fmt::Write;

/// Compute aggregate statistics for a specforge project.
pub fn compute_stats(
    graph: &SpecGraph,
    files: &[SpecFile],
    diagnostics: &[Diagnostic],
) -> ProjectStats {
    // Entity counts by kind
    let mut entity_counts = BTreeMap::new();
    for kind in EntityKind::ALL {
        let count = graph.nodes_of_kind(kind.clone()).len();
        if count > 0 {
            entity_counts.insert(kind.keyword().to_string(), count);
        }
    }

    // Orphans
    let orphan_nodes = graph.orphans();
    let orphans: Vec<String> = orphan_nodes.iter().map(|n| n.id.raw().to_string()).collect();

    // Coverage: behaviors with invariants
    let all_behaviors = graph.nodes_of_kind(EntityKind::Behavior);
    let total_behaviors = all_behaviors.len();
    let behaviors_with_invariants = if total_behaviors > 0 {
        let count = all_behaviors
            .iter()
            .filter(|b| {
                graph
                    .outgoing_edges(b.id.raw())
                    .iter()
                    .any(|(tgt, _)| tgt.kind == EntityKind::Invariant)
            })
            .count();
        (count as f64 / total_behaviors as f64) * 100.0
    } else {
        0.0
    };

    // Coverage: behaviors with verify statements
    let behaviors_with_verify = if total_behaviors > 0 {
        let mut count = 0usize;
        for file in files {
            for entity in &file.entities {
                if entity.kind == EntityKind::Behavior && entity.fields.get("verify").is_some_and(|v| matches!(v, FieldValue::VerifyList(list) if !list.is_empty())) {
                    count += 1;
                }
            }
        }
        (count as f64 / total_behaviors as f64) * 100.0
    } else {
        0.0
    };

    // Coverage: features with behaviors
    let all_features = graph.nodes_of_kind(EntityKind::Feature);
    let total_features = all_features.len();
    let features_with_behaviors = if total_features > 0 {
        let count = all_features
            .iter()
            .filter(|f| {
                graph
                    .outgoing_edges(f.id.raw())
                    .iter()
                    .any(|(_, edge)| edge.edge_type == EdgeType::Implements)
            })
            .count();
        (count as f64 / total_features as f64) * 100.0
    } else {
        0.0
    };

    // Diagnostic summary
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Warning)
        .count();
    let infos = diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Info)
        .count();

    ProjectStats {
        entity_counts,
        total_entities: graph.node_count(),
        total_edges: graph.edge_count(),
        orphan_count: orphans.len(),
        orphans,
        diagnostic_summary: DiagnosticSummary {
            errors,
            warnings,
            infos,
        },
        coverage: CoverageStats {
            behaviors_with_invariants_pct: behaviors_with_invariants,
            behaviors_with_verify_pct: behaviors_with_verify,
            features_with_behaviors_pct: features_with_behaviors,
        },
    }
}

/// Format project stats as a human-readable string for terminal output.
pub fn format_stats(stats: &ProjectStats) -> String {
    let mut out = String::new();

    writeln!(out, "Entities: {} total", stats.total_entities).unwrap();
    for (kind, count) in &stats.entity_counts {
        writeln!(out, "  {kind:<16}{count:>4}").unwrap();
    }

    writeln!(out, "Edges: {} total", stats.total_edges).unwrap();

    if stats.orphan_count > 0 {
        let ids = stats.orphans.join(", ");
        writeln!(out, "Orphans: {} ({ids})", stats.orphan_count).unwrap();
    } else {
        writeln!(out, "Orphans: 0").unwrap();
    }

    writeln!(out, "Coverage:").unwrap();
    writeln!(
        out,
        "  behaviors with invariants {:>6.1}%",
        stats.coverage.behaviors_with_invariants_pct
    )
    .unwrap();
    writeln!(
        out,
        "  behaviors with verify     {:>6.1}%",
        stats.coverage.behaviors_with_verify_pct
    )
    .unwrap();
    writeln!(
        out,
        "  features with behaviors   {:>6.1}%",
        stats.coverage.features_with_behaviors_pct
    )
    .unwrap();

    writeln!(
        out,
        "Diagnostics: {} errors, {} warnings, {} info",
        stats.diagnostic_summary.errors,
        stats.diagnostic_summary.warnings,
        stats.diagnostic_summary.infos
    )
    .unwrap();

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{
        Diagnostic, EdgeType, EntityId, EntityKind, FieldMap, FieldValue,
        SourceSpan, ValidationCode, VerifyKind, VerifyStatement,
    };
    use specforge_graph::{GraphEdge, GraphNode, SpecGraph};
    use specforge_parser::{AstEntity, SpecFile};

    fn make_node(id: &str, kind: EntityKind) -> GraphNode {
        GraphNode {
            id: EntityId::parse(id),
            kind,
            title: Some(format!("Test {id}")),
            file: "test.spec".to_string(),
            span: SourceSpan::new("test.spec", 1, 1, 1, 1),
        }
    }

    fn make_graph_with_edges() -> SpecGraph {
        let mut graph = SpecGraph::new();
        graph.add_node(make_node("data_integrity", EntityKind::Invariant));
        graph.add_node(make_node("email_uniqueness", EntityKind::Invariant));
        graph.add_node(make_node("validate_input", EntityKind::Behavior));
        graph.add_node(make_node("create_user", EntityKind::Behavior));
        graph.add_node(make_node("input_validation", EntityKind::Feature));
        graph.add_node(make_node("search_feature", EntityKind::Feature));

        // validate_input references data_integrity
        graph.add_edge(
            "validate_input",
            "data_integrity",
            GraphEdge {
                edge_type: EdgeType::References,
                field_name: "invariants".to_string(),
            },
        );
        // input_validation implements validate_input
        graph.add_edge(
            "input_validation",
            "validate_input",
            GraphEdge {
                edge_type: EdgeType::Implements,
                field_name: "behaviors".to_string(),
            },
        );
        // email_uniqueness, create_user, search_feature are orphans (no edges among themselves)
        graph
    }

    fn make_files_with_verify() -> Vec<SpecFile> {
        vec![SpecFile {
            path: "test.spec".to_string(),
            imports: vec![],
            entities: vec![
                AstEntity {
                    kind: EntityKind::Behavior,
                    id: EntityId::parse("validate_input"),
                    title: Some("Has verify".to_string()),
                    fields: {
                        let mut fm = FieldMap::new();
                        fm.insert(
                            "verify",
                            FieldValue::VerifyList(vec![VerifyStatement {
                                kind: VerifyKind::Unit,
                                description: "test".to_string(),
                            }]),
                        );
                        fm
                    },
                    span: SourceSpan::new("test.spec", 1, 1, 1, 1),
                },
                AstEntity {
                    kind: EntityKind::Behavior,
                    id: EntityId::parse("create_user"),
                    title: Some("No verify".to_string()),
                    fields: FieldMap::new(),
                    span: SourceSpan::new("test.spec", 5, 1, 5, 1),
                },
            ],
            custom_defs: vec![],
            errors: vec![],
        }]
    }

    fn make_diagnostics() -> Vec<Diagnostic> {
        let span = SourceSpan::file_start("test.spec");
        vec![
            Diagnostic::new(ValidationCode::E001, span.clone(), "err1"),
            Diagnostic::new(ValidationCode::W001, span.clone(), "warn1"),
            Diagnostic::new(ValidationCode::W003, span.clone(), "warn2"),
            Diagnostic::new(ValidationCode::I003, span, "info1"),
        ]
    }

    #[test]
    fn stats_counts_by_kind() {
        let graph = make_graph_with_edges();
        let stats = compute_stats(&graph, &[], &[]);
        assert_eq!(stats.entity_counts["invariant"], 2);
        assert_eq!(stats.entity_counts["behavior"], 2);
        assert_eq!(stats.entity_counts["feature"], 2);
        assert_eq!(stats.total_entities, 6);
    }

    #[test]
    fn stats_orphan_count() {
        let graph = make_graph_with_edges();
        let stats = compute_stats(&graph, &[], &[]);
        // email_uniqueness, create_user, search_feature have no edges
        assert_eq!(stats.orphan_count, 3);
        assert!(stats.orphans.contains(&"email_uniqueness".to_string()));
        assert!(stats.orphans.contains(&"create_user".to_string()));
        assert!(stats.orphans.contains(&"search_feature".to_string()));
    }

    #[test]
    fn stats_coverage_pct() {
        let graph = make_graph_with_edges();
        let files = make_files_with_verify();
        let stats = compute_stats(&graph, &files, &[]);
        // 1 of 2 behaviors references an invariant
        assert!((stats.coverage.behaviors_with_invariants_pct - 50.0).abs() < 0.1);
        // 1 of 2 behaviors has verify
        assert!((stats.coverage.behaviors_with_verify_pct - 50.0).abs() < 0.1);
        // 1 of 2 features has behaviors
        assert!((stats.coverage.features_with_behaviors_pct - 50.0).abs() < 0.1);
    }

    #[test]
    fn stats_empty_project() {
        let graph = SpecGraph::new();
        let stats = compute_stats(&graph, &[], &[]);
        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.total_edges, 0);
        assert_eq!(stats.orphan_count, 0);
        assert_eq!(stats.diagnostic_summary.errors, 0);
        assert!((stats.coverage.behaviors_with_invariants_pct - 0.0).abs() < 0.001);
    }

    #[test]
    fn stats_diagnostic_summary() {
        let graph = SpecGraph::new();
        let diags = make_diagnostics();
        let stats = compute_stats(&graph, &[], &diags);
        assert_eq!(stats.diagnostic_summary.errors, 1);
        assert_eq!(stats.diagnostic_summary.warnings, 2);
        assert_eq!(stats.diagnostic_summary.infos, 1);
    }

    #[test]
    fn format_stats_snapshot() {
        let graph = make_graph_with_edges();
        let files = make_files_with_verify();
        let diags = make_diagnostics();
        let stats = compute_stats(&graph, &files, &diags);
        let output = format_stats(&stats);
        insta::assert_snapshot!(output);
    }
}
