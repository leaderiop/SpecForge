#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;

use super::*;

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(id, test)| (id.to_string(), test.to_string()))
        .collect()
}

fn ids(tests: &[&str]) -> BTreeSet<String> {
    tests.iter().map(|id| id.to_string()).collect()
}

#[test]
fn x2_counts_the_items_whose_test_id_a_test_fact_holds_and_names_the_rest() {
    let items = pairs(&[
        ("rust:p::a", "rust:p::a"),
        ("rust:p::b", "rust:p::b"),
        ("rust:p::c", "rust:p::c"),
        ("rust:p::d", "rust:p::d"),
    ]);
    let x2 = ratio(&items, &ids(&["rust:p::a", "rust:p::b", "rust:p::d"]));
    assert_eq!(
        lines("p", &x2),
        ["X2 p: 3/4 (75.0%)", "  rust:p::c"],
        "the ratio, then the one item no test fact saw"
    );
}

#[test]
fn x2_of_a_project_with_no_test_id_is_zero_of_zero_with_its_reason() {
    let x2 = ratio(&[], &ids(&["js:t/a.test.js::adds"]));
    assert_eq!(
        lines("taskboard", &x2),
        ["X2 taskboard: 0/0 (no item has a test_id)"],
        "no percent and no line beneath"
    );
}

#[test]
fn x2_counts_a_test_of_any_status_and_not_an_item_without_a_test_id() {
    let export = json!({"nodes": [
        {"kind": "item", "id": "rust:p::skipped", "fields": {"test_id": "rust:p::skipped"}},
        {"kind": "item", "id": "rust:p::failed", "fields": {"test_id": "rust:p::failed"}},
        {"kind": "item", "id": "rust:p::helper", "fields": {"kind": "function"}},
        {"kind": "test", "id": "rust:p::skipped", "fields": {"status": "skipped"}},
        {"kind": "test", "id": "rust:p::failed", "fields": {"status": "failed"}},
        {"kind": "requirement", "id": "R-01", "fields": {"test_id": "rust:p::helper"}},
    ]});
    let (items, tests) = read(&export).unwrap();
    assert_eq!(
        items,
        pairs(&[
            ("rust:p::skipped", "rust:p::skipped"),
            ("rust:p::failed", "rust:p::failed"),
        ]),
        "only items carry a counted test_id"
    );
    let x2 = ratio(&items, &tests);
    assert_eq!(lines("p", &x2), ["X2 p: 2/2 (100.0%)"]);
}

#[test]
fn x2_lists_the_missing_items_sorted_once_each() {
    let items = pairs(&[
        ("rust:p::z", "rust:p::z"),
        ("rust:p::a", "rust:p::a"),
        ("rust:p::m", "rust:p::m"),
        ("rust:p::a", "rust:p::a"),
    ]);
    let x2 = ratio(&items, &ids(&[]));
    assert_eq!(x2.missing, ["rust:p::a", "rust:p::m", "rust:p::z"]);
    assert_eq!(lines("p", &x2)[0], "X2 p: 0/4 (0.0%)");
}

#[test]
fn x2_rounds_the_percent_down_to_one_decimal() {
    let items = pairs(&[("a", "a"), ("b", "b"), ("c", "c")]);
    assert_eq!(
        lines("p", &ratio(&items, &ids(&["a"])))[0],
        "X2 p: 1/3 (33.3%)"
    );
    assert_eq!(
        lines("p", &ratio(&items, &ids(&["a", "b"])))[0],
        "X2 p: 2/3 (66.6%)"
    );
}

/// `present` of `total` items with a test fact, the others missing.
fn of(present: usize, total: usize) -> X2 {
    X2 {
        present,
        total,
        missing: vec!["m".to_string(); total - present],
    }
}

#[test]
fn x2_never_prints_100_percent_while_an_item_is_missing() {
    assert_eq!(
        lines("p", &of(9_999, 10_000))[0],
        "X2 p: 9999/10000 (99.9%)"
    );
    assert_eq!(lines("p", &of(1_999, 2_000))[0], "X2 p: 1999/2000 (99.9%)");
    assert_eq!(lines("p", &of(29, 100))[0], "X2 p: 29/100 (29.0%)");
    assert_eq!(lines("p", &of(7, 7)), ["X2 p: 7/7 (100.0%)"]);
}

#[test]
fn the_floor_passes_only_every_item_of_at_least_one() {
    assert_eq!(floor("p", &of(7, 7)), Ok(()));
    assert_eq!(
        floor("p", &of(0, 0)),
        Err("X2 p: no item has a test_id".to_string())
    );
    assert_eq!(
        floor("p", &of(5, 6)),
        Err("X2 p: 5/6 (83.3%): missing m".to_string())
    );
    let error = floor("p", &of(9_999, 10_000)).unwrap_err();
    assert!(
        error.starts_with("X2 p: 9999/10000 (99.9%): missing m"),
        "never rounds to a pass: {error}"
    );
}

#[test]
fn the_floor_names_ten_missing_ids_in_order_and_counts_the_rest() {
    let missing: Vec<String> = (0..25).map(|n| format!("m{n:02}")).collect();
    let x2 = X2 {
        present: 0,
        total: 25,
        missing: missing.clone(),
    };
    let error = floor("p", &x2).unwrap_err();
    assert_eq!(
        error,
        format!(
            "X2 p: 0/25 (0.0%): missing {}, and 15 more (the log lists all)",
            missing[..10].join(", ")
        )
    );
    assert_eq!(MISSING_NAMED, 10);
    assert!(!error.contains("m10"), "an eleventh id named: {error}");
}

#[test]
fn x2_refuses_an_export_of_another_shape_rather_than_reading_0_of_0() {
    for (export, named) in [
        (json!({}), "nodes"),
        (json!({"nodes": {}}), "nodes"),
        (json!({"entities": []}), "nodes"),
        (json!({"nodes": [{"kind": "item"}]}), "nodes[0]"),
        (json!({"nodes": [{"id": "x", "type": "item"}]}), "nodes[0]"),
        (json!({"nodes": [{"id": 1, "kind": "item"}]}), "nodes[0]"),
    ] {
        let error = read(&export).expect_err(named);
        assert!(error.contains(named), "{export}: {error}");
    }
    let (items, tests) = read(&json!({"nodes": []})).unwrap();
    assert!(
        items.is_empty() && tests.is_empty(),
        "an empty graph is 0/0"
    );
}
