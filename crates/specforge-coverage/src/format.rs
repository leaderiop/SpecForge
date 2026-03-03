use crate::coverage::{CoverageLevel, CoverageSummary};
use serde::Serialize;

/// Format coverage summary as human-readable text.
pub fn format_text(summary: &CoverageSummary, verbose: bool) -> String {
    let mut out = String::new();

    if verbose && !summary.entities.is_empty() {
        // Column widths
        let max_id = summary
            .entities
            .iter()
            .map(|e| e.entity_id.len())
            .max()
            .unwrap_or(9)
            .max(9);
        let max_kind = summary
            .entities
            .iter()
            .map(|e| e.kind.keyword().len())
            .max()
            .unwrap_or(4)
            .max(4);

        out.push_str(&format!(
            "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>6}\n",
            "Entity", "Kind", "Level", "Tests",
        ));
        out.push_str(&format!(
            "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>6}\n",
            "-".repeat(max_id),
            "-".repeat(max_kind),
            "--------",
            "------",
        ));

        for entity in &summary.entities {
            let tests_col = if entity.total_tests > 0 {
                format!("{}/{}", entity.pass_count, entity.total_tests)
            } else {
                "-".to_string()
            };

            let level_marker = match entity.level {
                CoverageLevel::Passing => "passing",
                CoverageLevel::Executed => "executed",
                CoverageLevel::Linked => "linked",
                CoverageLevel::Declared => "declared",
                CoverageLevel::None => "none",
            };

            out.push_str(&format!(
                "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>6}\n",
                entity.entity_id,
                entity.kind.keyword(),
                level_marker,
                tests_col,
            ));
        }

        out.push('\n');
    }

    // Summary line
    out.push_str(&format!(
        "Coverage: {}% ({} passing / {} testable)\n",
        summary.percentage(),
        summary.count_passing,
        summary.total_testable,
    ));

    // Breakdown
    out.push_str(&format!(
        "  passing: {}  executed: {}  linked: {}  declared: {}  none: {}\n",
        summary.count_passing,
        summary.count_executed,
        summary.count_linked,
        summary.count_declared,
        summary.count_none,
    ));

    out
}

/// JSON representation of coverage for machine consumption.
#[derive(Serialize)]
struct JsonCoverageOutput {
    percentage: u32,
    total_testable: usize,
    passing: usize,
    executed: usize,
    linked: usize,
    declared: usize,
    none: usize,
    entities: Vec<JsonEntityCoverage>,
}

#[derive(Serialize)]
struct JsonEntityCoverage {
    entity_id: String,
    kind: String,
    level: String,
    pass_count: usize,
    fail_count: usize,
    total_tests: usize,
}

/// Format coverage summary as JSON.
pub fn format_json(summary: &CoverageSummary) -> String {
    let output = JsonCoverageOutput {
        percentage: summary.percentage(),
        total_testable: summary.total_testable,
        passing: summary.count_passing,
        executed: summary.count_executed,
        linked: summary.count_linked,
        declared: summary.count_declared,
        none: summary.count_none,
        entities: summary
            .entities
            .iter()
            .map(|e| JsonEntityCoverage {
                entity_id: e.entity_id.clone(),
                kind: e.kind.keyword().to_string(),
                level: e.level.label().to_string(),
                pass_count: e.pass_count,
                fail_count: e.fail_count,
                total_tests: e.total_tests,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&output).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::{EntityCoverage, CoverageLevel};
    use specforge_common::EntityKind;

    fn sample_summary() -> CoverageSummary {
        CoverageSummary {
            entities: vec![
                EntityCoverage {
                    entity_id: "auth_login".to_string(),
                    kind: EntityKind::Behavior,
                    level: CoverageLevel::Passing,
                    pass_count: 3,
                    fail_count: 0,
                    total_tests: 3,
                },
                EntityCoverage {
                    entity_id: "data_persist".to_string(),
                    kind: EntityKind::Behavior,
                    level: CoverageLevel::None,
                    pass_count: 0,
                    fail_count: 0,
                    total_tests: 0,
                },
            ],
            total_testable: 2,
            count_none: 1,
            count_declared: 0,
            count_linked: 0,
            count_executed: 0,
            count_passing: 1,
        }
    }

    #[test]
    fn text_format_non_verbose() {
        let text = format_text(&sample_summary(), false);
        assert!(text.contains("Coverage: 50%"));
        assert!(text.contains("1 passing / 2 testable"));
        // Should not contain entity table
        assert!(!text.contains("auth_login"));
    }

    #[test]
    fn text_format_verbose() {
        let text = format_text(&sample_summary(), true);
        assert!(text.contains("auth_login"));
        assert!(text.contains("data_persist"));
        assert!(text.contains("passing"));
        assert!(text.contains("none"));
    }

    #[test]
    fn json_format() {
        let json = format_json(&sample_summary());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["percentage"], 50);
        assert_eq!(parsed["total_testable"], 2);
        assert_eq!(parsed["entities"].as_array().unwrap().len(), 2);
    }
}
