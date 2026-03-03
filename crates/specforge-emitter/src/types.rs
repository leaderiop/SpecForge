use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A generated output file with a relative path and content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Options controlling which entities to render.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderOptions {
    pub only: Option<specforge_common::EntityKind>,
}

/// Aggregate statistics for a specforge project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub entity_counts: BTreeMap<String, usize>,
    pub total_entities: usize,
    pub total_edges: usize,
    pub orphan_count: usize,
    pub orphans: Vec<String>,
    pub diagnostic_summary: DiagnosticSummary,
    pub coverage: CoverageStats,
}

/// Counts of diagnostics by severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

/// Coverage percentages for key spec relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageStats {
    pub behaviors_with_invariants_pct: f64,
    pub behaviors_with_verify_pct: f64,
    pub features_with_behaviors_pct: f64,
}

/// A single link in a traceability chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLink {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub depth: usize,
}

/// A gap in traceability — an entity missing an expected connection direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceGap {
    pub entity_id: String,
    pub entity_kind: String,
    pub missing_direction: String,
}

/// A single trace chain rooted at an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceChain {
    pub root: String,
    pub root_kind: String,
    pub upstream: Vec<TraceLink>,
    pub downstream: Vec<TraceLink>,
    pub gaps: Vec<TraceGap>,
}

/// Full traceability report across all deliverables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceReport {
    pub chains: Vec<TraceChain>,
    pub total_gaps: usize,
}
