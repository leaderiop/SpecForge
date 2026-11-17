//! The converter's unit tests: the plan is pure, so most build old entities by hand; the ones that read the fixture
//! compile it in-process with its builtins.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

#[path = "tests/census.rs"]
mod census;
#[path = "tests/decision.rs"]
mod decision;
#[path = "tests/fates.rs"]
mod fates;
#[path = "tests/governance.rs"]
mod governance;
#[path = "tests/ids.rs"]
mod ids;
#[path = "tests/measure.rs"]
mod measure;
#[path = "tests/product.rs"]
mod product;
#[path = "tests/roadmap.rs"]
mod roadmap;
#[path = "tests/scope.rs"]
mod scope;

use serde_json::Value;
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::BTreeMap;
use std::path::Path;

use super::ids as id_map;
use super::ledger::Why;
use super::plan::{self, Planned, Stop};
use super::read::{self, OldEntity};

/// The old fixture project: the five old builtins and an entity of every old kind.
pub(super) const OLD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb/old");

/// An old entity of `kind` named `id`, titled after it, in `spec/<kind>.spec`, writing `fields` (a JSON object), with no
/// method and no field annotation.
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
        methods: Vec::new(),
        annotations: BTreeMap::new(),
    }
}

/// `entities` planned, once their pre-checks pass.
pub(super) fn planned(entities: &[OldEntity]) -> Planned {
    plan::prechecks(entities).expect("the pre-checks pass");
    plan::plan(entities)
}

/// The pre-checks' stops for `entities`.
pub(super) fn stops(entities: &[OldEntity]) -> Vec<Stop> {
    plan::prechecks(entities).expect_err("a stop")
}

/// The path of a create: its scope field's value, `/`, its id, for a kind the target scopes ([`id_map::SCOPED`]); its id
/// otherwise.
pub(super) fn path_of(edit: &Edit) -> String {
    let Edit::Create {
        kind, id, fields, ..
    } = edit
    else {
        panic!("only creates: {edit:?}");
    };
    let scope = id_map::SCOPED
        .iter()
        .find(|(scoped, _)| scoped == kind)
        .and_then(|(_, field)| fields.get(*field));
    match scope {
        Some(Input::One(scope)) => format!("{scope}/{id}"),
        _ => id.clone(),
    }
}

/// The paths of the creates, in order.
pub(super) fn paths(planned: &Planned) -> Vec<String> {
    planned.creates.iter().map(path_of).collect()
}

/// The create whose new path is `path`: its kind and fields.
pub(super) fn create<'p>(
    planned: &'p Planned,
    path: &str,
) -> (&'p str, &'p BTreeMap<String, Input>) {
    planned
        .creates
        .iter()
        .find_map(|edit| match edit {
            Edit::Create { kind, fields, .. } if path_of(edit) == path => {
                Some((kind.as_str(), fields))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no create of {path}"))
}

/// The value the create whose new path is `path` writes in `field`.
pub(super) fn field<'p>(planned: &'p Planned, path: &str, field: &str) -> Option<&'p Input> {
    create(planned, path).1.get(field)
}

/// The new path the id map gives the old entity (`kind`, `id`), which converts.
pub(super) fn new_path(planned: &Planned, kind: &str, id: &str) -> String {
    planned
        .ids
        .path(kind, id)
        .unwrap_or_else(|| panic!("{kind} '{id}' converts to nothing"))
}

/// The create of the old entity (`kind`, `id`), found through the id map: its kind and fields.
pub(super) fn create_of<'p>(
    planned: &'p Planned,
    kind: &str,
    id: &str,
) -> (&'p str, &'p BTreeMap<String, Input>) {
    create(planned, &new_path(planned, kind, id))
}

/// The value the create of the old entity (`kind`, `id`) writes in `field`.
pub(super) fn field_of<'p>(
    planned: &'p Planned,
    kind: &str,
    id: &str,
    field: &str,
) -> Option<&'p Input> {
    create_of(planned, kind, id).1.get(field)
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

/// A symbolic link to the directory `target` (which need not exist) at `link`; it never returns without one (ADR 0375
/// D4). On Windows, without the privilege to make a symbolic link (os error 1314), it makes a directory junction: a
/// name-surrogate reparse point, which `symlink_metadata` reports as a link as it does a symbolic one.
#[cfg(unix)]
pub(super) fn link_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link)
        .unwrap_or_else(|e| panic!("a link {} -> {}: {e}", link.display(), target.display()));
}

/// A symbolic link to the directory `target` (which need not exist) at `link`; it never returns without one (ADR 0375
/// D4). Without the privilege to make a symbolic link (os error 1314), a directory junction: a name-surrogate reparse
/// point, which `symlink_metadata` reports as a link as it does a symbolic one.
#[cfg(windows)]
pub(super) fn link_dir(target: &Path, link: &Path) {
    match std::os::windows::fs::symlink_dir(target, link) {
        Ok(()) => {}
        Err(e) if e.raw_os_error() == Some(1314) => {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .status()
                .expect("cmd runs");
            assert!(
                status.success(),
                "mklink /J {} {}: {status}",
                link.display(),
                target.display()
            );
        }
        Err(e) => panic!("a link {} -> {}: {e}", link.display(), target.display()),
    }
}

/// A symbolic link to the file `target` (which need not exist) at `link`; it never returns without one (ADR 0375 D4).
#[cfg(unix)]
pub(super) fn link_file(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link)
        .unwrap_or_else(|e| panic!("a link {} -> {}: {e}", link.display(), target.display()));
}

/// A symbolic link to the file `target` (which need not exist) at `link`; it never returns without one (ADR 0375 D4):
/// a file has no junction, so a link it cannot make fails the test.
#[cfg(windows)]
pub(super) fn link_file(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).unwrap_or_else(|e| {
        panic!(
            "a file link {} -> {}: {e}; a file link needs the privilege to create symbolic links or Developer Mode",
            link.display(),
            target.display()
        )
    });
}
