//! Merging an extension's answer into `specforge-report.json`.

use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_common::{Diagnostic, codes};
use specforge_project::coverage::{ReportedEntity, ReportedTest, TestReport};
use specforge_protocol_types::{CollectOutput, TestStatus};
use std::collections::{BTreeMap, BTreeSet};

// ── merging ─────────────────────────────────────────────────────────────────

/// The entities results may name, each with its obligation texts (the
/// `verify` descriptions a convention-linked test may prove).
#[derive(Debug, Clone, Default)]
pub(super) struct KnownEntities(BTreeMap<String, Vec<String>>);

impl KnownEntities {
    /// Every entity of the view's entity snapshot, with its obligation texts.
    pub(super) fn of(entities: &specforge_registry::EntitySnapshot) -> Self {
        entities
            .records()
            .iter()
            .map(|record| {
                let texts = record.obligations.iter().map(|o| o.text.clone()).collect();
                (record.id.clone(), texts)
            })
            .collect()
    }

    pub(super) fn contains(&self, id: &str) -> bool {
        self.0.contains_key(id)
    }

    /// The entity `id` names, as the test report reads an id (ADR 0284, ADR 0312): an exact key; else `id` rooted
    /// (one leading `/`); else, for an unrooted `id`, the one key that ends with `/{id}` at a segment boundary.
    pub(super) fn resolve<'a>(&'a self, id: &str) -> Resolved<'a> {
        if let Some((key, _)) = self.0.get_key_value(id) {
            return Resolved::Exact(key);
        }
        if let Some((key, _)) = id
            .strip_prefix('/')
            .and_then(|rooted| self.0.get_key_value(rooted))
        {
            return Resolved::Rekeyed(key);
        }
        if id.starts_with('/') {
            return Resolved::Unknown;
        }
        let suffix = format!("/{id}");
        let mut named: Vec<&str> = self
            .0
            .keys()
            .map(String::as_str)
            .filter(|k| k.ends_with(&suffix))
            .collect();
        match named.len() {
            0 => Resolved::Unknown,
            1 => Resolved::Rekeyed(named.remove(0)),
            _ => Resolved::Ambiguous(named),
        }
    }

    /// The entity's obligation texts, empty for an unknown entity.
    pub(super) fn obligations(&self, id: &str) -> &[String] {
        self.0.get(id).map(Vec::as_slice).unwrap_or_default()
    }
}

/// What a reported test id names ([`KnownEntities::resolve`]).
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Resolved<'a> {
    Exact(&'a str),
    Rekeyed(&'a str),
    /// More than one entity; their ids, sorted.
    Ambiguous(Vec<&'a str>),
    Unknown,
}

impl FromIterator<(String, Vec<String>)> for KnownEntities {
    fn from_iter<I: IntoIterator<Item = (String, Vec<String>)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// Counts from one merge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Shape)]
pub struct MergeStats {
    pub entities: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Replace `runner`'s results in `report` with `collected`. Other runners'
/// results are kept, so several runners can report on one project. Tests
/// that name an unknown entity become W115 warnings, and skipped tests are
/// counted but not recorded: a skipped test proves nothing.
pub(super) fn merge(
    report: &mut TestReport,
    runner: &str,
    collected: &CollectOutput,
    known: &KnownEntities,
) -> (MergeStats, Vec<Diagnostic>) {
    for entity in report.results.values_mut() {
        entity
            .tests
            .retain(|t| t.runner.as_deref().is_some_and(|r| r != runner));
    }
    report.results.retain(|_, e| !e.tests.is_empty());

    let mut stats = MergeStats::default();
    let mut diagnostics = Vec::new();
    // The unknown ids, each with the entities it names when it names more than one.
    let mut unknown: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut filed: BTreeMap<&str, Vec<ReportedTest>> = BTreeMap::new();
    for entity in &collected.entity_results {
        let full = match known.resolve(&entity.entity_id) {
            Resolved::Exact(full) | Resolved::Rekeyed(full) => full,
            Resolved::Ambiguous(names) => {
                unknown.insert(entity.entity_id.as_str(), names);
                continue;
            }
            Resolved::Unknown => {
                unknown.insert(entity.entity_id.as_str(), Vec::new());
                continue;
            }
        };
        let recorded = filed.entry(full).or_default();
        for test in &entity.test_results {
            if test.status == TestStatus::Skipped {
                stats.skipped += 1;
                continue;
            }
            // Two spellings of one entity report one test once.
            if recorded
                .iter()
                .any(|r| r.result.name == test.name && r.result.status == test.status)
            {
                continue;
            }
            match test.status {
                TestStatus::Passed => stats.passed += 1,
                _ => stats.failed += 1,
            }
            recorded.push(ReportedTest {
                result: test.clone(),
                runner: Some(runner.to_string()),
            });
        }
    }
    for (full, recorded) in filed {
        if recorded.is_empty() {
            continue;
        }
        stats.entities += 1;
        report
            .results
            .entry(full.to_string())
            .or_insert_with(|| ReportedEntity { tests: Vec::new() })
            .tests
            .extend(recorded);
    }
    for (id, names) in unknown {
        let mut message = format!("{runner} reported tests for unknown entity '{id}'");
        if !names.is_empty() {
            let shown = names.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            let more = if names.len() > 3 { ", \u{2026}" } else { "" };
            message.push_str(&format!(" (it names more than one entity: {shown}{more})"));
        }
        diagnostics.push(Diagnostic::new(codes::W115, message).with_suggestion(
            "check the test's entity annotation for a rename or typo".to_string(),
        ));
    }

    let runners: BTreeSet<&str> = report
        .results
        .values()
        .flat_map(|e| e.tests.iter().filter_map(|t| t.runner.as_deref()))
        .collect();
    report.runner =
        (!runners.is_empty()).then(|| runners.into_iter().collect::<Vec<_>>().join(", "));
    (stats, diagnostics)
}
