//! `--only` converts a selection, never stubbing what it leaves out (ADR 0360 D4): a reference to it is ledgered
//! `dropped` (ADR 0372 D4).

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
    assert_eq!(
        created(&planned),
        ["D-01", "D-02"],
        "a and c, in the old order, numbered over the selection (ADR 0372 D3)"
    );
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
        "--only names 2 id(s) that name no entity of the source:\n  'ghost'\n  'write_entry'"
    );
}

/// Every old kind has a fate (ADR 0372 D1), so `--only` may name an entity of any of them: one dropped whole leaves its
/// values in the ledger, a release becomes a version milestone.
#[test]
fn a_selected_entity_of_any_fated_kind_may_be_named() {
    let all = [
        old("module", "store_module", json!({"family": "core"})),
        old("behavior", "write_entry", json!({"contract": "C"})),
        old("release", "first", json!({"version": "0.1.0"})),
        decision("a", json!({})),
    ];
    let planned = selected(&all, &["a", "first", "store_module", "write_entry"]);
    assert_eq!(
        created(&planned),
        ["v0.1.0", "D-01"],
        "a release is a version milestone; a module and a behavior create nothing"
    );
    for id in ["store_module", "write_entry"] {
        assert!(
            ledgered(&planned, id)
                .iter()
                .all(|(_, why)| matches!(why, Why::Dropped(_))),
            "{id}"
        );
    }
}

#[test]
fn a_selected_failure_mode_is_kept_and_its_values_dropped() {
    let all = [
        old("failure_mode", "torn", json!({"severity": "high"})),
        decision("a", json!({})),
    ];
    let planned = selected(&all, &["a", "torn"]);
    assert_eq!(created(&planned), ["D-01"]);
    let fields: Vec<String> = ledgered(&planned, "torn")
        .into_iter()
        .map(|(field, why)| {
            assert!(matches!(why, Why::Dropped(_)), "{field}: {why:?}");
            field
        })
        .collect();
    assert_eq!(fields, ["severity", "title"]);
}

#[test]
fn a_selected_feature_creates_its_requirement_alone() {
    let all = [
        old("feature", "f", json!({"problem": "P", "solution": "S"})),
        old("feature", "g", json!({"problem": "P", "solution": "S"})),
    ];
    let planned = selected(&all, &["f"]);
    assert_eq!(
        created(&planned),
        ["FEATURE-01"],
        "its solution is in its text"
    );
    assert_eq!(planned.converted, 1);
}

#[test]
fn an_entity_of_a_kind_with_no_fate_outside_the_scope_does_not_stop_the_run() {
    let all = [
        decision("a", json!({})),
        old("widget", "gear", json!({"size": 3})),
    ];
    let entities = only(&["a"]).select(all.to_vec()).unwrap();
    assert_eq!(plan::prechecks(&entities), Ok(()));
    assert!(plan::prechecks(&all).is_err(), "the whole project stops");
}
