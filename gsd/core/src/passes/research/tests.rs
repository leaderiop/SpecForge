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

#[test]
fn an_open_question_without_resolved_by_is_w204() {
    let found = check(&input(vec![
        entity("v1.0/02/02-RESEARCH", "research", &[("phase", "v1.0/02")]),
        entity("v1.0/02/02-RESEARCH/OQ1", "open_question", &[]),
        entity(
            "v1.0/02/02-RESEARCH/OQ2",
            "open_question",
            &[("resolved_by", "D-01")],
        ),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W204");
    assert_eq!(found[0].entity.as_deref(), Some("v1.0/02/02-RESEARCH/OQ1"));
}

#[test]
fn a_project_research_question_is_not_w204() {
    let found = check(&input(vec![
        entity("RESEARCH", "research", &[]),
        entity("RESEARCH/OQ1", "open_question", &[]),
    ]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_misnamed_phaseless_research_gets_w204_like_a_phase_research() {
    let found = check(&input(vec![
        entity("03-RESEARCH", "research", &[]),
        entity("03-RESEARCH/OQ1", "open_question", &[]),
    ]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W204");
}
