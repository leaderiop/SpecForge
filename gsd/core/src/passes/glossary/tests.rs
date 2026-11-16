use std::collections::BTreeMap;

use super::*;

/// A term with its `name` and its lists, `(field, items)`.
fn term(id: &str, name: &str, lists: &[(&str, &[&str])]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "term".into(),
        fields: BTreeMap::from([("name".to_string(), name.to_string())]),
        values: lists
            .iter()
            .map(|(field, items)| (field.to_string(), serde_json::json!(items)))
            .collect(),
        ..Default::default()
    }
}

fn input(entities: Vec<PassEntity>) -> PassInput {
    PassInput {
        entities,
        ..Default::default()
    }
}

fn found(entities: Vec<PassEntity>) -> Vec<(String, String, String)> {
    check(&input(entities))
        .into_iter()
        .map(|d| {
            assert_eq!(d.code, "W538");
            assert_eq!(d.severity, PassSeverity::Warning);
            (
                d.entity.unwrap_or_default(),
                d.message,
                d.suggestion.unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn a_term_that_avoids_its_own_alias_is_w538() {
    let found = found(vec![term(
        "work_item",
        "task",
        &[("aliases", &["to-do"]), ("avoid", &["ticket", "To-Do"])],
    )]);
    assert_eq!(
        found,
        [(
            "work_item".to_string(),
            "term 'work_item' avoids \"To-Do\", which is its own alias".to_string(),
            "keep \"To-Do\" on one side: remove it from `avoid`, or from the `name` or `aliases` that accept it"
                .to_string(),
        )]
    );
}

#[test]
fn a_term_that_avoids_its_own_name_is_w538() {
    // Unicode lowercase on both sides, and a run of whitespace is one space.
    let found = found(vec![term(
        "eleve",
        "Élève  inscrit",
        &[("avoid", &["élève inscrit"])],
    )]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].1,
        "term 'eleve' avoids \"élève inscrit\", which is its own name"
    );
}

#[test]
fn a_term_that_avoids_another_terms_word_is_w538_naming_every_place() {
    let found = found(vec![
        term("customer", "customer", &[("avoid", &["client", "CLIENT"])]),
        term("api_client", "client", &[]),
        term("account", "account", &[("aliases", &["client"])]),
    ]);
    // One warning per term and avoided word, on the avoiding term, naming each term that accepts it in id order.
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, "customer");
    assert_eq!(
        found[0].1,
        "term 'customer' avoids \"client\", which is an alias of term 'account', the name of term 'api_client'"
    );
}

#[test]
fn a_glossary_whose_avoided_words_are_accepted_nowhere_is_not_w538() {
    let found = found(vec![
        term(
            "work_item",
            "task",
            &[("aliases", &["to-do"]), ("avoid", &["ticket", "todo"])],
        ),
        term("board", "board", &[("avoid", &["tasks"])]),
        // An avoid word that only contains an accepted word is another word.
        term("team", "team", &[("avoid", &["task force"])]),
        // A name written twice among its aliases contradicts nothing.
        term("plan", "plan", &[("aliases", &["Plan"])]),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn only_terms_are_read() {
    // A requirement's or a journey's `name` and lists are no glossary.
    let mut journey = term("checkout", "pay", &[("avoid", &["pay"])]);
    journey.kind = "journey".into();
    assert!(found(vec![journey]).is_empty());
}
