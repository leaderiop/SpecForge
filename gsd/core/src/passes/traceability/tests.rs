use super::*;

fn entity(id: &str, kind: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
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

fn codes_of(found: &[PassDiagnostic]) -> Vec<(&str, Option<&str>)> {
    found
        .iter()
        .map(|d| (d.code.as_str(), d.entity.as_deref()))
        .collect()
}

/// Plan `v/1/1-01` (in phase `v/1` when `phased`) claiming REQ-01, and its truth TR1 with the given `requirement`.
fn truth(phased: bool, requirement: Option<&str>) -> PassInput {
    let mut edges = vec![
        edge("v/1/1-01", "REQ-01", "requirements"),
        edge("v/1/1-01/TR1", "v/1/1-01", "plan"),
    ];
    if phased {
        edges.push(edge("v/1/1-01", "v/1", "phase"));
    }
    if let Some(r) = requirement {
        edges.push(edge("v/1/1-01/TR1", r, "requirement"));
    }
    PassInput {
        entities: vec![
            entity("v/1", "phase"),
            entity("v/1/1-01", "plan"),
            entity("v/1/1-01/TR1", "truth"),
            entity("REQ-01", "requirement"),
            entity("REQ-02", "requirement"),
        ],
        edges,
        ..Default::default()
    }
}

#[test]
fn a_truth_without_a_requirement_is_e519() {
    let found = check(&truth(true, None));
    assert_eq!(codes_of(&found), [("E519", Some("v/1/1-01/TR1"))]);
    assert_eq!(
        found[0].message,
        "truth 'v/1/1-01/TR1' names no requirement it proves"
    );
    assert!(check(&truth(true, Some("REQ-01"))).is_empty());
}

#[test]
fn a_truth_proving_another_plans_requirement_is_e519() {
    let found = check(&truth(true, Some("REQ-02")));
    assert_eq!(codes_of(&found), [("E519", Some("v/1/1-01/TR1"))]);
    assert_eq!(
        found[0].message,
        "truth 'v/1/1-01/TR1' proves 'REQ-02', which is not a requirement of plan 'v/1/1-01'"
    );
}

#[test]
fn a_quick_plans_truth_is_not_judged_for_e519() {
    assert!(check(&truth(false, None)).is_empty());
    assert!(check(&truth(false, Some("REQ-02"))).is_empty());
}
