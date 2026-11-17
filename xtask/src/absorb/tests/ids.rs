//! GSD's requirement and decision ids (ADR 0372 D3): a requirement is `<CAT>-<NN>`, its category and its prefix from
//! one rule; a decision in force is `D-<NN>` at project scope; numbers run in old-id order, at least two digits.

use regex::Regex;
use serde_json::json;
use specforge_ops::authoring::edit::{Edit, Input};

use super::super::plan::Planned;
use super::super::read::OldEntity;
use super::{field_of, new_path, old, one, planned};

fn constraint(id: &str, category: Option<&str>) -> OldEntity {
    let mut fields = json!({"description": "D"});
    if let Some(category) = category {
        fields["category"] = json!(category);
    }
    old("constraint", id, fields)
}

fn feature(id: &str) -> OldEntity {
    old("feature", id, json!({"problem": "P", "status": "accepted"}))
}

fn decision(id: &str) -> OldEntity {
    old(
        "decision",
        id,
        json!({"status": "accepted", "decision": "D"}),
    )
}

/// Each requirement create's (id, written category), in create order.
fn requirements(planned: &Planned) -> Vec<(String, String)> {
    planned
        .creates
        .iter()
        .filter_map(|edit| match edit {
            Edit::Create {
                kind, id, fields, ..
            } if kind == "requirement" => match fields.get("category") {
                Some(Input::One(category)) => Some((id.clone(), category.clone())),
                other => panic!("{id}: a requirement writes one category: {other:?}"),
            },
            _ => None,
        })
        .collect()
}

#[test]
fn a_requirement_id_is_its_category_prefix_and_a_number_in_old_id_order() {
    let planned = planned(&[
        feature("c"),
        feature("a"),
        feature("b"),
        old("invariant", "held", json!({"guarantee": "G"})),
    ]);
    for (kind, id, new) in [
        ("feature", "a", "FEATURE-01"),
        ("feature", "b", "FEATURE-02"),
        ("feature", "c", "FEATURE-03"),
        ("invariant", "held", "INVARIANT-01"),
    ] {
        assert_eq!(new_path(&planned, kind, id), new, "{kind} {id}");
    }
}

#[test]
fn a_constraint_category_word_is_its_requirement_category_and_prefix() {
    let planned = planned(&[
        constraint("fast", Some("performance")),
        constraint("plain", None),
    ]);
    assert_eq!(new_path(&planned, "constraint", "fast"), "PERFORMANCE-01");
    assert_eq!(
        field_of(&planned, "constraint", "fast", "category"),
        Some(&one("Performance"))
    );
    assert!(
        planned
            .ledger
            .entries()
            .all(|e| !(e.id == "fast" && e.field == "category")),
        "the class is the heading, written: nothing is ledgered for it"
    );
    assert_eq!(new_path(&planned, "constraint", "plain"), "CONSTRAINT-01");
    assert_eq!(
        field_of(&planned, "constraint", "plain", "category"),
        Some(&one("Constraint"))
    );
    assert!(
        planned.ledger.entries().any(|e| e.id == "plain"
            && e.field == "category"
            && matches!(&e.why, super::super::ledger::Why::Filled { field, value }
                if field == "requirement.category" && value == "Constraint")),
        "a constraint with no category is filled, ledgered"
    );
}

#[test]
fn a_category_with_no_usable_leading_letter_falls_back_to_the_old_kind() {
    let planned = planned(&[
        constraint("integrity", Some("data-integrity")),
        constraint("lives", Some("9lives")),
        constraint("marks", Some("--- !")),
    ]);
    assert_eq!(
        new_path(&planned, "constraint", "integrity"),
        "DATAINTEGRITY-01"
    );
    assert_eq!(
        field_of(&planned, "constraint", "integrity", "category"),
        Some(&one("Data-integrity"))
    );
    assert_eq!(new_path(&planned, "constraint", "lives"), "CONSTRAINT-01");
    assert_eq!(new_path(&planned, "constraint", "marks"), "CONSTRAINT-02");
    let prefix = Regex::new(r"^[A-Z][A-Z0-9]*-\d+$").unwrap();
    for (id, _) in requirements(&planned) {
        assert!(prefix.is_match(&id), "{id}");
    }
}

