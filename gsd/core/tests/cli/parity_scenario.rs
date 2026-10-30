//! The scenario check (D-r44-17): every golden recorded over the taskboard is read by one named comparison. A golden
//! nobody reads is a recording that proves nothing; a comparison nobody names can be deleted without a failure. The
//! table maps each of the 19 goldens of `gsd/parity/goldens/taskboard/` to the test that compares the twin with it and
//! to the text that test reads it by (its name, or the prefix of the names it builds).

use std::collections::BTreeSet;
use std::path::Path;

/// `(golden, test fn, the text the fn reads the golden by)`.
const SCENARIO: [(&str, &str, &str); 19] = [
    (
        "audit-uat",
        "the_uat_twin_reports_what_audit_uat_lists",
        "audit-uat",
    ),
    (
        "decision-coverage-plan-2",
        "taskboard_plan_verdicts_match",
        "decision-coverage-plan-2",
    ),
    (
        "gap-analysis-plan-post-1",
        "taskboard_plan_verdicts_match",
        "gap-analysis-plan-post-",
    ),
    (
        "gap-analysis-plan-post-2",
        "taskboard_plan_verdicts_match",
        "gap-analysis-plan-post-",
    ),
    (
        "junit",
        "taskboard_coverage_matches_recorded_tests",
        "junit.xml",
    ),
    (
        "phase-plan-index-1",
        "taskboard_waves_match",
        "phase-plan-index-",
    ),
    (
        "phase-plan-index-2",
        "taskboard_waves_match",
        "phase-plan-index-",
    ),
    (
        "progress-json",
        "taskboard_progress_matches_gsd_tools",
        "progress-json",
    ),
    (
        "roadmap-analyze",
        "taskboard_roadmap_matches_gsd_tools",
        "roadmap-analyze",
    ),
    (
        "state-json",
        "completed_phases_and_percent_match_upstream",
        "state-json",
    ),
    (
        "validate-health",
        "health_equals_the_golden_on_the_twin",
        "validate-health",
    ),
    (
        "verify-command-paths-1",
        "taskboard_plan_verdicts_match",
        "verify-command-paths",
    ),
    (
        "verify-command-paths-2",
        "taskboard_plan_verdicts_match",
        "verify-command-paths",
    ),
    (
        "verify-context-drift-2",
        "taskboard_discuss_matches_planning",
        "verify-context-drift-2",
    ),
    (
        "verify-failure-directions-1",
        "taskboard_plan_verdicts_match",
        "verify-failure-directions",
    ),
    (
        "verify-failure-directions-2",
        "taskboard_plan_verdicts_match",
        "verify-failure-directions",
    ),
    (
        "verify-plan-structure-01-01",
        "taskboard_plan_verdicts_match",
        "verify-plan-structure-",
    ),
    (
        "verify-plan-structure-01-02",
        "taskboard_plan_verdicts_match",
        "verify-plan-structure-",
    ),
    (
        "verify-plan-structure-02-01",
        "taskboard_plan_verdicts_match",
        "verify-plan-structure-",
    ),
];

/// `(name, body)` of every `#[test]` fn of the black-box tests; a body ends at the first lone `}` of its indentation.
fn test_fns() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cli");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("tests/cli") {
        let path = entry.expect("an entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a source file");
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != "#[test]" {
                continue;
            }
            let Some(start) = (i + 1..lines.len().min(i + 4)).find(|&j| lines[j].contains("fn "))
            else {
                continue;
            };
            let head = lines[start];
            let indent = head.len() - head.trim_start().len();
            let name = head
                .split("fn ")
                .nth(1)
                .and_then(|r| r.split('(').next())
                .unwrap_or_default()
                .trim()
                .to_string();
            let close = format!("{}}}", " ".repeat(indent));
            let end = (start..lines.len())
                .find(|&j| lines[j] == close)
                .unwrap_or(lines.len() - 1);
            found.push((name, lines[start..=end].join("\n")));
        }
    }
    found
}

#[test]
fn every_taskboard_golden_is_compared_once() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../parity/goldens/taskboard");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&dir)
        .expect("the goldens")
        .map(|e| {
            let name = e
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            name.rsplit_once('.')
                .map_or(name.clone(), |(stem, _)| stem.to_string())
        })
        .collect();
    assert_eq!(on_disk.len(), 19, "the recorded goldens");
    let listed: Vec<&str> = SCENARIO.iter().map(|(g, ..)| *g).collect();
    let unique: BTreeSet<String> = listed.iter().map(|g| g.to_string()).collect();
    assert_eq!(unique.len(), listed.len(), "each golden is named once");
    assert_eq!(
        unique, on_disk,
        "the table and the directory name the same goldens"
    );
    let fns = test_fns();
    for (golden, test, needle) in SCENARIO {
        let body = fns
            .iter()
            .find(|(n, _)| n == test)
            .unwrap_or_else(|| panic!("{golden}: no test `{test}` under tests/cli"));
        assert!(
            body.1.contains(needle),
            "{golden}: `{test}` does not read it by `{needle}`"
        );
    }
}

/// The health golden is compared whole by K18's test; this only holds the table to it.
#[test]
fn validate_health_matches() {
    let row = SCENARIO
        .iter()
        .find(|(g, ..)| *g == "validate-health")
        .expect("a validate-health row");
    assert_eq!(row.1, "health_equals_the_golden_on_the_twin");
    assert!(test_fns().iter().any(|(n, _)| n == row.1));
}
