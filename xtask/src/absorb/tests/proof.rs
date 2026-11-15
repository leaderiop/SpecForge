//! An invariant's obligations become proofs of the tests whose spec-link attributes state them (ADR 0360 D6).

use serde_json::json;
use std::collections::BTreeMap;
use xtask::links::Link;

use super::super::ledger::Why;
use super::super::plan::{self, Planned};
use super::super::proof::{Item, Proofs, Proved};
use super::super::read::{self, OldEntity};
use super::{field, index, ledgered, many, old};

const DURABLE: &str = "writes_are_durable";

fn link(line: usize, id_line: usize, verify: Option<&str>) -> Link {
    Link {
        file: "src/lib.rs".to_string(),
        line,
        id_line,
        id: DURABLE.to_string(),
        verify: verify.map(String::from),
    }
}

/// A test item whose attributes name the invariant at `src/lib.rs:line`.
fn item(test_id: &str, line: usize) -> Item {
    Item {
        test_id: test_id.to_string(),
        names: BTreeMap::from([(DURABLE.to_string(), vec![("src/lib.rs".to_string(), line)])]),
    }
}

/// The invariant with obligations `texts`.
fn invariant(texts: &[&str]) -> OldEntity {
    let mut entity = old("invariant", DURABLE, json!({"guarantee": "G"}));
    entity.obligations = texts
        .iter()
        .map(|t| (Some("unit".to_string()), t.to_string()))
        .collect();
    entity
}

fn planned_with(entities: &[OldEntity], proofs: &Proofs) -> Planned {
    plan::prechecks(entities).expect("the pre-checks pass");
    plan::plan(entities, &index(entities), proofs)
}

/// The ledger's whys for the invariant's obligations.
fn unproven(planned: &Planned) -> Vec<Why> {
    ledgered(planned, DURABLE)
        .into_iter()
        .filter(|(f, _)| f == "verify")
        .map(|(_, why)| why)
        .collect()
}

#[test]
fn an_obligation_a_test_item_states_becomes_its_test_id() {
    let proofs = Proofs {
        links: vec![link(4, 5, Some("survives a crash"))],
        items: vec![item("rust:store::tests::survives", 5)],
    };
    assert_eq!(
        proofs.of(DURABLE, "survives a crash"),
        Proved::Tests(vec!["rust:store::tests::survives".to_string()])
    );
    let planned = planned_with(&[invariant(&["survives a crash"])], &proofs);
    assert_eq!(
        field(&planned, DURABLE, "proof"),
        Some(&many(&["rust:store::tests::survives"]))
    );
    assert_eq!(
        unproven(&planned),
        [Why::Collapsed {
            field: "invariant.proof".to_string(),
            into: json!(["rust:store::tests::survives"]),
        }],
        "the obligation's kind and text are ledgered, written as the test that proves it"
    );
}

/// Two obligations one test proves: the proof names the test once, and the ledger keeps each obligation's kind and
/// text beside it, so the target's one proof still says which statements it stands for.
#[test]
fn two_obligations_one_test_proves_are_each_ledgered() {
    let proofs = Proofs {
        links: vec![
            link(4, 5, Some("survives a crash")),
            link(4, 5, Some("never reads half an entry")),
        ],
        items: vec![item("rust:store::tests::survives", 5)],
    };
    let planned = planned_with(
        &[invariant(&[
            "survives a crash",
            "never reads half an entry",
        ])],
        &proofs,
    );
    assert_eq!(
        field(&planned, DURABLE, "proof"),
        Some(&many(&["rust:store::tests::survives"]))
    );
    let values: Vec<serde_json::Value> = planned
        .ledger
        .entries()
        .filter(|e| e.id == DURABLE && e.field == "verify")
        .map(|e| e.value.clone())
        .collect();
    assert_eq!(
        values,
        [
            json!({"kind": "unit", "text": "never reads half an entry"}),
            json!({"kind": "unit", "text": "survives a crash"}),
        ]
    );
}

#[test]
fn an_obligation_stated_by_an_attribute_on_no_test_item_names_the_attribute() {
    let proofs = Proofs {
        links: vec![link(9, 9, Some("never half an entry"))],
        items: Vec::new(),
    };
    let planned = planned_with(&[invariant(&["never half an entry"])], &proofs);
    assert_eq!(field(&planned, DURABLE, "proof"), None);
    assert_eq!(
        unproven(&planned),
        [Why::Unproven {
            tests: vec!["src/lib.rs:9".to_string()]
        }]
    );
}

#[test]
fn an_obligation_no_attribute_states_is_unproven_with_no_tests() {
    let proofs = Proofs {
        links: vec![link(4, 5, Some("another text"))],
        items: vec![item("rust:store::tests::survives", 5)],
    };
    let planned = planned_with(&[invariant(&["stated nowhere"])], &proofs);
    assert_eq!(field(&planned, DURABLE, "proof"), None);
    assert_eq!(unproven(&planned), [Why::Unproven { tests: Vec::new() }]);
}

#[test]
fn an_item_naming_the_invariant_on_another_line_proves_nothing() {
    let proofs = Proofs {
        links: vec![link(4, 5, Some("survives a crash"))],
        items: vec![item("rust:store::tests::elsewhere", 12)],
    };
    assert_eq!(
        proofs.of(DURABLE, "survives a crash"),
        Proved::Unproven(vec!["src/lib.rs:4".to_string()])
    );
}

#[test]
fn a_source_loading_rust_source_with_no_recorded_item_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("spec")).unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "uncollected", "spec_root": "spec", "extensions": ["@code/core", "@rust/cargo", "@rust/source"]}"#,
    )
    .unwrap();
    let Err(why) = read::compile(dir.path()) else {
        panic!("an uncollected source is refused");
    };
    assert!(
        why.starts_with(
            "it loads @rust/source and holds no recorded item: collect the source first"
        ),
        "{why}"
    );
}

#[test]
fn the_summary_lists_the_cited_tests() {
    let proofs = Proofs {
        links: vec![
            link(4, 5, Some("survives a crash")),
            link(9, 10, Some("is on disk")),
        ],
        items: vec![
            item("rust:store::tests::survives", 5),
            item("rust:store::tests::on_disk", 10),
        ],
    };
    let planned = planned_with(
        &[invariant(&["survives a crash", "is on disk", "nowhere"])],
        &proofs,
    );
    let cited: Vec<&str> = planned.cited.iter().map(String::as_str).collect();
    assert_eq!(
        cited,
        ["rust:store::tests::on_disk", "rust:store::tests::survives"]
    );
    let whys = unproven(&planned);
    let unproven = whys
        .iter()
        .filter(|why| matches!(why, Why::Unproven { .. }))
        .count();
    assert_eq!(
        (whys.len(), unproven),
        (3, 1),
        "two collapsed into their tests, one unproven"
    );
}
