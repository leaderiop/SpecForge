//! Old constraints, failure modes and invariants become `@gsd/core` requirements, risks and invariants (ADR 0358
//! D8-D9, ADR 0359 D1-D6).

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Input;
use std::collections::BTreeMap;

use super::super::how;
use super::super::ledger::Why;
use super::super::table::{Pending, Scale};
use super::{field, ledgered, many, old, one, planned};

fn number(n: u64) -> Input {
    Input::Number(n.into())
}

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

/// A failure mode rated `severity` `occurrence` `detection`, plus `more`.
fn failure(id: &str, more: Value) -> super::super::read::OldEntity {
    let mut fields = json!({
        "severity": "high", "occurrence": "likely", "detection": "moderate",
        "cause": "C", "effect": "E",
    });
    for (k, v) in more.as_object().unwrap() {
        fields[k] = v.clone();
    }
    old("failure_mode", id, fields)
}

fn why_of(planned: &super::super::plan::Planned, id: &str, field: &str) -> Vec<Why> {
    ledgered(planned, id)
        .into_iter()
        .filter(|(f, _)| f == field)
        .map(|(_, why)| why)
        .collect()
}

#[test]
fn severity_occurrence_and_detection_words_map_to_the_adr_0358_numbers() {
    let table: [(Scale, &[(&str, u64)]); 3] = [
        (
            Scale::Severity,
            &[("critical", 10), ("high", 7), ("medium", 5), ("low", 2)],
        ),
        (
            Scale::Occurrence,
            &[
                ("certain", 10),
                ("likely", 7),
                ("occasional", 5),
                ("unlikely", 3),
                ("rare", 1),
            ],
        ),
        (
            Scale::Detection,
            &[
                ("certain", 1),
                ("likely", 3),
                ("moderate", 5),
                ("unlikely", 7),
                ("undetectable", 10),
            ],
        ),
    ];
    for (scale, words) in table {
        for (word, n) in words {
            assert_eq!(how::rating(scale, &json!(word)), Ok(*n), "{word}");
        }
        assert!(how::rating(scale, &json!("sometimes")).is_err());
    }
    let planned = planned(&[failure("f", json!({}))]);
    assert_eq!(field(&planned, "f", "severity"), Some(&number(7)));
    assert_eq!(field(&planned, "f", "occurrence"), Some(&number(7)));
    assert_eq!(field(&planned, "f", "detection"), Some(&number(5)));
}

#[test]
fn rpn_is_never_written_and_a_different_value_is_ledgered() {
    let mut planned = planned(&[
        failure("same", json!({"rpn": 245})),
        failure("other", json!({"rpn": 90})),
    ]);
    for id in ["same", "other"] {
        assert_eq!(field(&planned, id, "rpn"), None, "{id}");
    }
    planned.compare(|_, field| match field {
        "rpn" => json!(245),
        _ => Value::Null,
    });
    assert_eq!(why_of(&planned, "same", "rpn"), []);
    assert_eq!(
        why_of(&planned, "other", "rpn"),
        [Why::Computed {
            field: "risk.rpn".to_string(),
            new: json!(245)
        }]
    );
}

#[test]
fn a_missing_occurrence_or_detection_is_filled_at_ten() {
    let planned = planned(&[old(
        "failure_mode",
        "f",
        json!({"severity": "low", "cause": "C", "effect": "E"}),
    )]);
    assert_eq!(field(&planned, "f", "occurrence"), Some(&number(10)));
    assert_eq!(field(&planned, "f", "detection"), Some(&number(10)));
    assert_eq!(
        why_of(&planned, "f", "occurrence"),
        [Why::Filled {
            field: "risk.occurrence".to_string(),
            value: json!(10)
        }]
    );
}

#[test]
fn post_mitigation_becomes_the_post_block() {
    let block =
        json!({"severity": "medium", "occurrence": "rare", "detection": "likely", "rpn": 12});
    let mut planned = planned(&[failure(
        "f",
        json!({"post_mitigation": block, "post_severity": "low"}),
    )]);
    let post = BTreeMap::from([
        ("severity".to_string(), number(5)),
        ("occurrence".to_string(), number(1)),
        ("detection".to_string(), number(3)),
    ]);
    assert_eq!(field(&planned, "f", "post"), Some(&Input::Block(post)));
    assert_eq!(
        why_of(&planned, "f", "post_severity"),
        [Why::Unconverted(
            "post_mitigation's severity is written in its place".to_string()
        )],
        "the block wins over a post_ field"
    );
    planned.compare(|_, field| match field {
        "post_rpn" => json!(15),
        _ => Value::Null,
    });
    assert_eq!(
        why_of(&planned, "f", "post_mitigation"),
        [Why::Computed {
            field: "risk.post_rpn".to_string(),
            new: json!(15)
        }],
        "its rpn is compared with the computed post_rpn"
    );
}

#[test]
fn failure_mode_invariant_becomes_a_mitigation() {
    let planned = planned(&[
        failure("f", json!({"invariant": "durable"})),
        old("invariant", "durable", json!({"guarantee": "G"})),
    ]);
    assert_eq!(
        field(&planned, "f", "mitigated_by"),
        Some(&many(&["durable"]))
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
fn failure_mode_and_invariant_description_and_refs_are_kept() {
    let planned = planned(&[
        failure(
            "f",
            json!({"description": "Why it fails", "refs": ["Worksheet 3", "gh.issue:42"]}),
        ),
        old(
            "invariant",
            "i",
            json!({"guarantee": "G", "description": "Why it holds", "refs": ["Design note"]}),
        ),
    ]);
    assert_eq!(
        field(&planned, "f", "description"),
        Some(&one("Why it fails"))
    );
    assert_eq!(field(&planned, "f", "refs"), Some(&many(&["Worksheet 3"])));
    assert_eq!(
        why_of(&planned, "f", "refs"),
        [Why::Unconverted(how::SCHEME_REF.to_string())],
        "a scheme reference would name a ref the target lacks"
    );
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
fn a_reference_to_a_behavior_is_ledgered_with_what_it_waits_for() {
    let entities = [failure("f", json!({"affected_behaviors": ["write_entry"]}))];
    let mut index = super::index(&entities);
    index
        .0
        .insert("write_entry".to_string(), "behavior".to_string());
    super::super::plan::prechecks(&entities).unwrap();
    let planned = super::super::plan::plan(&entities, &[], &index, &Default::default());
    assert_eq!(field(&planned, "f", "affects"), None, "no empty list");
    assert_eq!(
        why_of(&planned, "f", "affected_behaviors"),
        [Why::TargetLater {
            target: "write_entry".to_string(),
            pending: Pending::Behavior
        }]
    );
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
