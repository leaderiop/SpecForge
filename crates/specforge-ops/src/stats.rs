//! `specforge stats` and `specforge.stats`: the project's statistics, one
//! operation over the project view (ADR 0015). The numbers live here; each
//! surface presents them in its own result type (`ProjectStatistics`,
//! `McpStatsResult`).

use specforge_common::Diagnostic;
use specforge_graph::Graph;
use specforge_project::coverage::{ReportError, Summary};
use std::collections::{BTreeMap, HashSet};

use crate::check::Counts;
use crate::view::ProjectView;

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub total_entities: usize,
    pub total_edges: usize,
    pub orphan_count: usize,
    /// Entities, of any kind, that declare at least one obligation.
    pub verified_count: usize,
    /// The entities that count toward coverage.
    pub testable_count: usize,
    /// Testable entities that declare at least one obligation.
    pub declared_count: usize,
    /// `declared_count` over `testable_count`, in percent (0 when nothing
    /// is testable): declared intent.
    pub declared_pct: f64,
    /// Deprecated alias of [`Self::declared_pct`], kept for readers of the
    /// old name.
    pub coverage_pct: f64,
    /// The share of testable entities proven, in percent: the
    /// `analyze coverage --min` gate's figure. `None` without recorded
    /// test results.
    pub proof_pct: Option<f64>,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub entities_by_kind: BTreeMap<String, usize>,
}

/// The project's statistics. The testable, declared and proven counts are
/// the coverage rule's over the view's recorded report, so stats and
/// `analyze coverage` report the same numbers: testable entities are those
/// that count toward coverage (the entities W004 exempts that declare
/// nothing are left out). The diagnostic counts are of what the view's
/// surface reports for the project ([`ProjectView::reported`]: the CLI what
/// `specforge check` reports, MCP that plus its surface registration
/// notices). A recorded report that cannot be read is the error.
pub fn stats(view: &ProjectView) -> Result<Stats, ReportError> {
    let coverage = view.coverage()?;
    Ok(tally(view.graph(), &coverage.summary, &view.reported()))
}

fn tally(graph: &Graph, coverage: &Summary, diagnostics: &[Diagnostic]) -> Stats {
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

    let declared_pct = if testable_count > 0 {
        (testable_verified as f64 / testable_count as f64) * 100.0
    } else {
        0.0
    };
    let proof_pct = coverage.test_results.as_ref().map(|_| coverage.proof_pct());

    let counts = Counts::of(diagnostics);

    Stats {
        total_entities: graph.node_count(),
        total_edges: graph.edge_count(),
        orphan_count,
        verified_count,
        testable_count,
        declared_count: testable_verified,
        declared_pct,
        coverage_pct: declared_pct,
        proof_pct,
        error_count: counts.errors,
        warning_count: counts.warnings,
        info_count: counts.infos,
        entities_by_kind,
    }
}
