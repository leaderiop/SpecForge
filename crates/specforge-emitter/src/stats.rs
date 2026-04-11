use specforge_common::{Diagnostic, Severity};
use specforge_graph::{FieldValue, Graph};
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

pub fn compute_stats_with_diagnostics(
    graph: &Graph,
    testable_kinds: &[&str],
    diagnostics: &[Diagnostic],
) -> ProjectStats {
    let mut entities_by_kind = BTreeMap::new();
    let mut verified_count = 0;
    let mut testable_count = 0;
    let mut testable_verified = 0;

    let testable_set: HashSet<&str> = testable_kinds.iter().copied().collect();

    for node in graph.nodes() {
        *entities_by_kind.entry(node.kind.raw.to_string()).or_insert(0) += 1;

        let is_verified = matches!(
            node.fields.get("verify"),
            Some(FieldValue::VerifyList(stmts)) if !stmts.is_empty()
        );

        if is_verified {
            verified_count += 1;
        }

        if testable_set.contains(node.kind.raw.as_str()) {
            testable_count += 1;
            if is_verified {
                testable_verified += 1;
            }
        }
    }

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
