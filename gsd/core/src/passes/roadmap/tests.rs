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

fn project(id: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "project".into(),
        ..Default::default()
    }
}

fn ids<'a>(found: &'a [PassDiagnostic], code: &str) -> Vec<&'a str> {
    found
        .iter()
        .filter(|d| d.code == code)
        .map(|d| d.entity.as_deref().unwrap_or(""))
        .collect()
}

#[test]
fn a_second_project_is_e108() {
    let found = check(&input(
        vec![project("taskboard"), project("alpha"), project("zeta")],
        vec![],
    ));
    assert_eq!(ids(&found, "E108"), ["taskboard", "zeta"]);
    assert!(found[0].message.contains("'alpha'"));
    assert!(check(&input(vec![project("taskboard")], vec![])).is_empty());
    assert!(check(&input(vec![], vec![])).is_empty());
}

#[test]
fn requirements_before_any_phase_are_not_e101() {
    let found = check(&input(vec![requirement("AUTH-01", "v1")], vec![]));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_member_of_a_cycle_is_named_not_only_the_back_edge_path() {
    // X->Y->Z->X and X->W->Z: W is on X->W->Z->X.
    let found = check(&input(
        vec![
            phase("v1.0/05"),
            phase("v1.0/06"),
            phase("v1.0/07"),
            phase("v1.0/08"),
            phase("v1.0/09"),
        ],
        vec![
            edge("v1.0/05", "v1.0/06", "PhaseDependsOn"),
            edge("v1.0/06", "v1.0/07", "PhaseDependsOn"),
            edge("v1.0/07", "v1.0/05", "PhaseDependsOn"),
            edge("v1.0/05", "v1.0/08", "PhaseDependsOn"),
            edge("v1.0/08", "v1.0/07", "PhaseDependsOn"),
            edge("v1.0/09", "v1.0/05", "PhaseDependsOn"),
        ],
    ));
    let on_cycle: Vec<_> = found
        .iter()
        .filter(|d| d.message.contains("cycle"))
        .map(|d| d.entity.as_deref().unwrap_or(""))
        .collect();
    assert_eq!(on_cycle, ["v1.0/05", "v1.0/06", "v1.0/07", "v1.0/08"]);
}

#[test]
fn a_long_dependency_chain_does_not_exhaust_the_stack() {
    let phases: Vec<_> = (1..=5000).map(|n| phase(&format!("v1.0/{n}"))).collect();
    let edges = (2..=5000)
        .map(|n| {
            edge(
                &format!("v1.0/{n}"),
                &format!("v1.0/{}", n - 1),
                "PhaseDependsOn",
            )
        })
        .collect();
    // 999 is the backlog number, so W104 may speak once there; E102 must not.
    assert!(ids(&check(&input(phases, edges)), "E102").is_empty());
}

#[test]
fn a_letter_phase_and_a_nested_decimal_phase_are_numbered() {
    let clean = check(&input(
        vec![
            phase("v1.0/12"),
            phase("v1.0/12A"),
            phase("v1.0/12.1"),
            phase("v1.0/12.1.1"),
            phase("v1.0/13"),
        ],
        vec![],
    ));
    assert!(clean.is_empty(), "{clean:?}");
    let found = check(&input(
        vec![phase("v1.0/12"), phase("v1.0/12.1.1"), phase("v1.0/12.3")],
        vec![],
    ));
    assert_eq!(
        codes(&found),
        [("W104", "v1.0/12.1.1"), ("W104", "v1.0/12.3")]
    );
    assert!(found[0].message.contains("12.1, which does not exist"));
    assert!(found[1].message.contains("skips 12.2"));
}

#[test]
fn a_forward_dependency_between_letter_and_nested_phases_is_e102() {
    let found = check(&input(
        vec![
            phase("v1.0/12A"),
            phase("v1.0/12.1.1"),
            phase("v1.0/12"),
            phase("v1.0/12.1"),
        ],
        vec![
            edge("v1.0/12", "v1.0/12A", "PhaseDependsOn"),
            edge("v1.0/12.1", "v1.0/12.1.1", "PhaseDependsOn"),
        ],
    ));
    assert_eq!(ids(&found, "E102"), ["v1.0/12", "v1.0/12.1"], "{found:?}");
}

#[test]
fn decimal_parts_order_numerically() {
    let ten = Number::of("v1.0/02.10").unwrap();
    let nine = Number::of("v1.0/02.9").unwrap();
    assert!(nine < ten);
    assert!(Number::of("v1.0/12").unwrap() < Number::of("v1.0/12A").unwrap());
    // upstream comparePhaseNum: the letter before the decimals, none before any letter.
    assert!(Number::of("v1.0/12.1").unwrap() < Number::of("v1.0/12A").unwrap());
    assert_eq!(Number::of("v1.0/12a"), Number::of("v1.0/12A"));
    for custom in ["v1.0/setup", "v1.0/1A2", "v1.0/1..2", "v1.0/a", "v1.0/12.A"] {
        assert_eq!(Number::of(custom), None, "{custom}");
    }
    let found = check(&input(
        vec![
            phase("v1.0/02"),
            phase("v1.0/02.1"),
            phase("v1.0/02.9"),
            phase("v1.0/02.10"),
        ],
        vec![edge("v1.0/02.9", "v1.0/02.10", "PhaseDependsOn")],
    ));
    assert_eq!(ids(&found, "E102"), ["v1.0/02.9"]);
}

#[test]
fn a_huge_gap_is_one_warning_and_cheap() {
    let found = check(&input(
        vec![
            phase("v1.0/01"),
            phase("v1.0/20000"),
            phase("v1.0/20260115"),
        ],
        vec![],
    ));
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0].message,
        "phase numbers skip 02-19999, before 'v1.0/20000'"
    );
    assert_eq!(
        found[1].message,
        "phase numbers skip 20001-20260114, before 'v1.0/20260115'"
    );
    let one = check(&input(vec![phase("v1.0/01"), phase("v1.0/03")], vec![]));
    assert_eq!(one[0].message, "phase numbers skip 02, before 'v1.0/03'");
}

