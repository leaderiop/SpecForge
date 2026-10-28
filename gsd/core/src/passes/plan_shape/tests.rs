use super::*;

fn plan(id: &str, phase: Option<&str>) -> (Vec<PassEntity>, Vec<PassEdge>) {
    let mut fields = std::collections::BTreeMap::new();
    let mut edges = Vec::new();
    if let Some(phase) = phase {
        fields.insert("phase".to_string(), phase.to_string());
        edges.push(PassEdge {
            source: id.into(),
            target: phase.into(),
            label: "phase".into(),
        });
    }
    let entities = vec![PassEntity {
        id: id.into(),
        kind: "plan".into(),
        fields,
        ..Default::default()
    }];
    (entities, edges)
}

fn found(id: &str, phase: Option<&str>) -> Vec<PassDiagnostic> {
    let (entities, edges) = plan(id, phase);
    check(&PassInput {
        entities,
        edges,
        ..Default::default()
    })
}

#[test]
fn a_phase_plan_named_after_another_phase_is_w323() {
    let found = found("v1.0/02/01-01", Some("v1.0/02"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "W323");
    assert_eq!(found[0].entity.as_deref(), Some("v1.0/02/01-01"));
}

#[test]
fn a_phaseless_plan_not_named_by_a_quick_id_is_w323() {
    let found = found("03-01", None);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("quick id"), "{found:?}");
    assert_eq!(found[0].entity.as_deref(), Some("03-01"));
}

#[test]
fn a_quick_id_and_a_phase_plan_are_clean() {
    assert!(found("261028-a1b", None).is_empty());
    assert!(found("v1.0/01/01-12", Some("v1.0/01")).is_empty());
    assert!(found("v1.0/03/03-100", Some("v1.0/03")).is_empty());
}

#[test]
fn phase_02_1_plan_02_1_01_is_clean() {
    assert!(found("v1.0/02.1/02.1-01", Some("v1.0/02.1")).is_empty());
    // `02-01` is not a plan of phase 02.1, and `02.1-1` has one digit.
    assert_eq!(found("v1.0/02.1/02-01", Some("v1.0/02.1")).len(), 1);
    assert_eq!(found("v1.0/02.1/02.1-1", Some("v1.0/02.1")).len(), 1);
}

#[test]
fn a_quick_id_is_six_digits_a_dash_and_three_base_36_digits() {
    for good in ["261028-a1b", "000000-000", "991231-zzz"] {
        assert!(is_quick_id(good), "{good}");
    }
    for bad in [
        "261028-A1B",
        "26102-a1bc",
        "261028-a1",
        "261028_a1b",
        "03-01",
    ] {
        assert!(!is_quick_id(bad), "{bad}");
    }
}
