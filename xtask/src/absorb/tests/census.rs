//! The census (ADR 0372 D8, ADR 0364 D4): per old kind, over this repository's own spec read as an old project and over
//! absorb's fixture, every entity is converted (it is in the id map) or dropped whole (its title is ledgered
//! `dropped`), never both and never neither, and an entity dropped whole leaves a `dropped` entry for each of its
//! values. Over the root spec, the id map is total, one to one, repeatable and GSD's; the pure plan's ledger holds one
//! outcome per entry; and every create writes only kinds and authored fields `@gsd/core` declares.
//!
//! Pure: no target and no `GSD_CORE_WASM`, so it runs in the unit slice and the gate's `tests` check. The root spec is
//! read as an old project, so the swap removes the root half with the self-spec sample (ADR 0367 D1); the fixture,
//! which holds every old kind, is what keeps the census after it.

use regex::Regex;
use serde_json::{Value, json};
use specforge_ops::authoring::edit::{Edit, Input};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use super::super::ledger::Why;
use super::super::plan::{self, Planned, Stop};
use super::super::read::{self, OldEntity};
use super::super::table::FATES;
use super::{fixture, old};

/// The structural keywords of the core grammar: an id no project may take.
const STRUCTURAL: [&str; 4] = ["spec", "ref", "use", "define"];

/// The root spec's authored entities, compiled once for the tests that read them.
static ROOT: LazyLock<Vec<OldEntity>> = LazyLock::new(|| {
    read::compile(specforge_repo_root::repo_root())
        .expect("the self-spec compiles clean")
        .entities()
});

/// One old kind's count: its entities, the converted and the dropped whole.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Count {
    entities: usize,
    converted: usize,
    dropped: usize,
}

/// The census of `entities`: the plan, and each old kind's count; the pre-checks' stops when the run cannot convert
/// them (a kind with no fate, a converting kind's field with no row). Panics, naming it, on an entity both converted
/// and dropped whole, on one that is neither, and on a value of an entity dropped whole that is not ledgered `dropped`.
fn census(entities: &[OldEntity]) -> Result<(Planned, BTreeMap<String, Count>), Vec<Stop>> {
    plan::prechecks(entities)?;
    let planned = plan::plan(entities);
    let dropped: BTreeSet<(&str, &str, &str, String)> = planned
        .ledger
        .entries()
        .filter(|e| matches!(e.why, Why::Dropped(_)))
        .map(|e| {
            (
                e.kind.as_str(),
                e.id.as_str(),
                e.field.as_str(),
                e.value.to_string(),
            )
        })
        .collect();
    let mut counts: BTreeMap<String, Count> = BTreeMap::new();
    for e in entities {
        let count = counts.entry(e.kind.clone()).or_default();
        count.entities += 1;
        let converted = planned.ids.get(&e.kind, &e.id).is_some();
        let title = e.title.clone().map_or(Value::Null, Value::String);
        let whole = dropped.contains(&(&e.kind, &e.id, "title", title.to_string()));
        assert!(
            converted != whole,
            "{} '{}': converted {converted}, dropped whole {whole}: one fate each (ADR 0372 D1)",
            e.kind,
            e.id
        );
        if converted {
            count.converted += 1;
            continue;
        }
        count.dropped += 1;
        let values = e.fields.iter().map(|(f, v)| (f.as_str(), v.clone()));
        let obligations = e
            .obligations
            .iter()
            .map(|(kind, text)| ("verify", json!({"kind": kind, "text": text})));
        let methods = e.methods.iter().map(|m| ("method", m.clone()));
        let annotations = (!e.annotations.is_empty()).then(|| {
            let held = e.annotations.clone().into_iter().collect();
            ("annotations", Value::Object(held))
        });
        for (field, value) in values.chain(obligations).chain(methods).chain(annotations) {
            assert!(
                dropped.contains(&(&e.kind, &e.id, field, value.to_string())),
                "{} '{}' {field}: an entity dropped whole ledgers each value (ADR 0372 D2)",
                e.kind,
                e.id
            );
        }
    }
    Ok((planned, counts))
}

/// The old kinds the fate table names (ADR 0372 D1).
fn fated() -> BTreeSet<String> {
    FATES.iter().map(|(kind, _)| kind.to_string()).collect()
}

/// `(kind, converted, dropped whole)` as counts.
fn pins(rows: &[(&str, usize, usize)]) -> BTreeMap<String, Count> {
    rows.iter()
        .map(|&(kind, converted, dropped)| {
            let count = Count {
                entities: converted + dropped,
                converted,
                dropped,
            };
            (kind.to_string(), count)
        })
        .collect()
}