#[test]
fn a_milestone_of_only_decimal_phases_is_still_checked() {
    let found = check(&input(vec![phase("v1.0/02.1")], vec![]));
    assert_eq!(codes(&found), [("W104", "v1.0/02.1")]);
}

#[test]
fn two_ids_with_one_number_are_w104() {
    let found = check(&input(vec![phase("v1.0/03"), phase("v1.0/003")], vec![]));
    assert_eq!(codes(&found), [("W104", "v1.0/03")]);
    assert!(found[0].message.contains("same number"));
}

#[test]
fn custom_named_and_backlog_phases_are_outside_the_numbering() {
    let found = check(&input(
        vec![
            phase("v1.0/01"),
            phase("v1.0/setup"),
            phase("v1.0/999.1"),
            phase("v1.0/02"),
        ],
        vec![],
    ));
    assert!(found.is_empty(), "{found:?}");
}

fn phase_with_criteria(id: &str, count: usize) -> PassEntity {
    let items: Vec<String> = (0..count).map(|n| format!("criterion {n}")).collect();
    PassEntity {
        values: BTreeMap::from([("success_criteria".to_string(), serde_json::json!(items))]),
        ..phase(id)
    }
}

#[test]
fn one_criterion_is_w103() {
    let found = check(&input(vec![phase_with_criteria("v1.0/01", 1)], vec![]));
    assert_eq!(codes(&found), [("W103", "v1.0/01")]);
}

#[test]
fn two_and_five_criteria_are_clean() {
    let found = check(&input(
        vec![
            phase_with_criteria("v1.0/01", 2),
            phase_with_criteria("v1.0/02", 5),
            phase("v1.0/03"),
        ],
        vec![],
    ));
    assert_eq!(codes(&found), []);
}

fn scoped(id: &str, kind: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: kind.into(),
        ..Default::default()
    }
}

#[test]
fn a_second_context_in_a_phase_is_e206() {
    let found = check(&input(
        vec![
            scoped("v1.0/02/02-CONTEXT", "context"),
            scoped("v1.0/02/02-CONTEXT-B", "context"),
        ],
        vec![],
    ));
    assert_eq!(codes(&found), [("E206", "v1.0/02/02-CONTEXT-B")]);
    assert!(
        found[0]
            .message
            .contains("a second context of phase 'v1.0/02'; its context is 'v1.0/02/02-CONTEXT'"),
        "{}",
        found[0].message
    );
}

