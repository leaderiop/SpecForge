//! Every old kind has one fate (ADR 0372 D1): six convert, eleven are dropped whole with their reason. absorb reads
//! every authored entity of the source, so an entity of a kind with no fate stops the run, and an entity dropped
//! whole leaves its title, fields, `verify` lines, methods and field annotations in the ledger (ADR 0372 D2).

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

use super::super::ledger::Why;
use super::super::read;
use super::super::table::{self, FATES, Fate, TABLE};
use super::{fixture, old, planned, stops};

/// The fate of `kind`, which the table holds.
fn fate(kind: &str) -> Fate {
    table::fate(kind).unwrap_or_else(|| panic!("{kind} has a fate"))
}

/// Why `kind`'s entities are dropped whole.
fn reason(kind: &str) -> &'static str {
    match fate(kind) {
        Fate::Dropped(reason) => reason,
        Fate::To(new) => panic!("{kind} converts to {new}"),
    }
}

#[test]
fn every_old_kind_has_one_fate() {
    let kinds: BTreeSet<&str> = FATES.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(
        (FATES.len(), kinds.len()),
        (17, 17),
        "one entry per old kind"
    );
    let converting: Vec<(&str, &str)> = FATES
        .iter()
        .filter_map(|(kind, fate)| fate.new_kind().map(|new| (*kind, new)))
        .collect();
    assert_eq!(
        converting,
        [
            ("decision", "decision"),
            ("constraint", "requirement"),
            ("invariant", "requirement"),
            ("feature", "requirement"),
            ("milestone", "phase"),
            ("release", "milestone"),
        ]
    );
    for (kind, fate) in FATES {
        let rows = table::rows_of(kind).count();
        match fate {
            Fate::Dropped(reason) => {
                assert!(!reason.trim().is_empty(), "{kind}: a reason");
                assert_eq!(
                    rows, 0,
                    "{kind} is dropped whole: its fate's reason is every value's"
                );
            }
            Fate::To(_) => assert!(rows > 0, "{kind} converts through its rows"),
        }
    }
    assert!(
        TABLE
            .iter()
            .all(|row| table::fate(row.kind).and_then(Fate::new_kind).is_some()),
        "a row is a converting kind's"
    );
    assert_eq!(table::fate("widget"), None);
    assert!(reason("deliverable").contains("ADR 0372 D1"));
    assert!(reason("module").contains("ADR 0365"));
    for kind in ["behavior", "event", "type", "port"] {
        assert!(reason(kind).contains("ADR 0364"), "{kind}");
    }
}

#[test]
fn an_entity_of_a_kind_with_no_fate_stops_the_run_naming_it() {
    let stops = stops(&[
        old("widget", "gear", json!({"size": 3})),
        old(
            "decision",
            "a",
            json!({"status": "accepted", "decision": "A"}),
        ),
    ]);
    let named: Vec<String> = stops.iter().map(ToString::to_string).collect();
    assert_eq!(named.len(), 1, "{named:?}");
    assert!(
        named[0].starts_with("widget 'gear': no fate for kind widget"),
        "{}",
        named[0]
    );
}

/// The entries the ledger holds for the old entity `id`: (field, value, why).
fn entries(planned: &super::super::plan::Planned, id: &str) -> Vec<(String, Value, Why)> {
    planned
        .ledger
        .entries()
        .filter(|e| e.id == id)
        .map(|e| (e.field.clone(), e.value.clone(), e.why.clone()))
        .collect()
}

#[test]
fn a_kind_dropped_whole_ledgers_its_title_fields_verify_lines_methods_and_annotations() {
    let mut port = old(
        "port",
        "Store",
        json!({"direction": "outbound", "category": "storage"}),
    );
    port.methods = vec![
        json!({"name": "save", "params": [{"name": "path", "type": "string"}], "returns": "Result<Entry, string>"}),
    ];
    port.obligations = vec![(Some("integration".to_string()), "saves".to_string())];
    let mut ty = old("type", "Entry", json!({"path": "string", "note": "string"}));
    ty.annotations = BTreeMap::from([("note".to_string(), json!([{"name": "optional"}]))]);
    let planned = planned(&[port, ty]);
    assert!(planned.creates.is_empty(), "{:?}", planned.creates);
    assert_eq!(planned.converted, 0);
    let port = Why::Dropped(reason("port").to_string());
    assert_eq!(
        entries(&planned, "Store"),
        [
            ("category".to_string(), json!("storage"), port.clone()),
            ("direction".to_string(), json!("outbound"), port.clone()),
            (
                "method".to_string(),
                json!({"name": "save", "params": [{"name": "path", "type": "string"}], "returns": "Result<Entry, string>"}),
                port.clone()
            ),
            ("title".to_string(), json!("The Store"), port.clone()),
            (
                "verify".to_string(),
                json!({"kind": "integration", "text": "saves"}),
                port
            ),
        ]
    );
    let ty = Why::Dropped(reason("type").to_string());
    assert_eq!(
        entries(&planned, "Entry"),
        [
            (
                "annotations".to_string(),
                json!({"note": [{"name": "optional"}]}),
                ty.clone()
            ),
            ("note".to_string(), json!("string"), ty.clone()),
            ("path".to_string(), json!("string"), ty.clone()),
            ("title".to_string(), json!("The Entry"), ty),
        ]
    );
}

#[test]
fn a_converting_kind_holding_a_method_or_an_annotation_stops_the_run() {
    let mut method = old("feature", "f", json!({"problem": "P"}));
    method.methods = vec![json!({"name": "run", "params": []})];
    let mut annotated = old("feature", "g", json!({"problem": "P"}));
    annotated.annotations =
        BTreeMap::from([("problem".to_string(), json!([{"name": "readonly"}]))]);
    let named: Vec<String> = stops(&[method, annotated])
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(named.len(), 2, "{named:?}");
    assert!(named[0].starts_with("feature 'f' method: "), "{}", named[0]);
    assert!(
        named[1].starts_with("feature 'g' problem: "),
        "{}",
        named[1]
    );
}

#[test]
fn read_takes_every_authored_entity_and_no_collected_fact() {
    for (kind, file) in [
        ("feature", "spec/product/features.spec"),
        ("behavior", "spec/behaviors/b.spec"),
        ("widget", "spec/w.spec"),
    ] {
        assert!(read::authored(kind, file), "{kind} in {file}");
    }
    for (kind, file) in [
        ("spec", "spec/project.spec"),
        ("ref", "spec/product/refs.spec"),
        ("feature", ".specforge/facts/main/cargo.json"),
        ("test", ".specforge/facts/main/rust.json"),
    ] {
        assert!(!read::authored(kind, file), "{kind} in {file}");
    }
}

#[test]
fn the_fixture_holds_every_old_kind() {
    let kinds: BTreeSet<String> = fixture().entities().into_iter().map(|e| e.kind).collect();
    let fated: BTreeSet<String> = FATES.iter().map(|(kind, _)| kind.to_string()).collect();
    assert_eq!(kinds, fated);
}
