//! `--only` converts a selection, never stubbing what it leaves out (ADR 0360 D4); no home writes a reference (ADR
//! 0370 D8), so what it leaves out changes nothing written.

use serde_json::json;
use specforge_ops::authoring::edit::Edit;

use super::super::ledger::Why;
use super::super::plan::{self, Planned};
use super::super::read::OldEntity;
use super::super::scope::Scope;
use super::{ledgered, old};

fn only(ids: &[&str]) -> Scope {
    Scope::Only(ids.iter().map(|id| id.to_string()).collect())
}

/// `ids` of `all` planned.
fn selected(all: &[OldEntity], ids: &[&str]) -> Planned {
    let entities = only(ids)
        .select(all.to_vec())
        .expect("the selection converts");
    plan::prechecks(&entities).expect("the pre-checks pass");
    plan::plan(&entities)
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
