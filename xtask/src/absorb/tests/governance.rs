//! Old constraints and invariants become `@gsd/core` requirements and invariants (ADR 0358 D9, ADR 0359 D1-D6); an
//! old failure mode converts to nothing, its values ledgered `dropped` (ADR 0370 D8).

use serde_json::json;
use specforge_ops::authoring::edit::Input;

use super::super::how;
use super::super::ledger::Why;
use super::super::table::{self, Home, Pending};
use super::{field, ledgered, many, old, one, planned};

fn entries(pairs: &[(&str, &str)]) -> Input {
    Input::Entries(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}

/// A constraint writing its description and `metric` (and `threshold`, when given).
fn constraint(id: &str, metric: &str, threshold: Option<&str>) -> super::super::read::OldEntity {
    let mut fields = json!({"description": "Fast.", "metric": metric});
    if let Some(threshold) = threshold {
        fields["threshold"] = json!(threshold);
    }
    old("constraint", id, fields)
}

fn why_of(planned: &super::super::plan::Planned, id: &str, field: &str) -> Vec<Why> {
    ledgered(planned, id)
        .into_iter()
        .filter(|(f, _)| f == field)
        .map(|(_, why)| why)
        .collect()
}

#[test]
fn a_failure_mode_converts_to_nothing_and_ledgers_each_value_dropped() {
    let mut f = old(
        "failure_mode",
        "f",
        json!({"severity": "high", "cause": "The disk fills."}),
    );
    f.obligations = vec![(
        Some("unit".to_string()),
        "a full disk refuses the write".to_string(),
    )];
    let planned = planned(&[f]);
    assert!(planned.creates.is_empty(), "{:?}", planned.creates);
    assert_eq!(planned.converted, 0);
    let &[Home::Dropped(reason)] = table::row_of("failure_mode", "severity").unwrap().homes else {
        panic!("a failure mode's severity is dropped");
    };
    assert!(reason.contains("ADR 0370"), "{reason}");
    let dropped = Why::Dropped(reason.to_string());
    let entries: Vec<_> = planned
        .ledger
        .entries()
        .map(|e| {
            (
                e.id.as_str(),
                e.field.as_str(),
                e.value.clone(),
                e.why.clone(),
            )
        })
        .collect();
    assert_eq!(
        entries,
        [
            ("f", "cause", json!("The disk fills."), dropped.clone()),
            ("f", "severity", json!("high"), dropped.clone()),
            (
                "f",
                "verify",
                json!({"kind": "unit", "text": "a full disk refuses the write"}),
                dropped
            ),
        ],
        "one entry per value, each with the table's reason"
    );
}

#[test]
fn a_bound_line_becomes_the_metric_block() {
    let planned = planned(&[constraint("c", "latency <= 95.0ms", None)]);
    assert_eq!(
        field(&planned, "c", "metric"),
        Some(&entries(&[
            ("name", "latency"),
            ("threshold", "95.0"),
            ("unit", "ms"),
            ("direction", "at_most"),
        ]))
    );
    assert_eq!(field(&planned, "c", "text"), Some(&one("Fast.")));
    assert_eq!(why_of(&planned, "c", "metric"), []);
}

#[test]
fn a_threshold_keeps_its_written_text() {
    for (line, threshold, unit) in [
        ("latency <= 95.0ms", "95.0", Some("ms")),
        ("load >= 1.50s", "1.50", Some("s")),
        ("ratio == 0.250", "0.250", None),
    ] {
        let metric = how::metric(line, None);
        let record = metric.record.expect(line);
        assert_eq!(record["threshold"], threshold, "{line}");
        assert_eq!(record.get("unit").map(String::as_str), unit, "{line}");
    }
}

#[test]
fn a_strict_bound_is_collapsed_and_ledgered() {
    let planned = planned(&[
        constraint("below", "wait < 30s", None),
        constraint("above", "uptime > 99.9", None),
    ]);
    assert_eq!(
        field(&planned, "below", "metric"),
        Some(&entries(&[
            ("name", "wait"),
            ("threshold", "30"),
            ("unit", "s"),
            ("direction", "at_most"),
        ]))
    );
    assert_eq!(
        why_of(&planned, "above", "metric"),
        [Why::Collapsed {
            field: "requirement.metric.direction".to_string(),
            into: json!("at_least")
        }]
    );
}

#[test]
fn the_other_lines_of_a_metric_join_the_text() {
    let planned = planned(&[constraint(
        "c",
        "latency <= 95.0ms\nmeasured at the 99th percentile\nof a warm cache",
        None,
    )]);
    assert_eq!(
        field(&planned, "c", "text"),
        Some(&one(
            "Fast.\n\nmeasured at the 99th percentile\nof a warm cache"
        ))
    );
    assert!(field(&planned, "c", "metric").is_some());
}

#[test]
fn a_metric_without_one_bound_line_is_written_into_the_text() {
    let planned = planned(&[
        constraint("prose", "Fast enough for a person.", None),
        constraint("two", "a <= 1\nb <= 2", None),
        constraint("unequal", "errors != 0", None),
    ]);
    for (id, text, why) in [
        (
            "prose",
            "Fast.\n\nFast enough for a person.",
            "no line is one bound `<name> <op> <number>`; every line joins the text",
        ),
        (
            "two",
            "Fast.\n\na <= 1\nb <= 2",
            "2 bound lines, and a metric holds one; every line joins the text",
        ),
        (
            "unequal",
            "Fast.\n\nerrors != 0",
            "a `!=` line bounds no direction; every line joins the text",
        ),
    ] {
        assert_eq!(field(&planned, id, "metric"), None, "{id}");
        assert_eq!(field(&planned, id, "text"), Some(&one(text)), "{id}");
        assert_eq!(
            why_of(&planned, id, "metric"),
            [Why::Unconverted(why.to_string())],
            "{id}"
        );
    }
}

#[test]
fn a_unitless_bound_becomes_a_metric_without_unit() {
    let planned = planned(&[constraint("c", "retries <= 3", None)]);
    assert_eq!(
        field(&planned, "c", "metric"),
        Some(&entries(&[
            ("name", "retries"),
            ("threshold", "3"),
            ("direction", "at_most"),
        ]))
    );
}

#[test]
fn a_threshold_field_completes_a_one_name_metric() {
    let planned = planned(&[
        constraint("one_name", "throughput", Some(">= 200")),
        constraint("own_bound", "latency <= 5ms", Some("< 10ms")),
    ]);
    assert_eq!(
        field(&planned, "one_name", "metric"),
        Some(&entries(&[
            ("name", "throughput"),
            ("threshold", "200"),
            ("direction", "at_least"),
        ]))
    );
    assert_eq!(why_of(&planned, "one_name", "threshold"), []);
    assert_eq!(
        field(&planned, "own_bound", "text"),
        Some(&one("Fast.\n\nthreshold < 10ms")),
        "a threshold no one-name metric takes joins the text"
    );
    assert_eq!(
        why_of(&planned, "own_bound", "threshold"),
        [Why::Unconverted(
            "no one-name metric it completes; it joins the text".to_string()
        )]
    );
}

#[test]
fn constraint_constrains_is_ledgered_until_behaviors_have_a_home() {
    let planned = planned(&[old(
        "constraint",
        "c",
        json!({"description": "D", "constrains": ["write_entry"]}),
    )]);
    assert_eq!(field(&planned, "c", "constrains"), None);
    assert_eq!(
        why_of(&planned, "c", "constrains"),
        [Why::Later(Pending::Behavior)]
    );
}

#[test]
fn invariant_description_and_refs_are_kept() {
    let planned = planned(&[old(
        "invariant",
        "i",
        json!({"guarantee": "G", "description": "Why it holds", "refs": ["Design note"]}),
    )]);
    assert_eq!(
        field(&planned, "i", "description"),
        Some(&one("Why it holds"))
    );
    assert_eq!(field(&planned, "i", "refs"), Some(&many(&["Design note"])));
}

#[test]
fn interoperability_collapses_into_compatibility() {
    let planned = planned(&[
        old(
            "constraint",
            "a",
            json!({"description": "D", "category": "interoperability"}),
        ),
        old(
            "constraint",
            "b",
            json!({"description": "D", "category": "security"}),
        ),
        old(
            "constraint",
            "c",
            json!({"description": "D", "category": "command"}),
        ),
    ]);
    assert_eq!(
        field(&planned, "a", "category"),
        Some(&one("compatibility"))
    );
    assert_eq!(
        why_of(&planned, "a", "category"),
        [Why::Collapsed {
            field: "requirement.category".to_string(),
            into: json!("compatibility")
        }]
    );
    assert_eq!(field(&planned, "b", "category"), Some(&one("security")));
    assert_eq!(why_of(&planned, "b", "category"), []);
    assert_eq!(field(&planned, "c", "category"), None);
    assert!(matches!(
        why_of(&planned, "c", "category").as_slice(),
        [Why::Unconverted(_)]
    ));
}

#[test]
fn priority_medium_and_low_collapse_to_could() {
    let words = ["critical", "high", "medium", "low"];
    let entities: Vec<_> = words
        .iter()
        .map(|w| old("constraint", w, json!({"description": "D", "priority": w})))
        .collect();
    let planned = planned(&entities);
    for (word, written, collapsed) in [
        ("critical", "must", false),
        ("high", "should", false),
        ("medium", "could", true),
        ("low", "could", true),
    ] {
        assert_eq!(field(&planned, word, "priority"), Some(&one(written)));
        assert_eq!(
            !why_of(&planned, word, "priority").is_empty(),
            collapsed,
            "{word}"
        );
    }
}

#[test]
fn a_later_field_is_ledgered_with_what_it_waits_for() {
    let mut c = old(
        "constraint",
        "c",
        json!({"description": "D", "enforced_by": ["write_entry"]}),
    );
    c.obligations = vec![(Some("load".to_string()), "benchmark it".to_string())];
    let i = old(
        "invariant",
        "i",
        json!({"guarantee": "G", "expression": "a <= b"}),
    );
    let planned = planned(&[c, i]);
    assert_eq!(
        why_of(&planned, "c", "enforced_by"),
        [Why::Later(Pending::Truth)]
    );
    assert_eq!(
        why_of(&planned, "c", "verify"),
        [Why::Later(Pending::Truth)]
    );
    assert_eq!(
        why_of(&planned, "i", "expression"),
        [Why::Later(Pending::Truth)]
    );
}

#[test]
fn invariant_risk_becomes_its_criticality() {
    let planned = planned(&[old(
        "invariant",
        "i",
        json!({"guarantee": "Always", "risk": "high"}),
    )]);
    assert_eq!(field(&planned, "i", "criticality"), Some(&one("high")));
    assert_eq!(field(&planned, "i", "statement"), Some(&one("Always")));
    assert_eq!(field(&planned, "i", "risk"), None);
}
