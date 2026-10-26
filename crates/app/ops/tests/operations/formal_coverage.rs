//! @specforge/formal's coverage_tracking pass, over the vendored blob, names as coverage items exactly the entities
//! `specforge stats` counts toward coverage (ADR 0269), with and without @specforge/testing.

use super::compiled_support::{TestProject, analyzed, stats_of};
use specforge_ops::analysis::analyze::{AnalyzeOptions, ReportSource};
use specforge_test_macros::test as specforge_test;

/// Two invariants that declare an obligation: `kept` proven by the recorded test, `open` not.
const SPEC: &str = concat!(
    "invariant kept \"Kept\" {\n  guarantee \"g\"\n  verify unit \"it holds\"\n}\n",
    "invariant open \"Open\" {\n  guarantee \"g\"\n  verify unit \"it stays\"\n}\n",
);
const REPORT: &str = r#"{"runner": "manual", "results": {
    "kept": {"tests": [{"name": "holds", "status": "passed", "verify": "it holds"}]}}}"#;

fn project(extensions: &[&str]) -> TestProject {
    TestProject::new()
        .without_config()
        .raw_config(format!(
            r#"{{"extensions": {}}}"#,
            serde_json::json!(extensions)
        ))
        .file("main.spec", SPEC)
}

/// The messages of coverage_tracking's `code` findings, with REPORT recorded.
fn messages(project: &TestProject, code: &str) -> Vec<String> {
    project.write("report.json", REPORT);
    let doc = analyzed(
        &project.compile(),
        &AnalyzeOptions {
            report: ReportSource::Named {
                path: project.root().join("report.json"),
                containment: specforge_ops::Containment::unbounded(),
            },
            ..Default::default()
        },
    );
    doc["passes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["pass"] == "@specforge/formal:coverage_tracking")
        .flat_map(|p| p["findings"].as_array().unwrap().iter())
        .filter(|f| f["code"] == code)
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "fa_track_coverage_discharge",
    verify = "W035 and I008 name only the entities stats counts toward coverage"
)]
fn coverage_items_are_the_entities_stats_counts() {
    // Without testing nothing is testable: stats counts none, so formal names none.
    let bare = project(&["@specforge/formal", "@specforge/software"]);
    assert_eq!(stats_of(&bare.compile()).testable_count, 0);
    assert_eq!(messages(&bare, "W035"), Vec::<String>::new());
    assert_eq!(messages(&bare, "I008"), Vec::<String>::new());
    // With testing both invariants count: `open` is unproven, `kept` proven.
    let tested = project(&[
        "@specforge/formal",
        "@specforge/software",
        "@specforge/testing",
    ]);
    assert_eq!(stats_of(&tested.compile()).testable_count, 2);
    assert_eq!(
        messages(&tested, "W035"),
        ["1 coverage item(s) are not proven by a recorded test or an entailed claim: open"]
    );
    assert_eq!(
        messages(&tested, "I008"),
        ["invariant 'kept': all 1 obligation(s) proven by recorded test(s): holds"]
    );
}