/// The totals of `counts`.
fn total(counts: &BTreeMap<String, Count>) -> Count {
    counts.values().fold(Count::default(), |sum, c| Count {
        entities: sum.entities + c.entities,
        converted: sum.converted + c.converted,
        dropped: sum.dropped + c.dropped,
    })
}

/// `@gsd/core`'s kinds golden: each kind and its fields, each with its writer (empty: authored).
fn gsd_kinds() -> BTreeMap<String, BTreeMap<String, String>> {
    let golden: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../gsd/core/tests/golden/kinds.json"
        ))
        .unwrap(),
    )
    .unwrap();
    golden["entities"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|kind| {
            let fields = kind["fields"].as_array().unwrap().iter().map(|f| {
                (
                    f["name"].as_str().unwrap().to_string(),
                    f["writer"].as_str().unwrap_or_default().to_string(),
                )
            });
            (kind["name"].as_str().unwrap().to_string(), fields.collect())
        })
        .collect()
}

#[test]
fn the_root_spec_census_holds_per_old_kind() {
    let kinds: BTreeSet<String> = ROOT.iter().map(|e| e.kind.clone()).collect();
    assert_eq!(kinds, fated(), "the root spec holds the 17 fated kinds");
    let (_, counts) = census(&ROOT).expect("the root spec converts");
    // ADR 0372 D1's fate table: a change here is read against it.
    let expected = pins(&[
        ("feature", 141, 0),
        ("constraint", 63, 0),
        ("invariant", 149, 0),
        ("decision", 100, 17),
        ("release", 3, 0),
        ("milestone", 13, 5),
        ("deliverable", 0, 12),
        ("failure_mode", 0, 35),
        ("journey", 0, 54),
        ("term", 0, 100),
        ("persona", 0, 6),
        ("channel", 0, 5),
        ("module", 0, 54),
        ("behavior", 0, 706),
        ("event", 0, 123),
        ("type", 0, 678),
        ("port", 0, 26),
    ]);
    for (kind, count) in &expected {
        assert_eq!(
            counts.get(kind),
            Some(count),
            "{kind}: entities, converted, dropped whole, as ADR 0372 D1's fate table gives them"
        );
    }
    assert_eq!(counts, expected);
    assert_eq!(
        total(&counts),
        Count {
            entities: 2_290,
            converted: 469,
            dropped: 1_821
        },
        "ADR 0372 D1: 2,290 authored entities (with the spec block, the 2,291 of the baseline)"
    );
}

#[test]
fn the_fixture_census_holds_per_old_kind() {
    let entities = fixture().entities();
    let kinds: BTreeSet<String> = entities.iter().map(|e| e.kind.clone()).collect();
    let mut fated = fated();
    fated.insert("ref".to_string());
    assert_eq!(
        kinds, fated,
        "the fixture holds every old kind (ADR 0364 D4) and a ref declaration (ADR 0372 D1)"
    );
    let (_, counts) = census(&entities).expect("the fixture converts");
    assert_eq!(
        counts,
        pins(&[
            ("feature", 3, 0),
            ("constraint", 5, 0),
            ("invariant", 2, 0),
            ("decision", 2, 5),
            ("release", 2, 0),
            ("milestone", 3, 1),
            ("deliverable", 0, 2),
            ("failure_mode", 0, 2),
            ("journey", 0, 2),
            ("term", 0, 2),
            ("persona", 0, 1),
            ("channel", 0, 2),
            ("module", 0, 1),
            ("behavior", 0, 1),
            ("event", 0, 1),
            ("type", 0, 1),
            ("port", 0, 1),
            ("ref", 0, 1),
        ]),
        "the fixture's fates, as ADR 0372 D1 gives them"
    );
    assert_eq!(
        total(&counts),
        Count {
            entities: 40,
            converted: 17,
            dropped: 23
        }
    );
}

