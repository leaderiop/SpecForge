use std::path::Path;

use super::*;
use crate::asking::testing::{Asked, Scripted};
use serde_json::json;
use specforge_effects::testing::MemoryConsent;
use specforge_effects::{EffectsCeiling, Provenance, Roots};
use specforge_protocol_types::{ExtensionDeclaration, Operation};

fn declaration() -> ExtensionDeclaration {
    let mut declaration = ExtensionDeclaration::default();
    declaration.handshake.name = "@acme/x".to_string();
    declaration.capabilities = serde_json::from_value(json!([
        {"name": "tests", "why": "run the tests", "call_kinds": ["collect"], "port": "process",
         "argv": ["cargo", "test"]},
        {"name": "links", "why": "check links", "port": "http", "hosts": {"list": ["crates.io"]}},
        {"name": "vendored", "why": "read vendored docs", "port": "fs", "roots": ["vendor"], "ops": ["read"]},
    ]))
    .unwrap();
    declaration
}

/// A project and a `vendor` root beside it, outside it.
struct Place {
    _dir: tempfile::TempDir,
    project: std::path::PathBuf,
    vendor: std::path::PathBuf,
    elsewhere: std::path::PathBuf,
}

fn place() -> Place {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("p");
    let vendor = dir.path().join("vendor");
    let elsewhere = dir.path().join("elsewhere");
    for d in [&project, &vendor, &elsewhere] {
        std::fs::create_dir_all(d).unwrap();
    }
    Place {
        _dir: dir,
        project,
        vendor,
        elsewhere,
    }
}

fn grant(project: &Path, vendor: &Path, consent: &dyn Consent) -> Grant {
    let roots = Roots::new(project, project).with_named("vendor", vendor);
    Grant::decide(
        &declaration(),
        &Provenance::third_party(b"blob"),
        &EffectsCeiling::default(),
        &roots,
        consent,
    )
}

fn uses(pairs: &[(Operation, &str)]) -> Uses {
    pairs
        .iter()
        .map(|(op, name)| (*op, name.to_string()))
        .collect()
}

fn run_tests() -> specforge_effects::EffectRequest {
    specforge_effects::EffectRequest::Process(specforge_effects::ProcessRequest {
        argv: vec!["cargo".into(), "test".into()],
        ..Default::default()
    })
}

#[test]
fn yes_approves_for_this_run_and_stores_nothing() {
    let place = place();
    let consent = MemoryConsent::default();
    let mut grant = grant(&place.project, &place.vendor, &consent);
    let settled = settle(
        &mut grant,
        &uses(&[(Operation::Collect, "tests"), (Operation::Pass, "vendored")]),
        Asking::Yes,
        &consent,
    )
    .unwrap();
    assert_eq!(
        settled.for_this_run.len(),
        2,
        "the command and the outside root"
    );
    assert!(settled.stored.is_empty());
    assert_eq!(
        consent.list().unwrap(),
        Default::default(),
        "nothing is stored"
    );
    assert_eq!(grant.permits(Operation::Collect, &run_tests()), Ok(()));
}

#[test]
fn an_answer_is_stored_once_per_scope() {
    let place = place();
    let consent = MemoryConsent::default();
    let asker = Scripted::answering(true);
    let used = uses(&[(Operation::Collect, "links"), (Operation::Command, "links")]);
    let mut first = grant(&place.project, &place.vendor, &consent);
    let settled = settle(&mut first, &used, Asking::Ask(&asker), &consent).unwrap();
    assert_eq!(settled.stored.len(), 1);
    assert_eq!(
        asker.asked(),
        [Asked::Capability {
            extension: "@acme/x".into(),
            scope: "links".into(),
        }]
    );
    let stored = consent.list().unwrap();
    assert_eq!(stored.capabilities.len(), 1);
    assert!(
        !stored.capabilities[0].approved_at.0.is_empty(),
        "the approval is dated"
    );
    // The next run holds it: nothing is asked.
    let mut again = grant(&place.project, &place.vendor, &consent);
    let settled = settle(&mut again, &used, Asking::Ask(&asker), &consent).unwrap();
    assert!(settled.stored.is_empty() && settled.for_this_run.is_empty());
    assert_eq!(asker.asked().len(), 1);
}

#[test]
fn nobody_uses_stored_approvals_only() {
    let place = place();
    let consent = MemoryConsent::default();
    let used = uses(&[(Operation::Collect, "tests")]);
    let mut grant_now = grant(&place.project, &place.vendor, &consent);
    let unsettled = settle(&mut grant_now, &used, Asking::Nobody, &consent).unwrap_err();
    assert_eq!(unsettled.needs.len(), 1);
    assert!(unsettled.store.is_none());
    assert!(grant_now.permits(Operation::Collect, &run_tests()).is_err());

    let held = grant(&place.project, &place.vendor, &consent);
    for granted in held.capabilities() {
        if let Some(key) = held.key(granted) {
            consent.approve(Approval::Capability(key)).unwrap();
        }
    }
    let mut grant_later = grant(&place.project, &place.vendor, &consent);
    assert!(settle(&mut grant_later, &used, Asking::Nobody, &consent).is_ok());
    assert_eq!(
        grant_later.permits(Operation::Collect, &run_tests()),
        Ok(())
    );
}

#[test]
fn nobody_refuses_an_unapproved_outside_root() {
    let place = place();
    let consent = MemoryConsent::default();
    let mut grant = grant(&place.project, &place.vendor, &consent);
    let unsettled = settle(
        &mut grant,
        &uses(&[(Operation::Pass, "vendored")]),
        Asking::Nobody,
        &consent,
    )
    .unwrap_err();
    assert!(
        matches!(&unsettled.needs[..], [Need::Root(root)] if root.root == "vendor"),
        "{unsettled:?}"
    );
}

#[test]
fn a_root_re_pointed_elsewhere_asks_again() {
    let place = place();
    let consent = MemoryConsent::default();
    let asker = Scripted::answering(true);
    let used = uses(&[(Operation::Pass, "vendored")]);
    let mut first = grant(&place.project, &place.vendor, &consent);
    settle(&mut first, &used, Asking::Ask(&asker), &consent).unwrap();
    let mut same = grant(&place.project, &place.vendor, &consent);
    settle(&mut same, &used, Asking::Ask(&asker), &consent).unwrap();
    assert_eq!(asker.asked().len(), 1, "the same target is approved");
    let mut moved = grant(&place.project, &place.elsewhere, &consent);
    settle(&mut moved, &used, Asking::Ask(&asker), &consent).unwrap();
    let asked = asker.asked();
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(
        matches!(&asked[1], Asked::Root { name, target } if name == "vendor" && target.contains("elsewhere")),
        "{asked:?}"
    );
}

#[test]
fn an_unusable_store_is_w186_and_asks_nothing_it_would_lose() {
    let place = place();
    let consent = MemoryConsent::unusable("not JSON");
    let asker = Scripted::answering(true);
    let mut grant = grant(&place.project, &place.vendor, &consent);
    let unsettled = settle(
        &mut grant,
        &uses(&[(Operation::Collect, "tests")]),
        Asking::Ask(&asker),
        &consent,
    )
    .unwrap_err();
    assert_eq!(
        unsettled.store.as_ref().map(|d| d.code.as_str()),
        Some("W186")
    );
    assert!(asker.asked().is_empty());
    // A run that says yes for itself still goes on.
    assert!(
        settle(
            &mut grant,
            &uses(&[(Operation::Collect, "tests")]),
            Asking::Yes,
            &consent
        )
        .is_ok()
    );
}
