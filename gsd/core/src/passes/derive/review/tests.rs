use super::*;

const REVIEW: &str = "v1.0/02/RV1";

fn entity(id: &str, kind: &str, fields: &[(&str, &str)]) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        fields: fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
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

/// A concern of the round, `RC<n>`.
fn concern(n: u8, fields: &[(&str, &str)]) -> (PassEntity, PassEdge) {
    let id = format!("{REVIEW}/RC{n}");
    (
        entity(&id, "review_concern", fields),
        edge(&id, "review", REVIEW),
    )
}

/// A disposition of round `round` ruling `outcome` on concern `RC<n>`.
fn disposition(name: &str, n: u8, round: &str, outcome: &str) -> (PassEntity, PassEdge) {
    let id = format!("v1.0/02/02-01/{name}");
    (
        entity(
            &id,
            "disposition",
            &[("round", round), ("outcome", outcome)],
        ),
        edge(&id, "concern", &format!("{REVIEW}/RC{n}")),
    )
}

fn input(items: Vec<(PassEntity, PassEdge)>) -> PassInput {
    let (mut entities, edges): (Vec<_>, Vec<_>) = items.into_iter().unzip();
    entities.push(entity(REVIEW, "review", &[]));
    PassInput {
        entities,
        edges,
        ..Default::default()
    }
}

fn found(values: &[PassValue], entity: &str, field: &str) -> serde_json::Value {
    values
        .iter()
        .find(|v| v.entity == entity && v.field == field)
        .unwrap_or_else(|| panic!("no {entity}.{field}"))
        .value
        .clone()
}

#[test]
fn a_concern_is_open_until_a_disposition_names_it() {
    let high = [("severity", "high")];
    let values = values(&input(vec![
        concern(1, &high),
        concern(2, &high),
        disposition("DSP1", 2, "1", "addressed"),
    ]));
    assert_eq!(found(&values, &format!("{REVIEW}/RC1"), "status"), "open");
    assert_eq!(
        found(&values, &format!("{REVIEW}/RC2"), "status"),
        "addressed"
    );
    assert_eq!(
        found(&values, REVIEW, "open"),
        json!({ "high": 1, "actionable": 0 })
    );
}

#[test]
fn the_highest_round_decides() {
    let values = values(&input(vec![
        concern(1, &[("severity", "medium")]),
        disposition("DSP2", 1, "2", "addressed"),
        disposition("DSP1", 1, "1", "deferred"),
        disposition("DSP3", 1, "10", "deferred"),
    ]));
    assert_eq!(
        found(&values, &format!("{REVIEW}/RC1"), "status"),
        "deferred"
    );
}

#[test]
fn drift_and_non_actionable_concerns_are_not_counted() {
    let values = values(&input(vec![
        concern(1, &[("severity", "medium"), ("actionable", "true")]),
        concern(
            2,
            &[
                ("severity", "low"),
                ("actionable", "true"),
                ("drift", "true"),
            ],
        ),
        concern(3, &[("severity", "low")]),
        concern(4, &[("severity", "high"), ("actionable", "true")]),
    ]));
    assert_eq!(
        found(&values, REVIEW, "open"),
        json!({ "high": 1, "actionable": 1 })
    );
}
