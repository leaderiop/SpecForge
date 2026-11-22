use specforge_code_facts::{finding_facts, Level};
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A clippy-sarif log, hand-written to SARIF 2.1.0 and the tool's shape: crate `demo` at `/project`.
const CLIPPY: &str = include_str!("../fixtures/clippy.sarif");
/// The served `item` and `package` facts a lint collector receives, as JSON.
const ITEMS: &str = include_str!("../fixtures/items.json");

fn served() -> Vec<Fact> {
    serde_json::from_str(ITEMS).expect("the served facts parse")
}

#[test]
fn a_clippy_report_mints_static_findings_citing_their_items() {
    let results = read(CLIPPY, "/project").expect("the report reads");
    let facts = finding_facts(&results, &served());
    let cited: Vec<(Option<&str>, Option<&str>)> = facts
        .iter()
        .map(|f| {
            let field = |name: &str| f.fields.get(name).and_then(|v| v.as_str());
            (field("item"), field("package"))
        })
        .collect();
    assert_eq!(
        cited,
        [
            (Some("rust:demo::Board::len"), None),
            (Some("rust:demo::tests::reads"), None),
            (None, Some("pkg:cargo/demo")),
        ]
    );
    insta::assert_json_snapshot!("clippy", facts);
}

#[test]
fn a_clippy_result_reads_its_rule_level_message_and_lines() {
    let results = read(CLIPPY, "/project").expect("the report reads");
    assert_eq!(
        results[1],
        SarifResult {
            tool: "clippy".to_string(),
            rule: "clippy::unwrap_used".to_string(),
            level: Level::Warning,
            message: "used `unwrap()` on a `Result` value".to_string(),
            file: Some("src/lib.rs".to_string()),
            line: Some(16),
            end_line: Some(17),
        }
    );
}
