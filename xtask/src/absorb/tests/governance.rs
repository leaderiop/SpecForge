//! Old constraints become `@gsd/core` requirements (ADR 0359 D1-D6), keeping what gsd-core's requirement holds; an
//! old failure mode and an old invariant convert to nothing, their values ledgered `dropped` (ADR 0370 D8).

use serde_json::json;

use super::super::ledger::Why;
use super::super::table::{self, Home, Pending};
use super::{create, field, ledgered, old, one, planned};

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
            ("f", "title", json!("The f"), dropped.clone()),
            (
                "f",
                "verify",
                json!({"kind": "unit", "text": "a full disk refuses the write"}),
                dropped
            ),
        ],
        "one entry for the title and one per value, each with the table's reason"
    );
}

/// A constraint's quality class, priority, metric, threshold, references and sources leave the requirement (ADR 0370
/// D3): each value is one `dropped` entry, and the requirement keeps its text.
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
    let (kind, fields) = create(&planned, "c");
    assert_eq!(kind, "requirement");
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        ["category", "text", "tier"],
        "only the text, and the fills"
    );
    assert_eq!(field(&planned, "c", "text"), Some(&one("Fast.")));
    for name in [
        "governs_features",
        "metric",
        "priority",
        "protects",
        "refs",
        "threshold",
    ] {
        assert!(
            matches!(why_of(&planned, "c", name).as_slice(), [Why::Dropped(reason)] if reason.contains("ADR 0370")),
            "{name}: {:?}",
            why_of(&planned, "c", name)
        );
    }
    // The quality class and the requirement's heading share the name `category`: the class is dropped, its reason
    // naming the class, not the heading that stays, and the heading is filled with "Constraint".
    assert!(
        matches!(
            why_of(&planned, "c", "category").as_slice(),
            [Why::Dropped(reason), Why::Filled { field, value }]
                if reason.contains("quality class")
                    && reason.contains("ADR 0370")
                    && !reason.contains("requirement.category")
                    && field == "requirement.category"
                    && value == "Constraint"
        ),
        "category: {:?}",
        why_of(&planned, "c", "category")
    );
    assert!(
        matches!(why_of(&planned, "c", "metric").as_slice(), [Why::Dropped(reason)] if reason.contains("ADR 0368 D2")),
        "a bound is the requirement's text, which the conversion change writes"
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
fn a_later_field_is_ledgered_with_what_it_waits_for() {
    let mut c = old(
        "constraint",
        "c",
        json!({"description": "D", "enforced_by": ["write_entry"]}),
    );
    c.obligations = vec![(Some("load".to_string()), "benchmark it".to_string())];
    let planned = planned(&[c]);
    assert_eq!(
        why_of(&planned, "c", "enforced_by"),
        [Why::Later(Pending::Truth)]
    );
    assert_eq!(
        why_of(&planned, "c", "verify"),
        [Why::Later(Pending::Truth)]
    );
}
