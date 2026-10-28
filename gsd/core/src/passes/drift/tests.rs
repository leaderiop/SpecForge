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

fn context(digest: &str) -> PassEntity {
    entity("v1.0/02/02-CONTEXT", "context", &[("digest", digest)])
}

#[test]
fn research_recorded_against_another_digest_is_w406() {
    let found = check(&input(vec![
        context("fnv1a64:2"),
        entity(
            "v1.0/02/02-RESEARCH",
            "research",
            &[("context_digest", "fnv1a64:1")],
        ),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W406");
    assert_eq!(found[0].entity.as_deref(), Some("v1.0/02/02-RESEARCH"));
    assert!(
        found[0]
            .message
            .contains("'v1.0/02/02-CONTEXT' changed since"),
        "{}",
        found[0].message
    );
    let suggestion = found[0].suggestion.as_deref().unwrap_or_default();
    assert!(
        suggestion.contains("/gsd-plan-phase 02 --research"),
        "{suggestion}"
    );
    assert!(suggestion.contains("do not hand-inject"), "{suggestion}");
}

#[test]
fn a_spec_recorded_against_another_digest_is_w406_and_a_matching_one_is_clean() {
    let found = check(&input(vec![
        context("fnv1a64:2"),
        entity(
            "v1.0/02/02-SPEC",
            "phase_spec",
            &[("context_digest", "fnv1a64:1")],
        ),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.starts_with("phase spec "),
        "{}",
        found[0].message
    );
    let clean = check(&input(vec![
        context("fnv1a64:2"),
        entity(
            "v1.0/02/02-SPEC",
            "phase_spec",
            &[("context_digest", "fnv1a64:2")],
        ),
    ]));
    assert!(clean.is_empty(), "{clean:?}");
}

#[test]
fn research_without_a_digest_is_not_judged() {
    let found = check(&input(vec![
        context("fnv1a64:2"),
        entity("v1.0/02/02-RESEARCH", "research", &[]),
        // No context in the phase, and the project's research.
        entity(
            "v1.0/03/03-RESEARCH",
            "research",
            &[("context_digest", "fnv1a64:1")],
        ),
        entity("RESEARCH", "research", &[]),
    ]));
    assert!(found.is_empty(), "{found:?}");
}
