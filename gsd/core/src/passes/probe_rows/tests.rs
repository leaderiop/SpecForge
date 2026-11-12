use std::collections::BTreeMap;

use super::*;

const SPEC: &str = "v1.0/02/02-SPEC";
const LR1: &str = "v1.0/02/02-SPEC/LR1";

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

fn edge(source: &str, label: &str, target: &str) -> PassEdge {
    PassEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

/// The spec, its locked requirement LR1, and `rows`, each pointing at the spec and at LR1.
fn input(rows: Vec<PassEntity>) -> PassInput {
    let mut edges = vec![edge(LR1, "spec", SPEC)];
    for row in &rows {
        edges.push(edge(&row.id, "spec", SPEC));
        edges.push(edge(&row.id, "requirement", LR1));
    }
    let mut entities = vec![
        entity(SPEC, "phase_spec", &[]),
        entity(LR1, "locked_req", &[("spec", SPEC)]),
    ];
    entities.extend(rows);
    PassInput {
        entities,
        edges,
        ..Default::default()
    }
}

fn row(n: u32, fields: &[(&str, &str)]) -> PassEntity {
    entity(&format!("{SPEC}/EC{n}"), "edge", fields)
}

fn messages(found: &[PassDiagnostic]) -> Vec<String> {
    assert!(found.iter().all(|d| d.code == "E523"), "{found:?}");
    found.iter().map(|d| d.message.clone()).collect()
}

#[test]
fn each_refusal_of_the_engine_is_e523() {
    for (fields, needle) in [
        (
            &[("category", "boundary"), ("status", "covered")][..],
            "invalid status \"covered\"",
        ),
        (
            &[
                ("category", "boundary"),
                ("status", "resolved"),
                ("resolution", "x"),
            ][..],
            "resolved requires a verification tier",
        ),
        (
            &[
                ("category", "boundary"),
                ("status", "resolved"),
                ("verification", "explicit"),
            ][..],
            "explicit requires a resolution",
        ),
        (
            &[("category", "boundary"), ("status", "dismissed")][..],
            "dismissed requires a reason",
        ),
        (
            &[
                ("category", "boundary"),
                ("status", "unresolved"),
                ("reason", "why"),
            ][..],
            "unresolved must not carry a reason",
        ),
        (
            &[
                ("category", "boundary"),
                ("status", "dismissed"),
                ("reason", "r"),
                ("verification", "backstop"),
            ][..],
            "verification must be null",
        ),
    ] {
        let found = messages(&check(&input(vec![row(1, fields)])));
        assert_eq!(found.len(), 1, "{fields:?}: {found:?}");
        assert!(found[0].contains(needle), "{found:?}");
        assert!(
            found[0].contains(&format!("{SPEC}/EC1")) && !found[0].contains("::"),
            "{found:?}"
        );
    }
    let prohibition = entity(
        &format!("{SPEC}/SP1"),
        "spec_prohibition",
        &[("status", "resolved"), ("verification", "explicit")],
    );
    let found = messages(&check(&input(vec![prohibition])));
    assert!(
        found[0].contains("invalid verification \"explicit\""),
        "{found:?}"
    );
}

#[test]
fn a_requirement_of_another_spec_is_e523() {
    let other = "v1.0/03/03-SPEC/LR1";
    let mut i = input(vec![]);
    i.entities.push(entity(other, "locked_req", &[]));
    i.edges.push(edge(other, "spec", "v1.0/03/03-SPEC"));
    i.entities
        .push(row(1, &[("category", "empty"), ("status", "unresolved")]));
    i.edges.push(edge(&format!("{SPEC}/EC1"), "spec", SPEC));
    i.edges
        .push(edge(&format!("{SPEC}/EC1"), "requirement", other));
    let found = messages(&check(&i));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("not a locked requirement of its spec"),
        "{found:?}"
    );
}

#[test]
fn a_second_edge_of_a_pair_is_e523_on_the_later_row() {
    let found = check(&input(vec![
        row(10, &[("category", "empty"), ("status", "unresolved")]),
        row(2, &[("category", "empty"), ("status", "unresolved")]),
        row(3, &[("category", "ordering"), ("status", "unresolved")]),
    ]));
    assert_eq!(messages(&found).len(), 1, "{found:?}");
    assert_eq!(
        found[0].entity.as_deref(),
        Some(format!("{SPEC}/EC10").as_str())
    );
    assert!(
        found[0].message.contains("duplicate resolution"),
        "{found:?}"
    );
}

#[test]
fn a_clean_set_of_rows_is_silent() {
    let found = check(&input(vec![
        row(
            1,
            &[
                ("category", "boundary"),
                ("status", "resolved"),
                ("verification", "explicit"),
                ("resolution", "AC#1"),
            ],
        ),
        row(
            2,
            &[
                ("category", "empty"),
                ("status", "dismissed"),
                ("reason", "never empty"),
            ],
        ),
        row(3, &[("category", "unclassified"), ("status", "unresolved")]),
        entity(
            &format!("{SPEC}/SP1"),
            "spec_prohibition",
            &[
                ("status", "resolved"),
                ("verification", "test"),
                ("check_kind", "node_test"),
            ],
        ),
        entity(
            &format!("{SPEC}/SP2"),
            "spec_prohibition",
            &[("status", "dismissed"), ("reason", "n/a")],
        ),
    ]));
    assert!(found.is_empty(), "{found:?}");
}
