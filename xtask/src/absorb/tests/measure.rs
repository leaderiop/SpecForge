//! The table is the measure (ADR 0360 D2): every field the old builtins register has exactly one row, every row a
//! home, and every home a field `@gsd/core` declares, authored or computed as the home says, of the type it writes.

use serde_json::Value;
use specforge_ops::authoring::edit;
use specforge_ops::view::ProjectView;
use specforge_protocol_types::FieldType;
use std::collections::BTreeMap;

use super::super::table::{self, Home, How, OLD_KINDS, Pending, TABLE, VERIFY};
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
            Home::Dropped(_) => "dropped",
        };
        *counts.entry(class).or_default() += 1;
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("to", 18),
            ("computed", 3),
            ("later", 13),
            ("refused", 8),
            ("dropped", 99)
        ])
    );
    for row in TABLE {
        if row.homes.iter().any(|h| matches!(h, Home::Dropped(_))) {
            assert!(
                matches!(row.homes, [Home::Dropped(reason)] if !reason.trim().is_empty()),
                "{}.{}: a dropped value has no other home, and a reason",
                row.kind,
                row.field
            );
        }
    }
    assert_eq!(
        pending,
        BTreeMap::from([
            (Pending::Package, 9),
            (Pending::Truth, 3),
            (Pending::Behavior, 1)
        ])
    );
}

#[test]
fn every_kind_that_takes_verify_has_one_obligation_row() {
    let old = fixture();
    let view = ProjectView::of(&old.project);
    let taking: Vec<&str> = OLD_KINDS
        .into_iter()
        .filter(|kind| !edit::verify_kinds_of(&view, kind).is_empty())
        .collect();
    let rows: Vec<&str> = TABLE
        .iter()
        .filter(|r| r.field == VERIFY)
        .map(|r| r.kind)
        .collect();
    assert_eq!(rows, taking);
    assert_eq!(rows, ["constraint", "failure_mode", "invariant"]);
}

/// The field type a home written `how` holds, from the old field's type (`None` for an obligation row, which no old
/// builtin registers). `As` writes the old value as it is, so its home is of the old field's own type; every other way
/// makes one shape, whatever the old one. Exhaustive: a new way of writing is classified here before it compiles.
fn written(how: How, old: Option<FieldType>) -> Vec<FieldType> {
    use FieldType::*;
    match how {
        How::As => old.into_iter().collect(),
        How::Text | How::Solution => vec![String],
        How::Parent => vec![Reference],
        How::Requirements | How::Earlier => vec![ReferenceList],
        How::FeatureTier | How::Outcome => vec![Enum],
        How::Settled => vec![Bool],
        // The id the map assigns writes no field: the golden check skips its sentinel.
        How::Id => Vec::new(),
    }
}

#[test]
fn every_home_names_a_field_the_gsd_core_golden_declares_of_the_type_it_writes() {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../gsd/core/tests/golden/kinds.json"
        ))
        .unwrap(),
    )
    .unwrap();
    // kind → field → its writer (empty: authored) and its type.
    let mut declared: BTreeMap<String, BTreeMap<String, (String, FieldType)>> = BTreeMap::new();
    for kind in golden["entities"]["items"].as_array().unwrap() {
        let fields = kind["fields"].as_array().unwrap().iter().map(|f| {
            let field_type: FieldType = serde_json::from_value(f["field_type"].clone()).unwrap();
            (
                f["name"].as_str().unwrap().to_string(),
                (
                    f["writer"].as_str().unwrap_or_default().to_string(),
                    field_type,
                ),
            )
        });
        declared.insert(kind["name"].as_str().unwrap().to_string(), fields.collect());
    }
    let old = fixture();
    let view = ProjectView::of(&old.project);
    let (mut homes, mut typed) = (0, 0);
    for row in TABLE {
        let old_type = edit::fields_of(&view, row.kind)
            .into_iter()
            .find(|f| f.name == row.field)
            .map(|f| f.field_type);
        for home in row.homes {
            let (kind, field, writer, how) = match *home {
                Home::To { field, .. } if field == table::ID => continue,
                Home::To { kind, field, how } => (kind, field, "", Some(how)),
                Home::Computed { kind, field, .. } => (kind, field, "computed", None),
                _ => continue,
            };
            homes += 1;
            let at = format!("{}.{} → {kind}.{field}", row.kind, row.field);
            let held = declared.get(kind).and_then(|f| f.get(field));
            assert_eq!(held.map(|(w, _)| w.as_str()), Some(writer), "{at}");
            if let (Some(how), Some((_, field_type))) = (how, held) {
                typed += 1;
                let writes = written(how, old_type);
                assert!(
                    writes.contains(field_type),
                    "{at}: {how:?} from {old_type:?} writes {writes:?}, the golden declares {field_type:?}"
                );
            }
        }
    }
    assert_eq!(
        (homes, typed),
        (22, 18),
        "every To and Computed home, and the To homes typed"
    );
    for old in OLD_KINDS {
        if let Some(kind) = table::new_kind(old) {
            assert!(declared.contains_key(kind), "{old} → {kind}");
        }
    }
}
