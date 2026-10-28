use super::*;

fn requirement(id: &str, tier: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "requirement".into(),
        fields: BTreeMap::from([("tier".to_string(), tier.to_string())]),
        ..Default::default()
    }
}

fn phase(id: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "phase".into(),
        ..Default::default()
    }
}

fn edge(source: &str, target: &str, label: &str) -> PassEdge {
    PassEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

fn input(entities: Vec<PassEntity>, edges: Vec<PassEdge>) -> PassInput {
    PassInput {
        entities,
        edges,
        ..Default::default()
    }
}

fn codes(found: &[PassDiagnostic]) -> Vec<(&str, &str)> {
    found
        .iter()
        .map(|d| (d.code.as_str(), d.entity.as_deref().unwrap_or("")))
        .collect()
}

#[test]
fn unmapped_requirement_is_e101() {
    let found = check(&input(
        vec![requirement("AUTH-01", "v1"), phase("v1.0/01")],
        vec![],
    ));
    assert_eq!(codes(&found), [("E101", "AUTH-01")]);
}

#[test]
fn requirement_in_two_phases_is_e101() {
    let found = check(&input(
        vec![
            requirement("AUTH-01", "v1"),
            phase("v1.0/01"),
            phase("v1.0/02"),
        ],
        vec![
            edge("v1.0/01", "AUTH-01", "PhaseRequirements"),
            edge("v1.0/02", "AUTH-01", "PhaseRequirements"),
        ],
    ));
    assert_eq!(codes(&found), [("E101", "AUTH-01")]);
    assert!(found[0].message.contains("2 phases"));
}

#[test]
fn a_requirement_in_one_phase_and_a_v2_requirement_in_none_are_clean() {
    let found = check(&input(
        vec![
            requirement("AUTH-01", "v1"),
            requirement("AUTH-02", "v2"),
            requirement("AUTH-03", "out_of_scope"),
            phase("v1.0/01"),
        ],
        vec![edge("v1.0/01", "AUTH-01", "PhaseRequirements")],
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn cycle_is_e102() {
    let found = check(&input(
        vec![phase("v1.0/01"), phase("v1.0/02"), phase("v1.0/03")],
        vec![
            edge("v1.0/02", "v1.0/03", "PhaseDependsOn"),
            edge("v1.0/03", "v1.0/02", "PhaseDependsOn"),
        ],
    ));
    let on_cycle: Vec<_> = found
        .iter()
        .filter(|d| d.message.contains("cycle"))
        .map(|d| d.entity.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(on_cycle, ["v1.0/02", "v1.0/03"]);
}

#[test]
fn forward_dependency_is_e102() {
    let found = check(&input(
        vec![phase("v1.0/01"), phase("v1.0/02")],
        vec![edge("v1.0/01", "v1.0/02", "PhaseDependsOn")],
    ));
    assert_eq!(codes(&found), [("E102", "v1.0/01")]);
    assert!(found[0].message.contains("not an earlier phase"));
}

#[test]
fn a_dependency_on_an_earlier_phase_or_an_inserted_one_is_clean() {
    let found = check(&input(
        vec![phase("v1.0/01"), phase("v1.0/01.1"), phase("v1.0/02")],
        vec![
            edge("v1.0/01.1", "v1.0/01", "PhaseDependsOn"),
            edge("v1.0/02", "v1.0/01.1", "PhaseDependsOn"),
        ],
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn gap_in_phase_numbers_is_w104() {
    let found = check(&input(vec![phase("v1.0/01"), phase("v1.0/03")], vec![]));
    assert_eq!(codes(&found), [("W104", "v1.0/03")]);
}

#[test]
fn decimal_phase_between_neighbours_is_clean() {
    let found = check(&input(
        vec![
            phase("v1.0/01"),
            phase("v1.0/01.1"),
            phase("v1.0/01.2"),
            phase("v1.0/02"),
            phase("v1.0/999.1"),
        ],
        vec![],
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_decimal_phase_without_its_integer_or_out_of_order_is_w104() {
    let found = check(&input(
        vec![phase("v1.0/01"), phase("v1.0/02.1"), phase("v1.0/01.3")],
        vec![],
    ));
    assert_eq!(
        codes(&found),
        [("W104", "v1.0/02.1"), ("W104", "v1.0/01.3")]
    );
}

#[test]
fn numbering_is_judged_per_milestone() {
    let found = check(&input(
        vec![
            phase("v1.0/01"),
            phase("v1.0/02"),
            phase("v2.0/03"),
            phase("v2.0/04"),
        ],
        vec![],
    ));
    assert!(found.is_empty(), "{found:?}");
}
