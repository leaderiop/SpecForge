//! Old product kinds become `@gsd/core` requirements and milestones (ADR 0359), keeping what gsd-core's kinds hold;
//! an old invariant, journey, persona, term and channel convert to nothing (ADR 0370 D8); a module waits for a kind,
//! and creates nothing. Releases and milestones are `roadmap.rs`'s.

use serde_json::{Value, json};
use specforge_ops::authoring::edit::Edit;
use std::collections::BTreeMap;

use super::super::ledger::Why;
use super::super::plan::Planned;
use super::super::table::{self, Home, Pending};
use super::{create, field, ledgered, old, one, planned, stops};

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

#[test]
fn a_feature_solution_becomes_a_project_decision_beside_the_requirement() {
    let planned = planned(&[
        feature("store", json!({"solution": "Write each entry."})),
        feature("other", json!({})),
    ]);
    assert_eq!(ids(&planned), ["store", "store_solution", "other"]);
    assert_eq!((planned.converted, planned.created), (2, 1));
    let (kind, fields) = create(&planned, "store_solution");
    assert_eq!(kind, "decision");
    assert_eq!(
        fields,
        &BTreeMap::from([
            ("text".to_string(), one("Write each entry.")),
            ("origin".to_string(), one("project")),
        ]),
        "no `affects`: the field left the decision (ADR 0370 D3)"
    );
    let Edit::Create { title, file, .. } = &planned.creates[1] else {
        panic!("a create")
    };
    assert_eq!(
        (title.as_str(), file.as_str()),
        ("The store", "spec/feature.spec")
    );
    assert_eq!(
        why_of(&planned, "store", "solution"),
        [Why::Created {
            kind: "decision".to_string(),
            id: "store_solution".to_string()
        }]
    );
    let clash = stops(&[
        feature("store", json!({"solution": "S"})),
        old(
            "decision",
            "store_solution",
            json!({"status": "accepted", "decision": "D"}),
        ),
    ]);
    assert_eq!(
        clash[0].to_string(),
        "'store_solution': two entities given one decision id, by feature 'store' (its solution) and \
         decision 'store_solution'"
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
            field(&planned, status, "tier"),
            Some(&one(tier)),
            "{status}"
        );
        assert_eq!(field(&planned, status, "status"), None, "{status}");
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
        field(&unwritten, "f", "tier"),
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
    assert_eq!(field(&planned, "bare", "tier"), Some(&one("out_of_scope")));
    assert_eq!(field(&planned, "bare", "reason"), Some(&one("deprecated")));
    assert_eq!(
        why_of(&planned, "bare", "reason"),
        [Why::Filled {
            field: "requirement.reason".to_string(),
            value: json!("deprecated")
        }]
    );
    assert_eq!(
        field(&planned, "told", "reason"),
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
    let fields = create(&planned, "f").1;
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
    assert_eq!(field(&planned, "both", "text"), Some(&one("P\n\nD")));
    assert_eq!(field(&planned, "only", "text"), Some(&one("D")));
}

#[test]
fn a_deliverable_becomes_a_milestone_keeping_its_id_goal_and_state() {
    let planned = planned(&[old(
        "deliverable",
        "store_cli",
        json!({"artifact_type": "cli", "description": "The tool", "status": "deprecated",
               "owner": "Alice Martin", "journeys": ["record"], "modules": ["store_module"]}),
    )]);
    let (kind, fields) = create(&planned, "store_cli");
    assert_eq!(kind, "milestone");
    assert_eq!(
        fields,
        &BTreeMap::from([
            ("name".to_string(), one("The store_cli")),
            ("goal".to_string(), one("The tool")),
        ]),
        "a milestone keeps gsd-core's fields alone (ADR 0370 D3)"
    );
    assert_eq!(planned.checks[0].expected, Some(json!("abandoned")));
    assert_eq!(
        why_of(&planned, "store_cli", "modules"),
        [Why::Later(Pending::Package)]
    );
    for name in ["artifact_type", "journeys", "owner"] {
        assert!(
            matches!(
                why_of(&planned, "store_cli", name).as_slice(),
                [Why::Dropped(_)]
            ),
            "{name}"
        );
    }
}

/// gsd-core has no invariant, journey, persona, term or channel kind (ADR 0370 D2): each old one creates no entity,
/// and its title and each of its values, an invariant's obligation too, is one `dropped` entry with its kind's reason,
/// the reason each of its rows gives.
#[test]
fn an_old_kind_that_converts_to_nothing_creates_no_entity_and_ledgers_each_value_dropped() {
    let mut invariant = old(
        "invariant",
        "durable",
        json!({"guarantee": "Writes survive.", "risk": "high"}),
    );
    invariant.obligations = vec![(Some("unit".to_string()), "survives a crash".to_string())];
    let entities = [
        invariant,
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
    assert_eq!((planned.converted, planned.created), (0, 0));
    assert!(planned.checks.is_empty());
    for (kind, id, fields) in [
        (
            "invariant",
            "durable",
            ["guarantee", "risk", "title", "verify"].as_slice(),
        ),
        ("journey", "record", &["flow", "persona", "title"]),
        ("persona", "writer", &["description", "goals", "title"]),
        ("term", "t_entry", &["aliases", "definition", "title"]),
        ("channel", "cli", &["description", "status", "title"]),
    ] {
        assert_eq!(table::new_kind(kind), None, "{kind}");
        let reason = table::dropped(kind).unwrap();
        assert!(reason.contains("ADR 0370"), "{reason}");
        let ledger = ledgered(&planned, id);
        assert_eq!(
            ledger.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            fields,
            "{kind}: one entry for the title and one per value"
        );
        for (field, why) in ledger {
            if let Some(row) = table::row_of(kind, &field) {
                assert_eq!(row.homes, [Home::Dropped(reason)], "{kind}.{field}");
            }
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
    let dropped = |kind: &str| Why::Dropped(table::dropped(kind).unwrap().to_string());
    assert_eq!(
        entries,
        [
            ("persona", "p", "title", Value::Null, dropped("persona")),
            ("term", "t", "title", json!("The t"), dropped("term")),
        ]
    );
}

/// The kinds whose every row drops its value, each with one reason, are exactly those that convert to nothing.
#[test]
fn the_kinds_that_convert_to_nothing_are_those_whose_every_row_drops_with_one_reason() {
    let dropped: Vec<&str> = table::OLD_KINDS
        .into_iter()
        .filter(|kind| table::dropped(kind).is_some())
        .collect();
    assert_eq!(
        dropped,
        [
            "failure_mode",
            "invariant",
            "journey",
            "term",
            "persona",
            "channel"
        ]
    );
    assert_eq!(
        table::dropped("constraint"),
        None,
        "a row writes, another drops"
    );
    assert_eq!(table::dropped("module"), None, "every row waits");
    assert_eq!(table::dropped("nothing"), None, "no row");
}

#[test]
fn a_module_creates_nothing_and_ledgers_each_value() {
    let planned = planned(&[old(
        "module",
        "store_module",
        json!({"family": "core", "description": "D", "features": ["store"]}),
    )]);
    assert!(planned.creates.is_empty());
    assert_eq!((planned.converted, planned.created), (0, 0));
    let module: Vec<_> = ledgered(&planned, "store_module");
    assert_eq!(
        module,
        ["description", "family", "features"]
            .map(|f| (f.to_string(), Why::Later(Pending::Package)))
    );
}
