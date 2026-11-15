use gsd_kit::dsl::{Kind, Ty};
use serde_json::json;

use super::*;

fn declared(kind: &str) -> &'static Kind {
    crate::kinds::KINDS
        .iter()
        .find(|k| k.name == kind)
        .unwrap_or_else(|| panic!("{kind} is declared"))
}

/// Each path of the table is a text field of a declared kind, or a text member of one of its block lists: a later
/// rename fails here rather than leave the vocabulary reading nothing.
#[test]
fn every_prose_field_is_a_declared_text_field() {
    for (kind, paths) in PROSE {
        let fields = declared(kind).fields;
        for path in *paths {
            let (name, member) = path
                .split_once('.')
                .map_or((*path, None), |(l, m)| (l, Some(m)));
            let field = fields
                .iter()
                .find(|f| f.name == name)
                .unwrap_or_else(|| panic!("{kind}.{name} is declared"));
            let ty = match (member, field.ty) {
                (None, ty) => ty,
                (Some(member), Ty::Blocks(members)) => {
                    members
                        .iter()
                        .find(|m| m.name == member)
                        .unwrap_or_else(|| panic!("{kind}.{path} is declared"))
                        .ty
                }
                (Some(_), _) => panic!("{kind}.{name} is no block list"),
            };
            assert!(matches!(ty, Ty::Str), "{kind}.{path} is a text field");
        }
    }
}

/// `term.used_in` lists entities of exactly the kinds that write prose.
#[test]
fn the_used_in_union_is_the_prose_kinds() {
    let used_in = declared("term")
        .fields
        .iter()
        .find(|f| f.name == "used_in")
        .expect("term.used_in");
    let Ty::RefsAny(kinds) = used_in.ty else {
        panic!("term.used_in is a union list");
    };
    let prose: Vec<&str> = PROSE.iter().map(|(kind, _)| *kind).collect();
    assert_eq!(kinds, prose.as_slice());
}

#[test]
fn a_journey_step_action_is_prose() {
    let journey: PassEntity = serde_json::from_value(json!({
        "id": "add_and_finish", "kind": "journey",
        "fields": { "persona": "owner" },
        "values": { "steps": [
            { "action": "adds a task", "requirement": "TASK-01" },
            { "action": "marks it complete" },
            { "action": "  " },
        ] },
    }))
    .expect("a journey");
    assert_eq!(
        prose(&journey),
        [
            ("step 1".to_string(), "adds a task".to_string()),
            ("step 2".to_string(), "marks it complete".to_string()),
        ]
    );
    let places = prose(&journey);
    assert_eq!(uses(&places, "Complete"), Some("step 2"));
    assert_eq!(uses(&places, "owner"), None);
}

/// A decision's three prose fields, in the table's order; its `area` is no prose.
#[test]
fn a_decision_reads_its_text_rationale_and_context() {
    let decision: PassEntity = serde_json::from_value(json!({
        "id": "v1.0/01/D1", "kind": "decision",
        "fields": { "text": "Store tasks", "area": "storage", "context": "a board",
                    "rationale": "a board needs them" },
    }))
    .expect("a decision");
    let places: Vec<String> = prose(&decision)
        .into_iter()
        .map(|(place, _)| place)
        .collect();
    assert_eq!(places, ["text", "rationale", "context"]);
    assert_eq!(uses(&prose(&decision), "board"), Some("rationale"));
    assert_eq!(uses(&prose(&decision), "storage"), None);
}
