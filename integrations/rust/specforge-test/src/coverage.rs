use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::registry::TestRecordEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExport {
    pub entities: Vec<ExportedEntity>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedEntity {
    pub id: String,
    pub kind: String,
    pub verify: Vec<ExportedVerify>,
    pub testable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedVerify {
    pub kind: String,
    pub description: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageDiff {
    pub entity_id: String,
    pub entity_kind: String,
    pub expected: usize,
    pub covered: usize,
    pub status: CoverageDiffStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageDiffStatus {
    FullyCovered,
    PartiallyCovered,
    Uncovered,
    NoIntent,
}

pub fn compute_coverage_diff(
    graph: &GraphExport,
    entries: &[TestRecordEntry],
) -> Vec<CoverageDiff> {
    // Only include entities that have at least one test in this binary
    let tested_ids: std::collections::HashSet<&str> =
        entries.iter().map(|e| e.entity_id.as_str()).collect();

    graph
        .entities
        .iter()
        .filter(|e| e.testable && tested_ids.contains(e.id.as_str()))
        .map(|entity| {
            let expected = entity.verify.len();
            let covered = entity
                .verify
                .iter()
                .filter(|v| {
                    entries.iter().any(|e| {
                        e.entity_id == entity.id
                            && e.verify.as_deref() == Some(v.description.as_str())
                    })
                })
                .count();

            let status = if expected == 0 {
                CoverageDiffStatus::NoIntent
            } else if covered >= expected {
                CoverageDiffStatus::FullyCovered
            } else if covered > 0 {
                CoverageDiffStatus::PartiallyCovered
            } else {
                CoverageDiffStatus::Uncovered
            };

            CoverageDiff {
                entity_id: entity.id.clone(),
                entity_kind: entity.kind.clone(),
                expected,
                covered,
                status,
            }
        })
        .collect()
}

pub fn format_coverage_summary(
    w: &mut impl std::io::Write,
    diffs: &[CoverageDiff],
    timestamp: &str,
) -> std::io::Result<()> {
    if diffs.is_empty() {
        return Ok(());
    }

    writeln!(w, "\n── specforge coverage (graph: {timestamp}) ──\n")?;

    let id_width = diffs.iter().map(|d| d.entity_id.len()).max().unwrap_or(10).max(6);

    writeln!(w, "  {:<w$}  {:>8}  Status", "Entity", "Coverage", w = id_width)?;
    writeln!(w, "  {:<w$}  {:>8}  ──────", "──────", "────────", w = id_width)?;

    for d in diffs {
        let coverage = format!("{}/{}", d.covered, d.expected);
        let status = match d.status {
            CoverageDiffStatus::FullyCovered => "✓ covered",
            CoverageDiffStatus::PartiallyCovered => "◐ partial",
            CoverageDiffStatus::Uncovered => "✗ uncovered",
            CoverageDiffStatus::NoIntent => "- no verify",
        };
        writeln!(w, "  {:<w$}  {:>8}  {status}", d.entity_id, coverage, w = id_width)?;
    }

    let total_expected: usize = diffs.iter().map(|d| d.expected).sum();
    let total_covered: usize = diffs.iter().map(|d| d.covered).sum();
    writeln!(w, "\n  Total: {total_covered}/{total_expected} verify statements covered")?;

    Ok(())
}

pub fn load_graph(path: &Path) -> Option<GraphExport> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
