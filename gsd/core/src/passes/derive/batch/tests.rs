use std::collections::BTreeMap;

use super::*;

const BATCH: &str = "261112-batch";

fn entity(id: &str, kind: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
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

/// A batch of `items` (quick plan ids), the plans depending as `depends` says.
fn input(items: &[&str], depends: &[(&str, &str)]) -> PassInput {
    let mut entities = vec![entity(BATCH, "quick_batch")];
    entities.extend(items.iter().map(|i| entity(i, "plan")));
    let mut edges: Vec<PassEdge> = items.iter().map(|i| edge(BATCH, "items", i)).collect();
    edges.extend(depends.iter().map(|(p, d)| edge(p, "depends_on", d)));
    PassInput {
        entities,
        edges,
        ..Default::default()
    }
}

fn answer(input: &PassInput, statuses: &[(&'static str, &'static str)]) -> (String, Vec<String>) {
    let statuses: BTreeMap<&str, &'static str> = statuses.iter().copied().collect();
    let values = values(input, &statuses);
    let field = |name: &str| {
        values
            .iter()
            .find(|v| v.entity == BATCH && v.field == name)
            .unwrap_or_else(|| panic!("no {name}"))
            .value
            .clone()
    };
    let eligible = field("eligible")
        .as_array()
        .expect("a list")
        .iter()
        .map(|v| v.as_str().expect("an id").to_string())
        .collect();
    (
        field("status").as_str().expect("a word").to_string(),
        eligible,
    )
}

#[test]
fn an_item_is_eligible_once_its_dependencies_are_complete() {
    let input = input(
        &["261112-aaa", "261112-bbb"],
        &[("261112-bbb", "261112-aaa")],
    );
    let (_, eligible) = answer(&input, &[]);
    assert_eq!(eligible, ["261112-aaa"]);
    let (_, eligible) = answer(&input, &[("261112-aaa", "complete")]);
    assert_eq!(eligible, ["261112-bbb"]);
    let (_, eligible) = answer(&input, &[("261112-aaa", "executing")]);
    assert!(eligible.is_empty());
}

#[test]
fn the_batch_is_complete_when_every_item_is() {
    let input = input(&["261112-aaa", "261112-bbb"], &[]);
    assert_eq!(answer(&input, &[("261112-aaa", "complete")]).0, "open");
    let both = [("261112-aaa", "complete"), ("261112-bbb", "complete")];
    assert_eq!(answer(&input, &both).0, "complete");
}

#[test]
fn eligible_is_empty_not_absent() {
    let input = input(&["261112-aaa"], &[]);
    let (status, eligible) = answer(&input, &[("261112-aaa", "halted")]);
    assert_eq!(status, "open");
    assert!(eligible.is_empty());
}
