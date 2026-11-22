use specforge_code_facts::{finding_facts, Level};
use specforge_extension_sdk::prelude::Fact;

use super::*;

/// A clippy-sarif log, hand-written to SARIF 2.1.0 and the tool's shape: crate `demo` at `/project`.
const CLIPPY: &str = include_str!("../fixtures/clippy.sarif");
/// An @microsoft/eslint-formatter-sarif log, hand-written: package `board` at `/project`.
const ESLINT: &str = include_str!("../fixtures/eslint.sarif");
/// Results stating no rule or no level, hand-written to SARIF 2.1.0 §3.27.10.
const RULE_LESS: &str = include_str!("../fixtures/rule-less.sarif");
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

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f str> {
    fact.fields.get(name).and_then(|v| v.as_str())
}

#[test]
fn an_eslint_report_mints_static_findings_citing_their_js_items() {
    let results = read(ESLINT, "/project").expect("the report reads");
    assert!(results
        .iter()
        .all(|r| r.file.as_deref() == Some("src/board view.js")));
    let facts = finding_facts(&results, &served());
    let cited: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|f| (field(f, "rule").unwrap_or_default(), field(f, "item")))
        .collect();
    let item = |name: &str| Some(format!("js:board/src/board%20view.js::{name}"));
    let expected = [
        ("no-unused-vars", item("render")),
        ("eqeqeq", item("View.draw")),
        ("local/no-raw-dom", item("View")),
        ("no-console", item("render")),
    ];
    let expected: Vec<(&str, Option<&str>)> = expected
        .iter()
        .map(|(rule, item)| (*rule, item.as_deref()))
        .collect();
    assert_eq!(cited, expected);
    assert_eq!(
        facts[2].id,
        "sarif:ESLint/local%2Fno-raw-dom#src%2Fboard%20view.js:18"
    );
    insta::assert_json_snapshot!("eslint", facts);
}

#[test]
fn a_result_stating_no_rule_or_level_is_read_as_unknown() {
    let results = read(RULE_LESS, "/project").expect("the report reads");
    let read: Vec<(&str, Level)> = results.iter().map(|r| (r.rule.as_str(), r.level)).collect();
    assert_eq!(
        read,
        [
            ("unknown", Level::Error),
            ("R1", Level::Unknown),
            ("R1", Level::None),
            ("R2", Level::Note),
            ("R2", Level::Note),
            ("R1", Level::Warning),
            ("R1", Level::Warning),
        ]
    );
    assert_eq!((results[4].file.as_deref(), results[4].line), (None, None));
    let facts = finding_facts(&results, &served());
    let ids: Vec<&str> = facts.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids[4..],
        [
            "sarif:demo-lint/R2",
            "sarif:demo-lint/R1#src%2Flib.rs:24",
            "sarif:demo-lint/R1#src%2Flib.rs:24~2"
        ]
    );
    insta::assert_json_snapshot!("rule_less", facts);
}

#[test]
fn a_file_uri_loses_its_fragment_and_a_relative_one_reads_from_the_root() {
    let log = |uri: &str| {
        format!(
            r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "t"}}}}, "results": [{{
                "locations": [{{"physicalLocation": {{"artifactLocation": {{"uri": "{uri}"}}}}}}]}}]}}]}}"#
        )
    };
    for uri in [
        "file:///project/src/lib.rs#L3",
        "file://localhost/project/src/lib.rs",
        "./src/lib.rs",
    ] {
        let results = read(&log(uri), "/project").expect(uri);
        assert_eq!(results[0].file.as_deref(), Some("src/lib.rs"), "{uri}");
    }
}

#[test]
fn a_bad_report_is_an_error_naming_its_value_or_path_and_reads_nothing() {
    let run = |run: &str| format!(r#"{{"version": "2.1.0", "runs": [{run}]}}"#);
    let at = |uri: &str| {
        run(&format!(
            r#"{{"tool": {{"driver": {{"name": "t"}}}}, "results": [{{"locations": [{{"physicalLocation":
                {{"artifactLocation": {{"uri": "{uri}"}}}}}}]}}]}}"#
        ))
    };
    let deep = format!("{}{}", "[".repeat(10_000), "]".repeat(10_000));
    let rows: Vec<(String, &str)> = vec![
        ("no json at all".to_string(), "no JSON"),
        (deep, "no JSON"),
        (r#"{"version": "2.0.0", "runs": []}"#.to_string(), "2.0.0"),
        (r#"{"runs": []}"#.to_string(), "version"),
        (r#"{"version": "2.1.0"}"#.to_string(), "runs"),
        (run(r#"{"results": []}"#), "runs[0].tool.driver.name"),
        (
            run(r#"{"tool": {"driver": {"name": "t"}}}"#),
            "runs[0].results",
        ),
        (
            run(r#"{"tool": {"driver": {"name": "t"}}, "results": {}}"#),
            "runs[0].results",
        ),
        (
            run(r#"{"tool": {"driver": {"name": "t"}}, "results": [{"ruleIndex": 5}]}"#),
            "rule index 5",
        ),
        (
            run(
                r#"{"tool": {"driver": {"name": "t"}}, "results": [{"locations": [{"physicalLocation":
                    {"artifactLocation": {"index": 3}}}]}]}"#,
            ),
            "artifact index 3",
        ),
        (at("file:///elsewhere/a.js"), "file:///elsewhere/a.js"),
        (at("%2E%2E/%2E%2E/etc/passwd"), "%2E%2E/%2E%2E/etc/passwd"),
        (at("file://server/share/a.js"), "file://server/share/a.js"),
        (at("https://example.com/a.js"), "https://example.com/a.js"),
    ];
    for (text, named) in &rows {
        let error = read(text, "/project").expect_err(named);
        assert!(error.contains(named), "{error} should name {named}");
    }
}

#[test]
fn an_error_shows_the_reports_text_bounded_and_escaped() {
    let hostile = format!("file:///elsewhere/\\u001b]0;{}\\u0007", "a".repeat(5_000));
    let log = format!(
        r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "t"}}}}, "results": [{{
            "locations": [{{"physicalLocation": {{"artifactLocation": {{"uri": "{hostile}"}}}}}}]}}]}}]}}"#
    );
    let error = read(&log, "/project").expect_err("outside the project");
    assert!(
        !error.contains(['\u{1b}', '\u{7}']) && error.contains("\\u{1b}"),
        "{error}"
    );
    assert!(error.len() < 400, "{} bytes", error.len());
    let version = format!(r#"{{"version": "{}", "runs": []}}"#, "9".repeat(5_000));
    assert!(read(&version, "/project").expect_err("version").len() < 400);
}

#[test]
fn an_empty_results_array_is_a_run_that_found_nothing() {
    let log =
        r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "t"}}, "results": []}]}"#;
    assert_eq!(read(log, "/project"), Ok(Vec::new()));
}
