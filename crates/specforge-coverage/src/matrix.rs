use crate::coverage::{CoverageLevel, CoverageSummary};

/// Render a traceability matrix combining trace report data with coverage.
///
/// Output format:
/// ```text
///   Entity             Kind        Intent  Tests File   Status    Pass/Fail
///   ------             ----        ------  ----------   ------    ---------
///   auth_login         behavior    yes     tests/a.rs   passing   3/0
///   data_persist       behavior    no      -            none      -
/// ```
pub fn render_traceability_matrix(summary: &CoverageSummary) -> String {
    let mut out = String::new();

    if summary.entities.is_empty() {
        out.push_str("No testable entities found.\n");
        return out;
    }

    let max_id = summary
        .entities
        .iter()
        .map(|e| e.entity_id.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let max_kind = summary
        .entities
        .iter()
        .map(|e| e.kind.keyword().len())
        .max()
        .unwrap_or(4)
        .max(4);

    out.push_str(&format!(
        "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>9}\n",
        "Entity", "Kind", "Level", "Pass/Fail",
    ));
    out.push_str(&format!(
        "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>9}\n",
        "-".repeat(max_id),
        "-".repeat(max_kind),
        "--------",
        "---------",
    ));

    for entity in &summary.entities {
        let pass_fail = if entity.total_tests > 0 {
            format!("{}/{}", entity.pass_count, entity.fail_count)
        } else {
            "-".to_string()
        };

        let level_icon = match entity.level {
            CoverageLevel::Passing => "passing",
            CoverageLevel::Executed => "executed",
            CoverageLevel::Linked => "linked",
            CoverageLevel::Declared => "declared",
            CoverageLevel::None => "none",
        };

        out.push_str(&format!(
            "  {:<max_id$}  {:<max_kind$}  {:>8}  {:>9}\n",
            entity.entity_id,
            entity.kind.keyword(),
            level_icon,
            pass_fail,
        ));
    }

    // Summary
    out.push('\n');
    out.push_str(&format!(
        "Coverage: {}% ({} passing / {} testable)\n",
        summary.percentage(),
        summary.count_passing,
        summary.total_testable,
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coverage::{EntityCoverage, CoverageSummary, CoverageLevel};
    use specforge_common::EntityKind;

    #[test]
    fn renders_matrix() {
        let summary = CoverageSummary {
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
        };

        let output = render_traceability_matrix(&summary);
        assert!(output.contains("auth_login"));
        assert!(output.contains("data_persist"));
        assert!(output.contains("Coverage: 50%"));
    }

    #[test]
    fn renders_empty_matrix() {
        let summary = CoverageSummary {
            entities: Vec::new(),
            total_testable: 0,
            count_none: 0,
            count_declared: 0,
            count_linked: 0,
            count_executed: 0,
            count_passing: 0,
        };

        let output = render_traceability_matrix(&summary);
        assert!(output.contains("No testable entities found"));
    }
}
