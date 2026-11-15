//! `--only` converts a selection: what leaves it is ledgered, never stubbed (ADR 0360 D4).

use serde_json::json;
use specforge_ops::authoring::edit::Edit;

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
    plan::plan(&entities, &index(all))
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
