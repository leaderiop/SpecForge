//! The converter's unit tests: the plan is pure, so most build old entities by hand; the ones that read the fixture
//! compile it in-process with its builtins.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

#[path = "tests/decision.rs"]
mod decision;
#[path = "tests/governance.rs"]
mod governance;
#[path = "tests/measure.rs"]
mod measure;
#[path = "tests/product.rs"]
mod product;
#[path = "tests/proof.rs"]
mod proof;
#[path = "tests/scope.rs"]
mod scope;

use serde_json::Value;
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::BTreeMap;

use super::ledger::Why;
use super::plan::{self, Planned, Stop};
use super::proof::Proofs;
use super::read::{self, Index, OldEntity};

/// The old fixture project: the five old builtins and one file per old kind.
pub(super) const OLD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb/old");

/// An old entity of `kind` named `id`, titled after it, in `spec/<kind>.spec`, writing `fields` (a JSON object).
pub(super) fn old(kind: &str, id: &str, fields: Value) -> OldEntity {
    OldEntity {
        kind: kind.to_string(),
        id: id.to_string(),
        title: Some(format!("The {id}")),
        file: format!("spec/{kind}.spec"),
        line: 1,
        fields: fields
            .as_object()
            .expect("an object of fields")
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        obligations: Vec::new(),
    }
}

/// Every entity's kind by id, as the old project would name them.
pub(super) fn index(entities: &[OldEntity]) -> Index {
    Index(
        entities
            .iter()
            .map(|e| (e.id.clone(), e.kind.clone()))
            .collect(),
    )
}

/// `entities` planned, once their pre-checks pass.
pub(super) fn planned(entities: &[OldEntity]) -> Planned {
    plan::prechecks(entities).expect("the pre-checks pass");
    plan::plan(entities, &[], &index(entities), &Proofs::default())
}

/// The pre-checks' stops for `entities`.
pub(super) fn stops(entities: &[OldEntity]) -> Vec<Stop> {
    plan::prechecks(entities).expect_err("a stop")
}

/// The create of `id`: its kind and fields.
pub(super) fn create<'p>(planned: &'p Planned, id: &str) -> (&'p str, &'p BTreeMap<String, Input>) {
    planned
        .creates
        .iter()
        .find_map(|edit| match edit {
            Edit::Create {
                kind,
                id: created,
                fields,
                ..
            } if created == id => Some((kind.as_str(), fields)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no create of {id}"))
}

/// The value the create of `id` writes in `field`.
pub(super) fn field<'p>(planned: &'p Planned, id: &str, field: &str) -> Option<&'p Input> {
    create(planned, id).1.get(field)
}

/// The ledger's (field, why) of the old entity `id`.
pub(super) fn ledgered(planned: &Planned, id: &str) -> Vec<(String, Why)> {
    planned
        .ledger
        .entries()
        .filter(|e| e.id == id)
        .map(|e| (e.field.clone(), e.why.clone()))
        .collect()
}

pub(super) fn one(text: &str) -> Input {
    Input::One(text.to_string())
}

pub(super) fn many(items: &[&str]) -> Input {
    Input::Many(items.iter().map(|s| s.to_string()).collect())
}

/// The old fixture, compiled.
pub(super) fn fixture() -> read::Old {
    read::compile(std::path::Path::new(OLD)).expect("the fixture compiles clean")
}
