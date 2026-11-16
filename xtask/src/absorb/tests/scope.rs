//! `--only` converts a selection: what leaves it is ledgered, never stubbed (ADR 0360 D4).

use serde_json::json;
use specforge_ops::authoring::edit::Edit;
use std::collections::{BTreeMap, BTreeSet};

use super::super::ledger::Why;
use super::super::plan::{self, Planned};
use super::super::read::OldEntity;
use super::super::scope::Scope;
use super::{field, index, ledgered, many, old};

fn only(ids: &[&str]) -> Scope {
    Scope::Only(ids.iter().map(|id| id.to_string()).collect())
}

/// `ids` of `all` planned, with every entity of `all` in the index.
fn selected(all: &[OldEntity], ids: &[&str]) -> Planned {
    let entities = only(ids)
        .select(all.to_vec())
        .expect("the selection converts");
    plan::prechecks(&entities).expect("the pre-checks pass");
    let left: Vec<OldEntity> = all
        .iter()
        .filter(|e| !ids.contains(&e.id.as_str()))
        .cloned()
        .collect();
    plan::plan(&entities, &left, &index(all), &Default::default())
}

fn created(planned: &Planned) -> Vec<&str> {
    planned
        .creates
        .iter()
        .map(|edit| match edit {
            Edit::Create { id, .. } => id.as_str(),
            other => panic!("only creates: {other:?}"),
        })
        .collect()
}

fn decision(id: &str, more: serde_json::Value) -> OldEntity {
    let mut fields = json!({"status": "accepted", "decision": "D"});
    for (k, v) in more.as_object().unwrap() {
        fields[k] = v.clone();
    }
    old("decision", id, fields)
}

#[test]
fn only_converts_the_selected_ids() {
    let all = [
        decision("a", json!({})),
        decision("b", json!({})),
        decision("c", json!({})),
    ];
    let planned = selected(&all, &["c", "a"]);
    assert_eq!(created(&planned), ["a", "c"], "in the old order");
    assert_eq!(planned.converted, 2);
    assert_eq!(Scope::All.select(all.to_vec()).unwrap().len(), 3);
}

#[test]
fn a_reference_leaving_the_scope_is_ledgered_not_stubbed() {
    let all = [
        decision("a", json!({"invariants": ["kept", "left"]})),
        old("invariant", "kept", json!({"guarantee": "G"})),
        old("invariant", "left", json!({"guarantee": "G"})),
    ];
    let planned = selected(&all, &["a", "kept"]);
    assert_eq!(field(&planned, "a", "affects"), Some(&many(&["kept"])));
    assert_eq!(created(&planned), ["a", "kept"], "no stub of 'left'");
    assert_eq!(
        ledgered(&planned, "a")
            .into_iter()
            .filter(|(f, _)| f == "invariants")
            .collect::<Vec<_>>(),
        [(
            "invariants".to_string(),
            Why::OutOfSelection("left".to_string())
        )]
    );
}

/// The other direction: an inverse write from an entity the selection leaves out, onto one it converts. The value the
/// converted entity loses is ledgered on it, one row per item, naming the entity left out.
#[test]
fn an_inverse_write_from_an_unselected_entity_is_ledgered_on_the_selected_one() {
    let all = [
        decision(
            "old",
            json!({"status": "superseded", "superseded_by": "new"}),
        ),
        decision("new", json!({})),
    ];
    let planned = selected(&all, &["new"]);
    assert_eq!(created(&planned), ["new"]);
    assert_eq!(field(&planned, "new", "supersedes"), None);
    assert_eq!(
        ledgered(&planned, "new")
            .into_iter()
            .filter(|(f, _)| f == "supersedes")
            .collect::<Vec<_>>(),
        [(
            "supersedes".to_string(),
            Why::OutOfSelection("old".to_string())
        )]
    );
}

#[test]
fn an_inverse_write_onto_an_unselected_entity_is_ledgered() {
    let all = [
        decision(
            "old",
            json!({"status": "superseded", "superseded_by": "new"}),
        ),
        decision("new", json!({})),
    ];
    let planned = selected(&all, &["old"]);
    assert_eq!(created(&planned), ["old"]);
    assert!(
        ledgered(&planned, "old").contains(&(
            "superseded_by".to_string(),
            Why::OutOfSelection("new".to_string())
        )),
        "{:?}",
        ledgered(&planned, "old")
    );
}

