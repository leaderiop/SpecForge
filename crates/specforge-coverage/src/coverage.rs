use crate::merge::MergedReport;
use serde::{Deserialize, Serialize};
use specforge_common::EntityKind;
use specforge_graph::SpecGraph;
use specforge_parser::SpecFile;

/// How far along an entity is in the test coverage chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CoverageLevel {
    /// No test intent declared.
    None,
    /// Has `verify` or `scenario` blocks (intent declared).
    Declared,
    /// Has `tests` field pointing to test files (linked).
    Linked,
    /// Entity ID appears in a `specforge-report.json` (executed).
    Executed,
    /// All tests are passing.
    Passing,
}

impl CoverageLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Declared => "declared",
            Self::Linked => "linked",
            Self::Executed => "executed",
            Self::Passing => "passing",
        }
    }
}

impl std::fmt::Display for CoverageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Coverage information for a single entity.
#[derive(Debug, Clone)]
pub struct EntityCoverage {
    pub entity_id: String,
    pub kind: EntityKind,
    pub level: CoverageLevel,
    /// Number of passing tests (if executed).
    pub pass_count: usize,
    /// Number of failing tests (if executed).
    pub fail_count: usize,
    /// Total tests (if executed).
    pub total_tests: usize,
}

/// Aggregate coverage summary across all testable entities.
#[derive(Debug, Clone)]
pub struct CoverageSummary {
    /// Per-entity coverage details.
    pub entities: Vec<EntityCoverage>,
    /// Total testable entities.
    pub total_testable: usize,
    /// Entities at each coverage level.
    pub count_none: usize,
    pub count_declared: usize,
    pub count_linked: usize,
    pub count_executed: usize,
    pub count_passing: usize,
}

impl CoverageSummary {
    /// Coverage percentage: entities at `Passing` level / total testable * 100.
    pub fn percentage(&self) -> u32 {
        if self.total_testable == 0 {
            return 100; // No testable entities = 100% coverage
        }
        ((self.count_passing as f64 / self.total_testable as f64) * 100.0).round() as u32
    }

    /// Whether coverage meets the given threshold.
    pub fn meets_threshold(&self, threshold: u32) -> bool {
        self.percentage() >= threshold
    }
}

/// Compute three-layer coverage for all testable entities.
///
/// Layers:
/// 1. **Intent**: `verify`/`scenario` in spec files → Declared
/// 2. **Linkage**: `tests` field → Linked
/// 3. **Proof**: entity in merged report → Executed / Passing
pub fn compute_coverage(
    graph: &SpecGraph,
    files: &[SpecFile],
    report: &MergedReport,
) -> CoverageSummary {
    use specforge_common::FieldValue;

    // Build a lookup: entity_id → (has_verify_or_scenario, has_tests_field)
    let mut intent_map: std::collections::HashMap<String, (bool, bool)> =
        std::collections::HashMap::new();

    for file in files {
        for entity in &file.entities {
            if !entity.kind.is_testable() {
                continue;
            }
            let id = entity.id.raw().to_string();
            let has_verify = entity.fields.get("verify").is_some()
                || matches!(entity.fields.get("scenarios"), Some(FieldValue::ScenarioList(s)) if !s.is_empty());
            let has_tests = entity.fields.get("tests").is_some();
            let entry = intent_map.entry(id).or_insert((false, false));
            if has_verify {
                entry.0 = true;
            }
            if has_tests {
                entry.1 = true;
            }
        }
    }

    // Compute per-entity coverage
    let mut entities = Vec::new();
    for node in graph.nodes() {
        if !node.kind.is_testable() {
            continue;
        }
        let id = node.id.raw().to_string();
        let (has_intent, has_tests) = intent_map.get(&id).copied().unwrap_or((false, false));

        let (level, pass_count, fail_count, total_tests) =
            if let Some(entity_result) = report.get(&id) {
                let pass = entity_result.pass_count();
                let fail = entity_result.fail_count();
                let total = entity_result.tests.len();
                if fail == 0 && total > 0 {
                    (CoverageLevel::Passing, pass, fail, total)
                } else {
                    (CoverageLevel::Executed, pass, fail, total)
                }
            } else if has_tests {
                (CoverageLevel::Linked, 0, 0, 0)
            } else if has_intent {
                (CoverageLevel::Declared, 0, 0, 0)
            } else {
                (CoverageLevel::None, 0, 0, 0)
            };

        entities.push(EntityCoverage {
            entity_id: id,
            kind: node.kind.clone(),
            level,
            pass_count,
            fail_count,
            total_tests,
        });
    }

    entities.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));

    let total_testable = entities.len();
    let count_none = entities.iter().filter(|e| e.level == CoverageLevel::None).count();
    let count_declared = entities.iter().filter(|e| e.level == CoverageLevel::Declared).count();
    let count_linked = entities.iter().filter(|e| e.level == CoverageLevel::Linked).count();
    let count_executed = entities.iter().filter(|e| e.level == CoverageLevel::Executed).count();
    let count_passing = entities.iter().filter(|e| e.level == CoverageLevel::Passing).count();

    CoverageSummary {
        entities,
        total_testable,
        count_none,
        count_declared,
        count_linked,
        count_executed,
        count_passing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_level_ordering() {
        assert!(CoverageLevel::None < CoverageLevel::Declared);
        assert!(CoverageLevel::Declared < CoverageLevel::Linked);
        assert!(CoverageLevel::Linked < CoverageLevel::Executed);
        assert!(CoverageLevel::Executed < CoverageLevel::Passing);
    }

    #[test]
    fn summary_percentage() {
        let summary = CoverageSummary {
            entities: Vec::new(),
            total_testable: 10,
            count_none: 2,
            count_declared: 3,
            count_linked: 1,
            count_executed: 1,
            count_passing: 3,
        };
        assert_eq!(summary.percentage(), 30);
        assert!(!summary.meets_threshold(50));
        assert!(summary.meets_threshold(30));
    }

    #[test]
    fn empty_testable_is_100_percent() {
        let summary = CoverageSummary {
            entities: Vec::new(),
            total_testable: 0,
            count_none: 0,
            count_declared: 0,
            count_linked: 0,
            count_executed: 0,
            count_passing: 0,
        };
        assert_eq!(summary.percentage(), 100);
    }
}
