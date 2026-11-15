//! The table is the measure (ADR 0360 D2): every field the old builtins register has exactly one row, every row a
//! home, and every home a field `@gsd/core` declares, authored or computed as the home says.

use serde_json::Value;
use specforge_ops::authoring::edit;
use specforge_ops::view::ProjectView;
use std::collections::BTreeMap;

use super::super::table::{self, Home, OLD_KINDS, Pending, TABLE, VERIFY};
use super::fixture;

#[test]
fn every_old_governance_and_product_field_has_exactly_one_row() {
    let old = fixture();
    let view = ProjectView::of(&old.project);
    let mut registered = Vec::new();
    for kind in OLD_KINDS {
        let fields = edit::fields_of(&view, kind);
        assert!(!fields.is_empty(), "the old builtins register {kind}");
        registered.extend(fields.into_iter().map(|f| (kind, f.name)));
    }
    assert_eq!(OLD_KINDS.len(), 13);
    assert_eq!(registered.len(), 141, "{registered:?}");
    for (kind, field) in &registered {
        let rows = TABLE
            .iter()
            .filter(|r| r.kind == *kind && r.field == field)
            .count();
        assert_eq!(rows, 1, "{kind}.{field} has {rows} rows");
    }
    let fields: Vec<_> = TABLE.iter().filter(|r| r.field != VERIFY).collect();
    assert_eq!(
        fields.len(),
        registered.len(),
        "no row for a field nobody registers"
    );
    assert!(
        TABLE.iter().all(|r| !r.homes.is_empty()),
        "no row without a home"
    );

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut pending: BTreeMap<Pending, usize> = BTreeMap::new();
    for row in &fields {
        let class = match row.homes[0] {
            Home::To { .. } => "to",
            Home::Computed { .. } => "computed",
            Home::Later(p) => {
                *pending.entry(p).or_default() += 1;
                "later"
            }
            Home::Refused => "refused",
            Home::Default(_) => "default",
        };
        *counts.entry(class).or_default() += 1;
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("to", 89),
            ("computed", 4),
            ("later", 31),
            ("refused", 16),
            ("default", 1)
        ])
    );
    assert_eq!(
        pending,
        BTreeMap::from([
            (Pending::Release, 14),
            (Pending::Package, 11),
            (Pending::Truth, 4),
            (Pending::Behavior, 2)
        ])
    );
    let obligations: Vec<&str> = TABLE
        .iter()
        .filter(|r| r.field == VERIFY)
        .map(|r| r.kind)
        .collect();
    assert_eq!(obligations, ["constraint", "failure_mode"]);
}

#[test]
fn every_home_names_a_field_the_gsd_core_golden_declares() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../gsd/core/tests/golden/kinds.json"
        ))
        .unwrap(),
    )
    .unwrap();
    // kind → field → its writer (empty: authored).
    let mut declared: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for kind in golden["entities"]["items"].as_array().unwrap() {
        let fields = kind["fields"].as_array().unwrap().iter().map(|f| {
            (
                f["name"].as_str().unwrap().to_string(),
                f["writer"].as_str().unwrap_or_default().to_string(),
            )
        });
        declared.insert(kind["name"].as_str().unwrap().to_string(), fields.collect());
    }
    let mut homes = 0;
    for row in TABLE {
        for home in row.homes {
            let (kind, field, writer) = match *home {
                Home::To { kind, field, .. } => (kind, field, ""),
                Home::Computed { kind, field, .. } => (kind, field, "computed"),
                _ => continue,
            };
            homes += 1;
            let at = format!("{}.{} → {kind}.{field}", row.kind, row.field);
            let held = declared.get(kind).and_then(|f| f.get(field));
            assert_eq!(held.map(String::as_str), Some(writer), "{at}");
        }
    }
    assert!(homes > 89, "{homes}");
    for old in OLD_KINDS {
        if let Some(kind) = table::new_kind(old) {
            assert!(declared.contains_key(kind), "{old} → {kind}");
        }
    }
}
