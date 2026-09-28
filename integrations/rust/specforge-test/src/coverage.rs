use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::registry::{TestOutcome, TestRecordEntry};

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
    /// BDD intent (C11-03): `gherkin` field values pointing at feature
    /// files. Present on entities whose spec intent is Cucumber-shaped even
    /// with no `verify` statements.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gherkin: Option<Vec<String>>,
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
    pub passing: usize,
    pub status: CoverageDiffStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageDiffStatus {
    FullyCovered,
    CoveredWithFailures,
    PartiallyCovered,
    Uncovered,
    /// BDD-only intent (C11-03): the behavior specifies scenarios via
    /// gherkin but no Cucumber results have been bound to it yet.
    GherkinSpecified,
    NoIntent,
}

/// Stamp `verify_kind` onto entries from the exported graph (C11-07): the
/// spec declares `verify unit` vs `verify e2e`; the report previously could
/// not distinguish a unit test standing in for an e2e obligation.
pub fn stamp_verify_kinds(
    mut entries: Vec<TestRecordEntry>,
    graph: &GraphExport,
) -> Vec<TestRecordEntry> {
    let mut kind_by_key: HashMap<(String, String), String> = HashMap::new();
    for entity in &graph.entities {
        for v in &entity.verify {
            kind_by_key.insert((entity.id.clone(), v.description.clone()), v.kind.clone());
            kind_by_key.insert((entity.id.clone(), v.slug.clone()), v.kind.clone());
        }
    }
    for entry in &mut entries {
        if let Some(desc) = &entry.verify {
            entry.verify_kind = kind_by_key
                .get(&(entry.entity_id.clone(), desc.clone()))
                .cloned();
        }
    }
    entries
}

pub fn compute_coverage_diff(
    graph: &GraphExport,
    entries: &[TestRecordEntry],
) -> Vec<CoverageDiff> {
    // Join index with slug fallback (C11-02): exact free-text first, then
    // the exported slug, so a reworded spec word no longer silently drops
    // coverage.
    let mut exact: HashMap<(&str, &str), Vec<&TestRecordEntry>> = HashMap::new();
    for entry in entries {
        if let Some(ref desc) = entry.verify {
            exact
                .entry((entry.entity_id.as_str(), desc.as_str()))
                .or_default()
                .push(entry);
        }
    }
    // Slug index keyed by the slugified description.
    let mut slug_of: HashMap<(&str, String), Vec<&TestRecordEntry>> = HashMap::new();
    for entry in entries {
        if let Some(ref desc) = entry.verify {
            let slug = crate::slugify::slugify_verify_description(desc);
            slug_of
                .entry((entry.entity_id.as_str(), slug))
                .or_default()
                .push(entry);
        }
    }

    // C11-02: no `tested_ids` filter — fully-orphaned entities stay visible
    // as Uncovered instead of silently disappearing from the diff.
    graph
        .entities
        .iter()
        .filter(|e| e.testable)
        .map(|entity| {
            let expected = entity.verify.len();
            let mut covered = 0usize;
            let mut passing = 0usize;

            for v in &entity.verify {
                let matched = exact
                    .get(&(entity.id.as_str(), v.description.as_str()))
                    .or_else(|| slug_of.get(&(entity.id.as_str(), v.slug.clone())));
                if let Some(matching) = matched {
                    covered += 1;
                    if matching.iter().all(|e| e.outcome == TestOutcome::Pass) {
                        passing += 1;
                    }
                }
            }

            let gherkin_specified = entity.gherkin.as_ref().is_some_and(|g| !g.is_empty());
            let status = if expected == 0 {
                if gherkin_specified {
                    CoverageDiffStatus::GherkinSpecified
                } else {
                    CoverageDiffStatus::NoIntent
                }
            } else if covered >= expected && passing >= expected {
                CoverageDiffStatus::FullyCovered
            } else if covered >= expected {
                CoverageDiffStatus::CoveredWithFailures
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
                passing,
                status,
            }
        })
        .collect()
}

/// Orphaned test records (C11-02): entries whose (entity_id, verify) match
/// no exported verify statement — exact or slug. `specforge trace`'s
/// unmatched-test promise (decisions.spec) is fulfilled by surfacing these
/// instead of silently ignoring them.
pub fn unmatched_records(graph: &GraphExport, entries: &[TestRecordEntry]) -> Vec<String> {
    let mut exported: HashSet<(String, String)> = HashSet::new();
    for entity in &graph.entities {
        for v in &entity.verify {
            exported.insert((entity.id.clone(), v.description.clone()));
            exported.insert((entity.id.clone(), v.slug.clone()));
        }
    }
    let mut out = Vec::new();
    for entry in entries {
        if let Some(desc) = &entry.verify {
            let key_exact = (entry.entity_id.clone(), desc.clone());
            let slug = crate::slugify::slugify_verify_description(desc);
            let key_slug = (entry.entity_id.clone(), slug);
            if !exported.contains(&key_exact) && !exported.contains(&key_slug) {
                out.push(format!(
                    "{}::{} — no exported verify '{}' on entity '{}'",
                    entry.file, entry.test_name, desc, entry.entity_id
                ));
            }
        }
    }
    out.sort();
    out.dedup();
    out
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

    let id_width = diffs
        .iter()
        .map(|d| d.entity_id.len())
        .max()
        .unwrap_or(10)
        .max(6);

    writeln!(
        w,
        "  {:<w$}  {:>8}  Status",
        "Entity",
        "Coverage",
        w = id_width
    )?;
    writeln!(
        w,
        "  {:<w$}  {:>8}  ──────",
        "──────",
        "────────",
        w = id_width
    )?;

    for d in diffs {
        let coverage = format!("{}/{}", d.covered, d.expected);
        let status = match d.status {
            CoverageDiffStatus::FullyCovered => "✓ covered",
            CoverageDiffStatus::CoveredWithFailures => "! failing",
            CoverageDiffStatus::PartiallyCovered => "◐ partial",
            CoverageDiffStatus::Uncovered => "✗ uncovered",
            CoverageDiffStatus::GherkinSpecified => "◆ gherkin (unbound)",
            CoverageDiffStatus::NoIntent => "- no verify",
        };
        writeln!(
            w,
            "  {:<w$}  {:>8}  {status}",
            d.entity_id,
            coverage,
            w = id_width
        )?;
    }

    // C11-09: pass/fail counts next to the coverage ratio.
    let total_expected: usize = diffs.iter().map(|d| d.expected).sum();
    let total_covered: usize = diffs.iter().map(|d| d.covered).sum();
    let total_passing: usize = diffs.iter().map(|d| d.passing).sum();
    writeln!(
        w,
        "\n  Total: {total_covered}/{total_expected} verify statements covered ({total_passing} passing)"
    )?;

    Ok(())
}
