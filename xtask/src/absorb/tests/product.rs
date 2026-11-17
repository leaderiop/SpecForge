//! An old feature becomes a `@gsd/core` requirement (ADR 0359), keeping what gsd-core's requirement holds; an old
//! deliverable, module, journey, persona, term and channel are dropped whole (ADR 0372 D1). Releases and milestones are
//! `roadmap.rs`'s.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Edit;

use super::super::ledger::Why;
use super::super::plan::Planned;
use super::super::table::{self, Fate};
use super::{create_of, field_of, ledgered, old, one, planned};

fn why_of(planned: &Planned, id: &str, field: &str) -> Vec<Why> {
    ledgered(planned, id)
        .into_iter()
        .filter(|(f, _)| f == field)
        .map(|(_, why)| why)
        .collect()
}

fn feature(id: &str, more: serde_json::Value) -> super::super::read::OldEntity {
    let mut fields = json!({"problem": "P", "status": "accepted"});
    for (k, v) in more.as_object().unwrap() {
        fields[k] = v.clone();
    }
    old("feature", id, fields)
}

fn ids(planned: &Planned) -> Vec<&str> {
    planned
        .creates
        .iter()
        .map(|edit| match edit {
            Edit::Create { id, .. } => id.as_str(),
            other => panic!("only creates: {other:?}"),
        })
        .collect()
}

/// A feature's text is its problem, its description and its solution (ADR 0372 D6): no entity is created for the
/// solution, and nothing is ledgered for it.
#[test]
fn a_feature_solution_joins_its_requirement_text_and_creates_nothing() {
    let planned = planned(&[
        feature(
            "store",
            json!({"description": "One file per entry.", "solution": "Write each entry."}),
        ),
        feature("other", json!({"solution": "Only a solution."})),
    ]);
    assert_eq!(
        ids(&planned),
        ["FEATURE-02", "FEATURE-01"],
        "store and other, each once, numbered in old-id order (ADR 0372 D3)"
    );
    assert_eq!(planned.converted, planned.creates.len());
    assert_eq!(
        field_of(&planned, "feature", "store", "text"),
        Some(&one("P\n\nOne file per entry.\n\nWrite each entry."))
    );
    assert_eq!(
        field_of(&planned, "feature", "other", "text"),
        Some(&one("P\n\nOnly a solution."))
    );
    assert_eq!(why_of(&planned, "store", "solution"), []);
    assert!(
        !planned.ledger.counts().contains_key("created"),
        "{:?}",
        planned.ledger.counts()
    );
}

#[test]
fn feature_status_sets_the_tier_and_compares_the_computed_status() {
    let statuses = ["proposed", "accepted", "in_progress", "done", "deferred"];
    let entities: Vec<_> = statuses
        .iter()
        .map(|s| old("feature", s, json!({"problem": "P", "status": s})))
        .collect();
    let mut planned = planned(&entities);
    for (status, tier, means) in [
        ("proposed", "v2", "pending"),
        ("accepted", "v1", "pending"),
        ("in_progress", "v1", "in_progress"),
        ("done", "v1", "complete"),
        ("deferred", "v2", "pending"),
    ] {
        assert_eq!(
            field_of(&planned, "feature", status, "tier"),
            Some(&one(tier)),
            "{status}"
        );
        assert_eq!(
            field_of(&planned, "feature", status, "status"),
            None,
            "{status}"
        );
        let check = planned.checks.iter().find(|c| c.id == status).unwrap();
        assert_eq!(
            (check.new_field, check.expected.clone()),
            ("status", Some(json!(means)))
        );
    }
    planned.compare(|_, _| json!("pending"));
    assert_eq!(why_of(&planned, "accepted", "status"), []);
    assert_eq!(
        why_of(&planned, "done", "status"),
        [Why::Computed {
            field: "requirement.status".to_string(),
            new: json!("pending")
        }]
    );
    let unwritten = planned_without_status();
    assert_eq!(
        field_of(&unwritten, "feature", "f", "tier"),
        Some(&one("v2")),
        "the old default, proposed"
    );
}

fn planned_without_status() -> Planned {
    planned(&[old("feature", "f", json!({"problem": "P"}))])
}

