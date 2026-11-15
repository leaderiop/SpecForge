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

/// A pass input of `entities` (JSON).
fn input(entities: serde_json::Value) -> PassInput {
    serde_json::from_value(json!({ "entities": entities, "edges": [] })).expect("a pass input")
}

/// `(code, entity, message, suggestion)` of each diagnostic.
fn found(input: &PassInput) -> Vec<(String, String, String, String)> {
    check(input)
        .into_iter()
        .map(|d| {
            (
                d.code,
                d.entity.unwrap_or_default(),
                d.message,
                d.suggestion.unwrap_or_default(),
            )
        })
        .collect()
}

fn term(id: &str, name: &str, avoid: &[&str]) -> serde_json::Value {
    json!({ "id": id, "kind": "term", "fields": { "name": name, "definition": "a word" },
            "values": { "avoid": avoid } })
}

#[test]
fn an_avoided_synonym_in_a_requirement_is_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        { "id": "TASK-01", "kind": "requirement", "fields": { "text": "A user can add a Ticket" } },
    ])));
    assert_eq!(
        found,
        [(
            "W534".to_string(),
            "TASK-01".to_string(),
            "requirement 'TASK-01' writes \"ticket\" in text, a word avoided by term 'work_item'"
                .to_string(),
            "write \"task\" (term 'work_item') instead".to_string(),
        )]
    );
}

/// French: an accent and a capital match ("l'Élève" holds "élève"), a longer word does not ("élèves").
#[test]
fn an_avoided_french_word_is_w534_and_a_longer_word_is_not() {
    let found = found(&input(json!([
        term("apprenant", "apprenant", &["élève"]),
        { "id": "R1", "kind": "requirement", "fields": { "text": "Gérer l'Élève" } },
        { "id": "R2", "kind": "requirement", "fields": { "text": "Lister les élèves" } },
    ])));
    let on: Vec<&str> = found
        .iter()
        .map(|(_, entity, _, _)| entity.as_str())
        .collect();
    assert_eq!(on, ["R1"], "{found:#?}");
}

/// Two terms avoid "client", which a decision writes twice in two fields: one W534, naming both terms and the first
/// field that holds it.
#[test]
fn one_w534_per_entity_and_word_naming_every_term() {
    let found = found(&input(json!([
        term("customer", "customer", &["client"]),
        term("buyer", "buyer", &["Client"]),
        { "id": "D1", "kind": "decision",
          "fields": { "text": "Bill the client", "rationale": "a client pays" } },
    ])));
    assert_eq!(found.len(), 1, "{found:#?}");
    let (code, entity, message, suggestion) = &found[0];
    assert_eq!((code.as_str(), entity.as_str()), ("W534", "D1"));
    assert_eq!(
        message,
        "decision 'D1' writes \"Client\" in text, a word avoided by term 'buyer' and term 'customer'"
    );
    assert_eq!(
        suggestion,
        "write \"buyer\" (term 'buyer') or \"customer\" (term 'customer') instead"
    );
}

/// A term that avoids another term's name contradicts the glossary: that is gsd/W538 on the term, and no prose that
/// writes the accepted word is W534 for it. A term is no prose.
#[test]
fn a_term_that_avoids_a_terms_name_is_w538_and_not_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["board"]),
        term("board", "board", &[]),
        { "id": "SHARE-01", "kind": "requirement", "fields": { "text": "A user can share a board" } },
    ])));
    assert_eq!(found, []);
}

#[test]
fn prose_without_an_avoided_word_is_not_w534() {
    let found = found(&input(json!([
        term("work_item", "task", &["ticket"]),
        { "id": "TASK-01", "kind": "requirement", "fields": { "text": "A user can add a task; tickets are gone" } },
        { "id": "T1", "kind": "task", "fields": { "action": "close the ticket" } },
    ])));
    assert_eq!(found, []);
}
