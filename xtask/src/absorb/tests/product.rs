//! Old product kinds become `@gsd/core` requirements, journeys, personas, terms and milestones (ADR 0357, ADR 0359);
//! a module or a release waits for a kind, and creates nothing.

use serde_json::json;
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::BTreeMap;

use super::super::ledger::Why;
use super::super::plan::Planned;
use super::super::table::Pending;
use super::{create, field, ledgered, many, old, one, planned, stops};

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
fn a_feature_solution_becomes_a_decision_affecting_the_requirement() {
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
            ("affects".to_string(), many(&["store"])),
        ])
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
        "'store_solution': two creates with one id, by feature 'store' (its solution) and decision \
         'store_solution'"
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

#[test]
fn feature_effort_and_sources_are_kept() {
    let planned = planned(&[feature(
        "f",
        json!({
            "effort": "s", "owner": "Alice Martin", "contributors": ["Bob Stone", "Alice Martin"],
            "refs": ["Design note", "gh.issue:42"], "tags": ["storage"], "priority": "high",
        }),
    )]);
    assert_eq!(field(&planned, "f", "effort"), Some(&one("s")));
    assert_eq!(field(&planned, "f", "tags"), Some(&many(&["storage"])));
    assert_eq!(field(&planned, "f", "priority"), Some(&one("should")));
    assert_eq!(
        field(&planned, "f", "source"),
        Some(&many(&["Alice Martin", "Bob Stone", "Design note"])),
        "owner, contributors, then refs, each once"
    );
    assert_eq!(
        why_of(&planned, "f", "refs"),
        [Why::Unconverted(super::super::how::SCHEME_REF.to_string())]
    );
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

fn journey(id: &str, more: serde_json::Value) -> super::super::read::OldEntity {
    let mut fields = json!({"flow": ["Open", "Save", "See"]});
    for (k, v) in more.as_object().unwrap() {
        fields[k] = v.clone();
    }
    old("journey", id, fields)
}

#[test]
fn a_journey_flow_becomes_its_steps_in_order() {
    let planned = planned(&[journey("j", json!({}))]);
    let step = |action: &str| BTreeMap::from([("action".to_string(), one(action))]);
    assert_eq!(
        field(&planned, "j", "steps"),
        Some(&Input::Blocks(vec![
            step("Open"),
            step("Save"),
            step("See")
        ]))
    );
}

#[test]
fn journey_channels_are_written_as_words() {
    let planned = planned(&[
        journey(
            "j",
            json!({"channels": ["cli", "web"], "priority": "critical"}),
        ),
        old("channel", "cli", json!({"description": "D"})),
    ]);
    assert_eq!(
        field(&planned, "j", "channels"),
        Some(&many(&["cli", "web"]))
    );
    assert_eq!(field(&planned, "j", "priority"), Some(&one("must")));
}

#[test]
fn journey_features_become_its_requirements() {
    let planned = planned(&[
        journey("j", json!({"features": ["store"], "persona": "writer"})),
        feature("store", json!({})),
        old("persona", "writer", json!({"description": "D"})),
    ]);
    assert_eq!(
        field(&planned, "j", "requirements"),
        Some(&many(&["store"]))
    );
    assert_eq!(field(&planned, "j", "persona"), Some(&one("writer")));
}

#[test]
fn a_journey_lists_every_deliverable_that_listed_it() {
    let planned = planned(&[
        journey("shared", json!({})),
        journey("one", json!({})),
        old(
            "deliverable",
            "web",
            json!({"artifact_type": "web_app", "journeys": ["shared"]}),
        ),
        old(
            "deliverable",
            "cli",
            json!({"artifact_type": "cli", "journeys": ["shared", "one"]}),
        ),
    ]);
    assert_eq!(
        field(&planned, "shared", "milestones"),
        Some(&many(&["cli", "web"]))
    );
    assert_eq!(field(&planned, "one", "milestones"), Some(&many(&["cli"])));
    assert_eq!(field(&planned, "cli", "journeys"), None);
}

#[test]
fn a_deliverable_becomes_a_milestone_keeping_its_id_and_its_artifact_type_as_a_list() {
    let planned = planned(&[old(
        "deliverable",
        "store_cli",
        json!({"artifact_type": "cli", "description": "The tool", "status": "deprecated",
               "owner": "Alice Martin", "modules": ["store_module"]}),
    )]);
    let (kind, _) = create(&planned, "store_cli");
    assert_eq!(kind, "milestone");
    assert_eq!(
        field(&planned, "store_cli", "name"),
        Some(&one("The store_cli"))
    );
    assert_eq!(
        field(&planned, "store_cli", "artifact_types"),
        Some(&many(&["cli"]))
    );
    assert_eq!(field(&planned, "store_cli", "goal"), Some(&one("The tool")));
    assert_eq!(
        field(&planned, "store_cli", "owner"),
        Some(&one("Alice Martin"))
    );
    assert_eq!(planned.checks[0].expected, Some(json!("abandoned")));
    assert_eq!(
        why_of(&planned, "store_cli", "modules"),
        [Why::Later(Pending::Package)]
    );
}

#[test]
fn a_milestone_keeps_its_exit_criteria_dates_owner_and_dependencies() {
    let planned = planned(&[
        old("milestone", "groundwork", json!({"status": "blocked"})),
        old(
            "milestone",
            "first_cut",
            json!({
                "status": "completed", "start_date": "2026-09-01", "target_date": "2026-10-15",
                "owner": "Alice Martin", "contributors": ["Bob Stone"], "depends_on": ["groundwork"],
                "exit_criteria": ["Survives a crash"], "features": ["store"], "behaviors": ["write"],
            }),
        ),
    ]);
    for (name, value) in [
        ("starts", one("2026-09-01")),
        ("targets", one("2026-10-15")),
        ("owner", one("Alice Martin")),
        ("contributors", many(&["Bob Stone"])),
        ("depends_on", many(&["groundwork"])),
        ("exit_criteria", many(&["Survives a crash"])),
        ("target_features", many(&["store"])),
        ("name", one("The first_cut")),
    ] {
        assert_eq!(field(&planned, "first_cut", name), Some(&value), "{name}");
    }
    let expected: Vec<_> = planned
        .checks
        .iter()
        .map(|c| (c.id.as_str(), c.expected.clone()))
        .collect();
    assert_eq!(
        expected,
        [("groundwork", None), ("first_cut", Some(json!("shipped")))],
        "blocked has no equal state"
    );
    assert_eq!(
        why_of(&planned, "first_cut", "behaviors"),
        [Why::Later(Pending::Behavior)]
    );
}

#[test]
fn persona_status_and_key_features_are_kept() {
    let planned = planned(&[
        old(
            "persona",
            "writer",
            json!({"description": "Writes daily", "status": "deprecated", "technical_level": "beginner",
                   "goals": ["Keep entries"], "key_features": ["store"]}),
        ),
        feature("store", json!({})),
    ]);
    assert_eq!(
        field(&planned, "writer", "status"),
        Some(&one("deprecated"))
    );
    assert_eq!(
        field(&planned, "writer", "context"),
        Some(&one("Writes daily"))
    );
    assert_eq!(
        field(&planned, "writer", "technical_level"),
        Some(&one("beginner"))
    );
    assert_eq!(
        field(&planned, "writer", "goals"),
        Some(&many(&["Keep entries"]))
    );
    assert_eq!(
        field(&planned, "store", "personas"),
        Some(&many(&["writer"]))
    );
}

#[test]
fn a_term_keeps_its_aliases_and_writes_see_also_as_related() {
    let planned = planned(&[
        old(
            "term",
            "t_entry",
            json!({"definition": "D", "aliases": ["note"], "see_also": ["t_store", "t_gone"]}),
        ),
        old("term", "t_store", json!({"definition": "D"})),
    ]);
    assert_eq!(
        field(&planned, "t_entry", "aliases"),
        Some(&many(&["note"]))
    );
    assert_eq!(
        field(&planned, "t_entry", "related"),
        Some(&many(&["t_store"]))
    );
    assert_eq!(
        field(&planned, "t_entry", "name"),
        Some(&one("The t_entry"))
    );
    assert!(matches!(
        why_of(&planned, "t_entry", "see_also").as_slice(),
        [Why::Unconverted(_)]
    ));
}

#[test]
fn a_channel_becomes_a_term_of_its_id() {
    let planned = planned(&[old(
        "channel",
        "cli",
        json!({"description": "The tool", "interaction_model": "request_response", "tags": ["primary"],
               "status": "active"}),
    )]);
    let (kind, fields) = create(&planned, "cli");
    assert_eq!(kind, "term");
    assert_eq!(
        fields,
        &BTreeMap::from([
            ("name".to_string(), one("The cli")),
            ("definition".to_string(), one("The tool")),
            ("tags".to_string(), many(&["request_response", "primary"])),
        ])
    );
}

#[test]
fn an_active_channel_is_implied_and_a_deprecated_one_stops_the_run() {
    let active = planned(&[old(
        "channel",
        "cli",
        json!({"description": "D", "status": "active"}),
    )]);
    assert_eq!(field(&active, "cli", "status"), None);
    assert_eq!(why_of(&active, "cli", "status"), []);
    let stopped = stops(&[old(
        "channel",
        "fax",
        json!({"description": "D", "status": "deprecated"}),
    )]);
    assert_eq!(
        stopped.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["channel 'fax' status: no home for \"deprecated\": a term implies only \"active\""]
    );
}

#[test]
fn a_module_or_release_creates_nothing_and_ledgers_each_value() {
    let planned = planned(&[
        old(
            "module",
            "store_module",
            json!({"family": "core", "description": "D", "features": ["store"]}),
        ),
        old(
            "release",
            "first",
            json!({"version": "0.1.0", "status": "planned", "milestones": ["cut"]}),
        ),
    ]);
    assert!(planned.creates.is_empty());
    assert_eq!((planned.converted, planned.created), (0, 0));
    let module: Vec<_> = ledgered(&planned, "store_module");
    assert_eq!(
        module,
        ["description", "family", "features"]
            .map(|f| (f.to_string(), Why::Later(Pending::Package)))
    );
    let release: Vec<_> = ledgered(&planned, "first");
    assert_eq!(
        release,
        ["milestones", "status", "version"].map(|f| (f.to_string(), Why::Later(Pending::Release)))
    );
}
