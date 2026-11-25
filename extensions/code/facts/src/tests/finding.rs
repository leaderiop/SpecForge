//! The `static_finding` facts minted from a SARIF reader's results and the served items and packages.

use crate::*;
use specforge_extension_sdk::prelude::{Fact, FactLocation};

fn item(id: &str, file: &str, start: usize, end: usize) -> Fact {
    Fact::new("item", id).at(FactLocation {
        file: file.to_string(),
        start_line: start,
        start_col: 1,
        end_line: end,
        end_col: 2,
    })
}

/// `fn add` at lines 1-5 of `src/lib.rs`, and the root package.
fn served() -> Vec<Fact> {
    vec![
        item("rust:demo::add", "src/lib.rs", 1, 5),
        Fact::new("package", "pkg:cargo/demo").field("path", ""),
    ]
}

fn result(rule: &str, file: Option<&str>, line: Option<usize>) -> SarifResult {
    SarifResult {
        tool: "clippy".to_string(),
        rule: rule.to_string(),
        level: Level::Warning,
        message: "a message".to_string(),
        file: file.map(str::to_string),
        line,
        end_line: None,
    }
}

fn field<'f>(fact: &'f Fact, name: &str) -> Option<&'f str> {
    fact.fields.get(name).and_then(|v| v.as_str())
}

#[test]
fn a_finding_cites_its_item_else_its_package_else_nothing() {
    let results = [
        result("r", Some("src/lib.rs"), Some(3)),
        result("r", Some("src/lib.rs"), Some(40)),
        result("r", Some("src/lib.rs"), None),
        result("r", None, None),
    ];
    let facts = finding_facts(&results, &served());
    let cited: Vec<(&str, Option<&str>, Option<&str>)> = facts
        .iter()
        .map(|f| (f.id.as_str(), field(f, "item"), field(f, "package")))
        .collect();
    assert_eq!(
        cited,
        [
            (
                "sarif:clippy/r#src%2Flib.rs:3",
                Some("rust:demo::add"),
                None
            ),
            (
                "sarif:clippy/r#src%2Flib.rs:40",
                None,
                Some("pkg:cargo/demo")
            ),
            ("sarif:clippy/r#src%2Flib.rs", None, Some("pkg:cargo/demo")),
            ("sarif:clippy/r", None, None),
        ]
    );
    assert!(facts.iter().all(|f| f.kind == "static_finding"));
    assert!(!facts[3].fields.contains_key("file") && facts[3].at.is_empty());
}

#[test]
fn a_finding_holds_what_the_report_says_and_spans_whole_lines() {
    let mut read = result("clippy::unwrap_used", Some("src/lib.rs"), Some(3));
    read.end_line = Some(4);
    read.level = Level::Unknown;
    let fact = &finding_facts(&[read], &served())[0];
    assert_eq!(
        ["tool", "rule", "level", "message", "file"].map(|name| field(fact, name)),
        [
            Some("clippy"),
            Some("clippy::unwrap_used"),
            Some("unknown"),
            Some("a message"),
            Some("src/lib.rs")
        ]
    );
    assert_eq!(fact.fields.get("line").and_then(|v| v.as_u64()), Some(3));
    assert_eq!(
        fact.at,
        [FactLocation {
            file: "src/lib.rs".to_string(),
            start_line: 3,
            start_col: 1,
            end_line: 5,
            end_col: 1,
        }]
    );
}

#[test]
fn two_results_at_one_place_take_a_repeat_suffix_in_document_order() {
    let at = result("r", Some("src/lib.rs"), Some(3));
    let ids: Vec<String> = finding_facts(&[at.clone(), at.clone(), at], &[])
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(
        ids,
        [
            "sarif:clippy/r#src%2Flib.rs:3",
            "sarif:clippy/r#src%2Flib.rs:3~2",
            "sarif:clippy/r#src%2Flib.rs:3~3"
        ]
    );
}

#[test]
fn a_rule_holding_a_separator_cannot_forge_another_id() {
    let forged = result("a#src/lib.rs:3", None, None);
    let honest = result("a", Some("src/lib.rs"), Some(3));
    let ids: Vec<String> = finding_facts(&[forged, honest], &[])
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(
        ids,
        [
            "sarif:clippy/a%23src%2Flib.rs%3A3",
            "sarif:clippy/a#src%2Flib.rs:3"
        ]
    );
}

#[test]
fn a_level_reads_sarifs_four_values_only() {
    assert_eq!(
        Level::NAMES,
        ["error", "warning", "note", "none", "unknown"]
    );
    for name in &Level::NAMES[..4] {
        assert_eq!(Level::parse(name).map(Level::as_str), Some(*name));
    }
    assert_eq!(Level::parse("unknown"), None);
    assert_eq!(Level::parse("Warning"), None);
    assert_eq!(Level::Unknown.as_str(), UNKNOWN);
}