#[test]
fn kinds_sharing_a_prefix_share_one_counter_in_kind_then_id_order() {
    let planned = planned(&[
        feature("zz"),
        constraint("aa", Some("feature")),
        feature("ab"),
    ]);
    for (kind, id, new) in [
        ("constraint", "aa", "FEATURE-01"),
        ("feature", "ab", "FEATURE-02"),
        ("feature", "zz", "FEATURE-03"),
    ] {
        assert_eq!(new_path(&planned, kind, id), new, "{kind} {id}");
    }
}

#[test]
fn numbers_have_at_least_two_digits_and_as_many_as_the_group_needs() {
    let three: Vec<OldEntity> = ["a", "b", "c"].map(feature).to_vec();
    let three = planned(&three);
    assert_eq!(
        requirements(&three)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["FEATURE-01", "FEATURE-02", "FEATURE-03"]
    );
    let hundred: Vec<OldEntity> = (0..100).map(|n| feature(&format!("f{n:03}"))).collect();
    let hundred = planned(&hundred);
    assert_eq!(new_path(&hundred, "feature", "f000"), "FEATURE-001");
    assert_eq!(new_path(&hundred, "feature", "f099"), "FEATURE-100");
}

#[test]
fn a_decision_in_force_is_d_nn_at_project_scope_in_old_id_order() {
    let planned = planned(&[decision("zeta"), decision("alpha")]);
    assert_eq!(new_path(&planned, "decision", "alpha"), "D-01");
    assert_eq!(new_path(&planned, "decision", "zeta"), "D-02");
    for id in ["alpha", "zeta"] {
        assert_eq!(
            field_of(&planned, "decision", id, "phase"),
            None,
            "{id}: project scope"
        );
    }
}

#[test]
fn every_new_id_matches_its_grammar() {
    let planned = planned(&[
        feature("store"),
        constraint("fast", Some("performance")),
        constraint("odd", Some("9lives")),
        old("invariant", "held", json!({"guarantee": "G"})),
        decision("parser"),
        old(
            "release",
            "one",
            json!({"version": "0.1.0", "milestones": ["first"]}),
        ),
        old(
            "milestone",
            "first",
            json!({"description": "D", "exit_criteria": ["E"], "features": ["store"]}),
        ),
    ]);
    let requirement = Regex::new(r"^[A-Z][A-Z0-9]*-\d+$").unwrap();
    let decision = Regex::new(r"^D-\d+$").unwrap();
    let milestone = Regex::new(r"^v\d").unwrap();
    let phase = Regex::new(r"^v\d[^/]*/\d{2,}$").unwrap();
    for (kind, old_id, new) in planned.ids.entries() {
        let grammar = match new.kind {
            "requirement" => &requirement,
            "decision" => &decision,
            "milestone" => &milestone,
            "phase" => &phase,
            other => panic!("{kind} {old_id}: no grammar for {other}"),
        };
        assert!(
            grammar.is_match(&new.path()),
            "{kind} {old_id}: {}",
            new.path()
        );
    }
    assert_eq!(planned.ids.entries().count(), 7);
}

#[test]
fn the_written_category_is_the_id_prefix_for_every_requirement() {
    let planned = planned(&[
        feature("store"),
        constraint("fast", Some("performance")),
        constraint("join", Some("inter-operability")),
        constraint("plain", None),
        old("invariant", "held", json!({"guarantee": "G"})),
    ]);
    let written = requirements(&planned);
    assert_eq!(written.len(), 5);
    for (id, category) in written {
        let letters: String = category
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_uppercase();
        assert_eq!(id.split_once('-').map(|(p, _)| p), Some(letters.as_str()));
    }
}
