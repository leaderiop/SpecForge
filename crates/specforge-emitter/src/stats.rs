use specforge_common::{Diagnostic, Severity};
use specforge_graph::Graph;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug)]
pub struct ProjectStats {
    pub total_entities: usize,
    pub total_edges: usize,
    pub orphan_count: usize,
    pub verified_count: usize,
    pub testable_count: usize,
    pub coverage_pct: f64,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub entities_by_kind: BTreeMap<String, usize>,
}

pub fn compute_stats(graph: &Graph) -> ProjectStats {
    compute_stats_with_diagnostics(graph, &[], &[])
}

pub fn compute_stats_with_testable(graph: &Graph, testable_kinds: &[&str]) -> ProjectStats {
    compute_stats_with_diagnostics(graph, testable_kinds, &[])
}

/// Stats knowing only which kinds are testable: every testable kind must
/// declare obligations, and only structure exempts an entity (a union
/// type). With the project's registries, use [`compute_project_stats`].
pub fn compute_stats_with_diagnostics(
    graph: &Graph,
    testable_kinds: &[&str],
    diagnostics: &[Diagnostic],
) -> ProjectStats {
    let coverage =
        crate::coverage::ProjectCoverage::with_testable_kinds(graph, testable_kinds, None);
    compute_project_stats(graph, &coverage.summary, diagnostics)
}

/// The project's statistics. The testable and verified counts are the
/// coverage rule's (`coverage`, from
/// [`crate::coverage::ProjectCoverage`]), so stats and `analyze coverage`
/// report the same numbers: testable entities are those that count toward
/// coverage (entities W004 exempts and that declare nothing are left
/// out), and an entity is verified when it declares at least one
/// obligation.
pub fn compute_project_stats(
    graph: &Graph,
    coverage: &crate::coverage::Summary,
    diagnostics: &[Diagnostic],
) -> ProjectStats {
    let mut entities_by_kind = BTreeMap::new();
    for node in graph.nodes() {
        *entities_by_kind
            .entry(node.kind.raw.to_string())
            .or_insert(0) += 1;
    }
    let verified_count = coverage.discharge_funnel.entities_with_obligations;
    let testable_count = coverage.testable_total;
    let testable_verified = coverage.testable_verified;

    // Orphans: nodes with no incoming and no outgoing edges
    let mut connected: HashSet<&str> = HashSet::new();
    for edge in graph.edges() {
        connected.insert(edge.source.as_str());
        connected.insert(edge.target.as_str());
    }
    let orphan_count = graph
        .nodes()
        .iter()
        .filter(|n| !connected.contains(n.id.raw.as_str()))
        .count();

    let coverage_pct = if testable_count > 0 {
        (testable_verified as f64 / testable_count as f64) * 100.0
    } else {
        0.0
    };

    let mut error_count = 0;
    let mut warning_count = 0;
    let mut info_count = 0;
    for diag in diagnostics {
        match diag.severity {
            Severity::Error => error_count += 1,
            Severity::Warning => warning_count += 1,
            Severity::Info => info_count += 1,
        }
    }

    ProjectStats {
        total_entities: graph.node_count(),
        total_edges: graph.edge_count(),
        orphan_count,
        verified_count,
        testable_count,
        coverage_pct,
        error_count,
        warning_count,
        info_count,
        entities_by_kind,
    }
}
