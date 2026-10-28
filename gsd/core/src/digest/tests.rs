use std::collections::BTreeMap;

use super::*;

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
        ..Default::default()
    }
}

fn input(entities: Vec<PassEntity>) -> PassInput {
    PassInput {
        entities,
        ..Default::default()
    }
}

const CONTEXT: &str = "v1.0/02/02-CONTEXT";

fn context_entities() -> Vec<PassEntity> {
    vec![
        entity(CONTEXT, "context", &[("boundary", "Tasks are saved")]),
        entity(
            "v1.0/02/D-01",
            "decision",
            &[("origin", "context"), ("text", "One JSON file")],
        ),
        entity("v1.0/02/DI1", "deferred_idea", &[("text", "Sync")]),
    ]
}

fn digest_of(entities: Vec<PassEntity>) -> String {
    context(&input(entities), CONTEXT).expect("a digest")
}

#[test]
fn the_digest_has_a_prefix_and_sixteen_hex_digits() {
    let digest = digest_of(context_entities());
    let hex = digest.strip_prefix("fnv1a64:").expect("the prefix");
    assert_eq!(hex.len(), 16, "{digest}");
    assert!(
        hex.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
}

#[test]
fn fnv1a64_matches_the_published_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn the_digest_ignores_computed_fields_and_entity_order() {
    let plain = digest_of(context_entities());
    let mut reordered = context_entities();
    reordered.reverse();
    assert_eq!(digest_of(reordered), plain);
    let mut computed = context_entities();
    computed[0]
        .fields
        .insert("status".into(), "gathering".into());
    computed[0]
        .fields
        .insert("digest".into(), "fnv1a64:0".into());
    assert_eq!(digest_of(computed), plain);
}

#[test]
fn a_decision_of_another_origin_is_not_in_the_digest() {
    let plain = digest_of(context_entities());
    let mut entities = context_entities();
    entities.push(entity(
        "v1.0/02/D-09",
        "decision",
        &[("origin", "summary"), ("text", "a summary decision")],
    ));
    assert_eq!(digest_of(entities.clone()), plain);
    // Nor is a decision of another phase, or of the project.
    entities.push(entity(
        "v1.0/03/D-01",
        "decision",
        &[("origin", "context"), ("text", "elsewhere")],
    ));
    entities.push(entity(
        "D-01",
        "decision",
        &[("origin", "context"), ("text", "project")],
    ));
    assert_eq!(digest_of(entities), plain);
}

#[test]
fn an_edited_decision_or_a_new_deferred_idea_changes_the_digest() {
    let plain = digest_of(context_entities());
    let mut edited = context_entities();
    edited[1].fields.insert("text".into(), "Two files".into());
    assert_ne!(digest_of(edited), plain);
    let mut added = context_entities();
    added.push(entity("v1.0/02/DI2", "deferred_idea", &[("text", "More")]));
    assert_ne!(digest_of(added), plain);
}

#[test]
fn structured_values_count_and_a_missing_context_has_no_digest() {
    let mut text_only = context_entities();
    text_only[0]
        .fields
        .insert("canonical_refs".into(), "topic".into());
    let mut structured = text_only.clone();
    structured[0].values.insert(
        "canonical_refs".into(),
        serde_json::json!([{ "topic": "x" }]),
    );
    assert_ne!(digest_of(structured), digest_of(text_only));
    assert_eq!(context(&input(vec![]), CONTEXT), None);
}