#[test]
fn a_deprecated_feature_is_out_of_scope_with_a_reason() {
    let planned = planned(&[
        feature("bare", json!({"status": "deprecated"})),
        feature(
            "told",
            json!({"status": "deprecated", "reason": "Replaced by sync"}),
        ),
    ]);
    assert_eq!(
        field_of(&planned, "feature", "bare", "tier"),
        Some(&one("out_of_scope"))
    );
    assert_eq!(
        field_of(&planned, "feature", "bare", "reason"),
        Some(&one("deprecated"))
    );
    assert_eq!(
        why_of(&planned, "bare", "reason"),
        [Why::Filled {
            field: "requirement.reason".to_string(),
            value: json!("deprecated")
        }]
    );
    assert_eq!(
        field_of(&planned, "feature", "told", "reason"),
        Some(&one("Replaced by sync"))
    );
    assert_eq!(why_of(&planned, "told", "reason"), []);
}

/// A feature's effort, priority, tags and sources leave the requirement (ADR 0370 D3): each value is one `dropped`
/// entry, a scheme reference among the sources too.
#[test]
fn feature_effort_priority_tags_and_sources_are_dropped() {
    let planned = planned(&[feature(
        "f",
        json!({
            "effort": "s", "owner": "Alice Martin", "contributors": ["Bob Stone", "Alice Martin"],
            "refs": ["Design note", "gh.issue:42"], "tags": ["storage"], "priority": "high",
        }),
    )]);
    let fields = create_of(&planned, "feature", "f").1;
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        ["category", "text", "tier"],
        "{fields:?}"
    );
    for name in [
        "contributors",
        "effort",
        "owner",
        "priority",
        "refs",
        "tags",
    ] {
        assert!(
            matches!(why_of(&planned, "f", name).as_slice(), [Why::Dropped(reason)] if reason.contains("ADR 0370")),
            "{name}: {:?}",
            why_of(&planned, "f", name)
        );
    }
}

#[test]
fn a_feature_description_joins_the_text_after_the_problem() {
    let planned = planned(&[
        feature("both", json!({"description": "D"})),
        old(
            "feature",
            "only",
            json!({"problem": "", "description": "D"}),
        ),
    ]);
    assert_eq!(
        field_of(&planned, "feature", "both", "text"),
        Some(&one("P\n\nD"))
    );
    assert_eq!(
        field_of(&planned, "feature", "only", "text"),
        Some(&one("D"))
    );
}

/// A blank part adds nothing to a requirement's text: no blank lines after a problem whose description is empty, and no
/// trailing space after a metric whose threshold is empty (ADR 0372 D6).
#[test]
fn a_blank_part_adds_nothing_to_a_requirement_text() {
    let planned = planned(&[
        feature("empty", json!({"description": ""})),
        feature("spaces", json!({"description": "  ", "solution": "S"})),
        old(
            "constraint",
            "unbounded",
            json!({"description": "D", "metric": "throughput", "threshold": ""}),
        ),
    ]);
    for (kind, id, text) in [
        ("feature", "empty", "P"),
        ("feature", "spaces", "P\n\nS"),
        ("constraint", "unbounded", "D\n\nthroughput"),
    ] {
        assert_eq!(
            field_of(&planned, kind, id, "text"),
            Some(&one(text)),
            "{id}"
        );
    }
}

/// GSD writes acceptance as a plan's truths, which the conversion does not create (ADR 0372 D6): a feature's acceptance
/// and its tests are each one `dropped` entry.
#[test]
fn feature_acceptance_and_tests_are_dropped_with_their_reasons() {
    let planned = planned(&[feature(
        "f",
        json!({"acceptance": ["A saved entry is on disk"], "tests": ["crates/store/tests/save.rs"]}),
    )]);
    assert_eq!(
        create_of(&planned, "feature", "f")
            .1
            .keys()
            .collect::<Vec<_>>(),
        ["category", "text", "tier"]
    );
    for name in ["acceptance", "tests"] {
        assert!(
            matches!(
                why_of(&planned, "f", name).as_slice(),
                [Why::Dropped(reason)] if reason.contains("plan's truths") && reason.contains("ADR 0372 D6")
            ),
            "{name}: {:?}",
            why_of(&planned, "f", name)
        );
    }
}

/// Why `kind`'s entities are dropped whole.
fn reason(kind: &str) -> &'static str {
    match table::fate(kind) {
        Some(Fate::Dropped(reason)) => reason,
        other => panic!("{kind} is dropped whole: {other:?}"),
    }
}