#[test]
fn the_root_spec_ids_are_total_one_to_one_repeatable_and_grammatical() {
    let (planned, _) = census(&ROOT).expect("the root spec converts");
    let created: Vec<(String, String)> = planned
        .creates
        .iter()
        .map(|edit| {
            let Edit::Create { kind, .. } = edit else {
                panic!("only creates: {edit:?}")
            };
            (kind.clone(), super::path_of(edit))
        })
        .collect();
    let mapped: Vec<(String, String)> = planned
        .ids
        .entries()
        .map(|(_, _, new)| (new.kind.to_string(), new.path()))
        .collect();
    let distinct: BTreeSet<&(String, String)> = created.iter().collect();
    assert_eq!(
        distinct.len(),
        created.len(),
        "no new path repeats within its kind"
    );
    assert_eq!(
        mapped.iter().collect::<BTreeSet<_>>(),
        distinct,
        "the map covers every create once, and nothing else"
    );
    assert_eq!(mapped.len(), created.len());
    assert_eq!(planned.ledger.ids().count(), created.len());

    let again = plan::plan(&ROOT);
    assert_eq!(planned.ids, again.ids, "a second plan gives the same map");
    assert_eq!(
        planned.ledger.to_json("specforge"),
        again.ledger.to_json("specforge"),
        "and the same ledger bytes"
    );

    let grammar = |kind: &str| match kind {
        "requirement" => r"^[A-Z][A-Z0-9]*-\d+$",
        "decision" => r"^D-\d+$",
        "milestone" => r"^v\d[0-9A-Za-z.\-]*$",
        "phase" => r"^\d{2,}$",
        other => panic!("no grammar for {other}"),
    };
    let mut keywords: BTreeSet<String> = gsd_kinds().into_keys().collect();
    assert!(
        keywords.contains("blocker"),
        "gsd declares a `blocker` kind"
    );
    keywords.extend(STRUCTURAL.map(String::from));
    for (kind, id, new) in planned.ids.entries() {
        assert!(
            Regex::new(grammar(new.kind)).unwrap().is_match(&new.name),
            "{kind} '{id}': {} matches no {} id",
            new.name,
            new.kind
        );
        assert!(
            !keywords.contains(&new.name),
            "{kind} '{id}': {} is a keyword the target refuses (E013)",
            new.name
        );
    }
    assert!(
        ROOT.iter().any(|e| e.kind == "term" && e.id == "blocker"),
        "the old term that stopped the first trial with E013"
    );
    assert!(
        planned.ids.get("term", "blocker").is_none(),
        "an old term is dropped whole (ADR 0372 D1)"
    );

    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut tiers: BTreeMap<String, usize> = BTreeMap::new();
    for edit in &planned.creates {
        let Edit::Create { kind, fields, .. } = edit else {
            continue;
        };
        *by_kind.entry(kind.as_str()).or_default() += 1;
        if let Some(Input::One(tier)) = fields.get("tier") {
            *tiers.entry(tier.clone()).or_default() += 1;
        }
    }
    assert_eq!(
        by_kind,
        BTreeMap::from([
            ("decision", 100),
            ("milestone", 3),
            ("phase", 13),
            ("requirement", 353)
        ]),
        "ADR 0372 D3, D4, D5"
    );
    assert_eq!(
        tiers,
        BTreeMap::from([("v1".to_string(), 69), ("v2".to_string(), 284)]),
        "ADR 0372 D7: a requirement no written phase lists is v2"
    );
}

#[test]
fn the_root_spec_ledger_holds_one_outcome_per_entry() {
    let (planned, _) = census(&ROOT).expect("the root spec converts");
    let classes: BTreeSet<String> = planned.ledger.counts().into_keys().collect();
    assert_eq!(
        classes,
        BTreeSet::from(["dropped".to_string(), "filled".to_string()]),
        "no target, so no computed comparison and no scaffold entry: {:?}",
        planned.ledger.counts()
    );
    let mut seen = BTreeSet::new();
    for e in planned.ledger.entries() {
        assert!(
            seen.insert((&e.kind, &e.id, &e.field, e.value.to_string())),
            "{} '{}' {} {}: two outcomes for one value",
            e.kind,
            e.id,
            e.field,
            e.value
        );
    }
}

#[test]
fn every_create_writes_only_authored_kinds_and_fields_of_gsd_core() {
    let (planned, _) = census(&ROOT).expect("the root spec converts");
    let declared = gsd_kinds();
    for edit in &planned.creates {
        let Edit::Create {
            kind,
            id,
            fields,
            verify,
            ..
        } = edit
        else {
            panic!("only creates: {edit:?}")
        };
        let Some(held) = declared.get(kind) else {
            panic!("{kind} '{id}': no kind of @gsd/core's kinds.json")
        };
        for field in fields.keys() {
            assert_eq!(
                held.get(field).map(String::as_str),
                Some(""),
                "{kind} '{id}' {field}: an authored field of @gsd/core's {kind}, none it computes and none \
                 Phase 3 or 4 removed"
            );
        }
        assert!(verify.is_empty(), "{kind} '{id}': no obligation is written");
    }
}

#[test]
fn a_kind_with_no_fate_fails_the_census() {
    let mut entities = fixture().entities();
    entities.push(old("widget", "gear", json!({"size": 3})));
    let stops = census(&entities).expect_err("a kind with no fate stops the census");
    assert!(
        stops
            .iter()
            .any(|s| s.to_string() == "widget 'gear': no fate for kind widget (ADR 0372 D1)"),
        "{stops:?}"
    );
}
