//! Old constraints and invariants become `@gsd/core` requirements (ADR 0359 D1-D6, ADR 0372 D6), keeping what
//! gsd-core's requirement holds; an old failure mode is dropped whole, its values ledgered `dropped` (ADR 0372 D1).

use serde_json::json;

use super::super::ledger::Why;
use super::super::table::{self, Fate};
use super::{create_of, field_of, ledgered, old, one, planned};

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
    let Some(Fate::Dropped(reason)) = table::fate("failure_mode") else {
        panic!("a failure mode is dropped whole");
    };
    assert!(reason.contains("ADR 0372 D1"), "{reason}");
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
            ("f", "title", json!("The f"), dropped.clone()),
            (
                "f",
                "verify",
                json!({"kind": "unit", "text": "a full disk refuses the write"}),
                dropped
            ),
        ],
        "one entry for the title and one per value, each with its fate's reason"
    );
}

/// A constraint's priority, references and sources leave the requirement (ADR 0370 D3): each value is one `dropped`
/// entry, and the requirement keeps its text, its bound in it, and its quality class as its heading (ADR 0372 D3).
#[test]
fn a_constraint_keeps_its_text_and_drops_the_fields_that_left() {
    let planned = planned(&[old(
        "constraint",
        "c",
        json!({
            "description": "Fast.", "category": "performance", "priority": "high",
            "metric": "latency <= 95.0ms", "threshold": "< 100ms", "governs_features": ["store"],
            "protects": ["durable"], "refs": ["Design note"],
        }),
    )]);
    let (kind, fields) = create_of(&planned, "constraint", "c");
    assert_eq!(kind, "requirement");
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        ["category", "text", "tier"],
        "only the text, and the fills"
    );
    assert_eq!(
        field_of(&planned, "constraint", "c", "text"),
        Some(&one("Fast.\n\nlatency <= 95.0ms < 100ms"))
    );
    for name in ["governs_features", "priority", "protects", "refs"] {
        assert!(
            matches!(why_of(&planned, "c", name).as_slice(), [Why::Dropped(reason)] if reason.contains("ADR 0370")),
            "{name}: {:?}",
            why_of(&planned, "c", name)
        );
    }
    // The quality class is the requirement's heading, by the rule that also gives its id (ADR 0372 D3): written,
    // so nothing is ledgered for it.
    assert_eq!(
        field_of(&planned, "constraint", "c", "category"),
        Some(&one("Performance"))
    );
    assert_eq!(why_of(&planned, "c", "category"), []);
}

/// A bound is part of the requirement's text (ADR 0368 D2, ADR 0372 D6): the description, a blank line, the metric as
/// written, the threshold appended to its last line after one space; nothing is ledgered for either.
#[test]
fn a_constraint_text_holds_its_description_then_its_metric_and_threshold() {
    let planned = planned(&[
        old(
            "constraint",
            "both",
            json!({"description": "D", "metric": "throughput", "threshold": ">= 200"}),
        ),
        old(
            "constraint",
            "lines",
            json!({"description": "D", "metric": "latency <= 95.0ms\nmeasured at the 99th percentile"}),
        ),
        old(
            "constraint",
            "bare",
            json!({"description": "D", "threshold": ">= 200"}),
        ),
    ]);
    for (id, text) in [
        ("both", "D\n\nthroughput >= 200"),
        (
            "lines",
            "D\n\nlatency <= 95.0ms\nmeasured at the 99th percentile",
        ),
        ("bare", "D >= 200"),
    ] {
        assert_eq!(
            field_of(&planned, "constraint", id, "text"),
            Some(&one(text)),
            "{id}"
        );
        for name in ["metric", "threshold"] {
            assert_eq!(why_of(&planned, id, name), [], "{id}.{name}");
        }
    }
}

/// Every value of `id`'s `fields` is one `dropped` entry whose reason holds each of `naming`.
fn dropped_naming(
    planned: &super::super::plan::Planned,
    id: &str,
    fields: &[&str],
    naming: &[&str],
) {
    for name in fields {
        assert!(
            matches!(
                why_of(planned, id, name).as_slice(),
                [Why::Dropped(reason)] if naming.iter().all(|n| reason.contains(n))
            ),
            "{name}: {:?}",
            why_of(planned, id, name)
        );
    }
}

#[test]
fn constraint_constrains_enforced_by_and_verify_are_dropped_with_their_reasons() {
    let mut c = old(
        "constraint",
        "c",
        json!({"description": "D", "constrains": ["write_entry"], "enforced_by": ["write_entry"]}),
    );
    c.obligations = vec![(Some("load".to_string()), "benchmark it".to_string())];
    let planned = planned(&[c]);
    assert_eq!(field_of(&planned, "constraint", "c", "constrains"), None);
    dropped_naming(
        &planned,
        "c",
        &["constrains", "enforced_by"],
        &["ADR 0364 D2", "ADR 0372 D6"],
    );
    dropped_naming(&planned, "c", &["verify"], &["ADR 0368 D1", "ADR 0372 D6"]);
}

#[test]
fn an_invariant_becomes_a_requirement_whose_text_is_its_guarantee_then_its_description() {
    let mut invariant = old(
        "invariant",
        "durable",
        json!({
            "guarantee": "An acknowledged write survives a crash.", "risk": "high",
            "description": "No reader sees half an entry.", "refs": ["Storage note"], "expression": "forall w. durable(w)",
        }),
    );
    invariant.obligations = vec![(Some("unit".to_string()), "survives a crash".to_string())];
    let planned = planned(&[invariant]);
    assert_eq!(planned.converted, 1);
    let (kind, fields) = create_of(&planned, "invariant", "durable");
    assert_eq!(kind, "requirement");
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        ["category", "text", "tier"],
        "{fields:?}"
    );
    assert_eq!(
        field_of(&planned, "invariant", "durable", "text"),
        Some(&one(
            "An acknowledged write survives a crash.\n\nNo reader sees half an entry."
        ))
    );
    for (name, value) in [("category", "Invariant"), ("tier", "v1")] {
        assert_eq!(
            field_of(&planned, "invariant", "durable", name),
            Some(&one(value))
        );
        assert_eq!(
            why_of(&planned, "durable", name),
            [Why::Filled {
                field: format!("requirement.{name}"),
                value: json!(value)
            }]
        );
    }
    dropped_naming(
        &planned,
        "durable",
        &["risk", "refs", "expression", "verify"],
        &["ADR 0372 D6"],
    );
    dropped_naming(&planned, "durable", &["expression"], &["ADR 0368 D3"]);
    assert_eq!(why_of(&planned, "durable", "guarantee"), []);
    assert_eq!(why_of(&planned, "durable", "description"), []);
}