#[test]
fn an_unknown_selected_id_is_refused() {
    let all = [decision("a", json!({}))];
    let why = only(&["a", "ghost", "write_entry"])
        .select(all.to_vec())
        .unwrap_err();
    assert_eq!(
        why,
        "--only names 2 id(s) that do not convert:\n  'ghost' names no entity of the absorbed kinds\n  \
         'write_entry' names no entity of the absorbed kinds"
    );
}

#[test]
fn a_selected_module_or_release_id_is_refused() {
    let all = [
        old("module", "store_module", json!({"family": "core"})),
        old("release", "first", json!({"version": "0.1.0"})),
        decision("a", json!({})),
    ];
    let why = only(&["a", "first", "store_module"])
        .select(all.to_vec())
        .unwrap_err();
    assert!(
        why.contains("'first' is a release, which creates no entity"),
        "{why}"
    );
    assert!(
        why.contains("'store_module' is a module, which creates no entity"),
        "{why}"
    );
}

#[test]
fn a_selected_failure_mode_is_kept_and_its_values_dropped() {
    let all = [
        old("failure_mode", "torn", json!({"severity": "high"})),
        decision("a", json!({})),
    ];
    let planned = selected(&all, &["a", "torn"]);
    assert_eq!(created(&planned), ["a"]);
    assert!(
        matches!(
            ledgered(&planned, "torn").as_slice(),
            [(field, Why::Dropped(_))] if field == "severity"
        ),
        "{:?}",
        ledgered(&planned, "torn")
    );
}

#[test]
fn a_selected_feature_brings_its_solution_decision() {
    let all = [
        old("feature", "f", json!({"problem": "P", "solution": "S"})),
        old("feature", "g", json!({"problem": "P", "solution": "S"})),
    ];
    let planned = selected(&all, &["f"]);
    assert_eq!(created(&planned), ["f", "f_solution"]);
    assert_eq!((planned.converted, planned.created), (1, 1));
}

#[test]
fn a_refused_value_outside_the_scope_does_not_stop_the_run() {
    let all = [
        decision("a", json!({})),
        decision("refusing", json!({"description": "no kind holds this"})),
    ];
    let entities = only(&["a"]).select(all.to_vec()).unwrap();
    assert_eq!(plan::prechecks(&entities), Ok(()));
    assert!(plan::prechecks(&all).is_err(), "the whole project stops");
}

/// A reference the selection leaves out is recorded where it was to be written (the new entity and field), so a
/// required one is a stop that names the selection: a selected entity's own reference, and an inverse write from an
/// entity left out onto it. An inverse write onto an entity left out records nothing there.
#[test]
fn a_reference_the_selection_leaves_out_is_recorded_where_it_was_to_be_written() {
    let all = [
        old(
            "journey",
            "record",
            json!({"persona": "writer", "flow": ["Writer saves"]}),
        ),
        old("persona", "writer", json!({"description": "D"})),
        old("deliverable", "store_cli", json!({"journeys": ["record"]})),
    ];
    assert_eq!(
        selected(&all, &["record", "store_cli"]).left_out,
        BTreeMap::from([(
            ("record".to_string(), "persona".to_string()),
            BTreeSet::from(["writer".to_string()])
        )])
    );
    assert_eq!(
        selected(&all, &["store_cli"]).left_out,
        BTreeMap::new(),
        "a deliverable's journeys are written on the journey"
    );
    let planned = selected(&all, &["record", "writer"]);
    assert_eq!(
        planned.left_out,
        BTreeMap::from([(
            ("record".to_string(), "milestones".to_string()),
            BTreeSet::from(["store_cli".to_string()])
        )]),
        "a deliverable left out takes its milestone off the journey"
    );
    assert!(ledgered(&planned, "record").contains(&(
        "milestones".to_string(),
        Why::OutOfSelection("store_cli".to_string())
    )));
}
