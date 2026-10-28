use super::*;

fn input(plans: &[&str], depends: &[(&str, &str)]) -> PassInput {
    PassInput {
        entities: plans
            .iter()
            .map(|id| PassEntity {
                id: (*id).into(),
                kind: "plan".into(),
                ..Default::default()
            })
            .collect(),
        edges: depends
            .iter()
            .map(|(source, target)| PassEdge {
                source: (*source).into(),
                target: (*target).into(),
                label: "depends_on".into(),
            })
            .collect(),
        ..Default::default()
    }
}

fn entities(found: &[PassDiagnostic]) -> Vec<&str> {
    found.iter().filter_map(|d| d.entity.as_deref()).collect()
}

#[test]
fn a_dependency_in_the_phase_is_clean() {
    let found = check(&input(
        &["v/1/1-01", "v/1/1-02"],
        &[("v/1/1-02", "v/1/1-01")],
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_dependency_on_another_phase_is_e102() {
    let found = check(&input(
        &["v/1/1-02", "v/2/2-01"],
        &[("v/2/2-01", "v/1/1-02")],
    ));
    assert_eq!(entities(&found), ["v/2/2-01"]);
    assert!(
        found[0].message.contains("plan 'v/2/2-01' depends on 'v/1/1-02', which is not a plan of its phase: GSD drops the edge"),
        "{found:?}"
    );
}

#[test]
fn a_cycle_names_each_plan_on_it_and_only_those() {
    let found = check(&input(
        &["v/1/1-01", "v/1/1-02", "v/1/1-03"],
        &[
            ("v/1/1-01", "v/1/1-02"),
            ("v/1/1-02", "v/1/1-01"),
            ("v/1/1-03", "v/1/1-01"),
        ],
    ));
    assert_eq!(entities(&found), ["v/1/1-01", "v/1/1-02"]);
}

#[test]
fn a_quick_plan_is_checked_for_cycles_only() {
    let clean = check(&input(
        &["261028-a1b", "v/1/1-01"],
        &[("261028-a1b", "v/1/1-01")],
    ));
    assert!(clean.is_empty(), "{clean:?}");
    let found = check(&input(&["261028-a1b"], &[("261028-a1b", "261028-a1b")]));
    assert_eq!(entities(&found), ["261028-a1b"]);
}