/// A deliverable is GSD's coverage item of a plan's summary, and the old spec holds no plan; a module has no kind
/// (ADR 0372 D1, ADR 0365): each creates nothing, and its title and every value is one `dropped` entry with its fate's
/// reason.
#[test]
fn a_deliverable_and_a_module_are_dropped_whole() {
    let planned = planned(&[
        old(
            "deliverable",
            "store_cli",
            json!({"artifact_type": "cli", "description": "The tool", "status": "deprecated",
                   "owner": "Alice Martin", "journeys": ["record"], "modules": ["store_module"],
                   "version": "1.0", "milestones": ["first_cut"], "reason": "Shipped"}),
        ),
        old(
            "module",
            "store_module",
            json!({"family": "core", "description": "D", "features": ["store"]}),
        ),
    ]);
    assert!(planned.creates.is_empty(), "{:?}", planned.creates);
    assert!(planned.checks.is_empty());
    assert_eq!(planned.converted, 0);
    for (kind, id, fields) in [
        (
            "deliverable",
            "store_cli",
            [
                "artifact_type",
                "description",
                "journeys",
                "milestones",
                "modules",
                "owner",
                "reason",
                "status",
                "title",
                "version",
            ]
            .as_slice(),
        ),
        (
            "module",
            "store_module",
            &["description", "family", "features", "title"],
        ),
    ] {
        let dropped = Why::Dropped(reason(kind).to_string());
        assert_eq!(
            ledgered(&planned, id),
            fields
                .iter()
                .map(|f| (f.to_string(), dropped.clone()))
                .collect::<Vec<_>>(),
            "{kind}"
        );
    }
    assert!(reason("deliverable").contains("plan"));
}

/// gsd-core has no journey, persona, term or channel kind (ADR 0370 D2): each old one creates no entity, and its
/// title and each of its values is one `dropped` entry with its fate's reason.
#[test]
fn an_old_kind_that_converts_to_nothing_creates_no_entity_and_ledgers_each_value_dropped() {
    let entities = [
        old(
            "journey",
            "record",
            json!({"persona": "writer", "flow": ["Open", "Save"]}),
        ),
        old(
            "persona",
            "writer",
            json!({"description": "Writes daily", "goals": ["Keep entries"]}),
        ),
        old(
            "term",
            "t_entry",
            json!({"definition": "D", "aliases": ["note"]}),
        ),
        old(
            "channel",
            "cli",
            json!({"description": "The tool", "status": "deprecated"}),
        ),
    ];
    let planned = planned(&entities);
    assert!(planned.creates.is_empty(), "{:?}", planned.creates);
    assert_eq!(planned.converted, 0);
    assert!(planned.checks.is_empty());
    for (kind, id, fields) in [
        ("journey", "record", ["flow", "persona", "title"].as_slice()),
        ("persona", "writer", &["description", "goals", "title"]),
        ("term", "t_entry", &["aliases", "definition", "title"]),
        ("channel", "cli", &["description", "status", "title"]),
    ] {
        let reason = reason(kind);
        assert!(reason.contains("ADR 0370"), "{reason}");
        let ledger = ledgered(&planned, id);
        assert_eq!(
            ledger.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            fields,
            "{kind}: one entry for the title and one per value"
        );
        for (field, why) in ledger {
            assert_eq!(why, Why::Dropped(reason.to_string()), "{kind}.{field}");
        }
    }
}

/// An entity of a kind that converts to nothing, with no value, is still one `dropped` entry: its title, or `null`
/// when it has none (ADR 0360 D3: the ledger lists every old value not written as itself).
#[test]
fn an_entity_with_no_value_of_a_kind_that_converts_to_nothing_ledgers_its_title() {
    let mut untitled = old("persona", "p", json!({}));
    untitled.title = None;
    let planned = planned(&[old("term", "t", json!({})), untitled]);
    assert!(planned.creates.is_empty(), "{:?}", planned.creates);
    let entries: Vec<_> = planned
        .ledger
        .entries()
        .map(|e| {
            (
                e.kind.as_str(),
                e.id.as_str(),
                e.field.as_str(),
                e.value.clone(),
                e.why.clone(),
            )
        })
        .collect();
    let dropped = |kind: &str| Why::Dropped(reason(kind).to_string());
    assert_eq!(
        entries,
        [
            ("persona", "p", "title", Value::Null, dropped("persona")),
            ("term", "t", "title", json!("The t"), dropped("term")),
        ]
    );
}