#[test]
fn contexts_of_two_phases_are_clean() {
    let found = check(&input(
        vec![
            scoped("v1.0/01/01-CONTEXT", "context"),
            scoped("v1.0/02/02-CONTEXT", "context"),
            scoped("v1.0/02/02-SPEC", "phase_spec"),
            scoped("v1.0/02/02-DISCUSSION-LOG", "discussion"),
            scoped("v1.0/02/02-RESEARCH", "research"),
            scoped("RESEARCH", "research"),
        ],
        vec![],
    ));
    assert_eq!(codes(&found), []);
}

#[test]
fn a_second_project_research_is_e206() {
    let found = check(&input(
        vec![
            scoped("RESEARCH", "research"),
            scoped("OTHER", "research"),
            scoped("v1.0/02/02-RESEARCH", "research"),
        ],
        vec![],
    ));
    assert_eq!(codes(&found), [("E206", "OTHER")]);
}

#[test]
fn the_research_named_research_is_the_projects_whatever_sorts_first() {
    let found = check(&input(
        vec![
            scoped("03-RESEARCH", "research"),
            scoped("RESEARCH", "research"),
        ],
        vec![],
    ));
    assert_eq!(codes(&found), [("E206", "03-RESEARCH")]);
}

#[test]
fn a_lone_research_without_phase_not_named_research_is_e206() {
    let found = check(&input(vec![scoped("03-RESEARCH", "research")], vec![]));
    assert_eq!(codes(&found), [("E206", "03-RESEARCH")]);
}

#[test]
fn a_quick_plan_holds_one_context_and_one_research_of_any_name() {
    let quick = |id: &str| edge(id, "251101-abc", "plan");
    let clean = check(&input(
        vec![
            scoped("251101-abc", "plan"),
            scoped("251101-abc-CONTEXT", "context"),
            scoped("251101-abc-D-01", "decision"),
            scoped("foo", "research"),
            scoped("251101-xyz-CONTEXT", "context"),
            scoped("RESEARCH", "research"),
        ],
        vec![
            quick("251101-abc-CONTEXT"),
            quick("251101-abc-D-01"),
            quick("foo"),
            edge("251101-xyz-CONTEXT", "251101-xyz", "plan"),
        ],
    ));
    assert_eq!(codes(&clean), []);
    let found = check(&input(
        vec![
            scoped("251101-abc-CONTEXT", "context"),
            scoped("251101-abc-CONTEXT-B", "context"),
        ],
        vec![quick("251101-abc-CONTEXT"), quick("251101-abc-CONTEXT-B")],
    ));
    assert_eq!(codes(&found), [("E206", "251101-abc-CONTEXT-B")]);
    assert!(
        found[0]
            .message
            .contains("a second context of quick plan '251101-abc'"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_context_names_exactly_one_owner_and_a_plan_owner_is_a_quick_plan() {
    let found = check(&input(
        vec![
            scoped("v1.0/01/01-CONTEXT", "context"),
            scoped("v1.0/01/D-01", "decision"),
            scoped("STRAY-CONTEXT", "context"),
            scoped("251101-abc-RESEARCH", "research"),
        ],
        vec![
            edge("v1.0/01/01-CONTEXT", "251101-abc", "plan"),
            edge("v1.0/01/D-01", "251101-abc", "plan"),
            edge("251101-abc-RESEARCH", "v1.0/01/01-01", "plan"),
            edge("v1.0/01/01-01", "v1.0/01", "phase"),
        ],
    ));
    assert_eq!(
        codes(&found),
        [
            ("E206", "v1.0/01/01-CONTEXT"),
            ("E206", "v1.0/01/D-01"),
            ("E206", "STRAY-CONTEXT"),
            ("E206", "251101-abc-RESEARCH"),
        ]
    );
    assert!(found[0].message.contains("names both a phase and a plan"));
    assert!(
        found[2]
            .message
            .contains("names neither a phase nor a quick plan")
    );
    assert!(found[3].message.contains("which is not a quick plan"));
}

#[test]
fn a_quick_plans_research_named_research_is_e206() {
    let found = check(&input(
        vec![scoped("251101-abc", "plan"), scoped("RESEARCH", "research")],
        vec![edge("RESEARCH", "251101-abc", "plan")],
    ));
    assert_eq!(codes(&found), [("E206", "RESEARCH")]);
    assert!(
        found[0]
            .message
            .contains("research 'RESEARCH' names quick plan '251101-abc'"),
        "{}",
        found[0].message
    );
}
