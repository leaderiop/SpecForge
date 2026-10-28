use super::*;

fn plan(id: &str, wave0: bool) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "plan".into(),
        fields: if wave0 {
            [("wave0".to_string(), "true".to_string())].into()
        } else {
            Default::default()
        },
        ..Default::default()
    }
}

fn checkpoint(id: &str) -> PassEntity {
    PassEntity {
        id: id.into(),
        kind: "checkpoint".into(),
        ..Default::default()
    }
}

/// `(plan, depends on)` pairs.
fn input(plans: &[(&str, bool)], depends: &[(&str, &str)], extra: Vec<PassEntity>) -> PassInput {
    PassInput {
        entities: plans
            .iter()
            .map(|(id, wave0)| plan(id, *wave0))
            .chain(extra)
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

fn waves_of(input: &PassInput) -> Vec<(String, i64)> {
    waves(input)
        .into_iter()
        .map(|(id, wave)| (id.to_string(), wave))
        .collect()
}

#[test]
fn roots_are_wave_1() {
    let found = waves_of(&input(
        &[("v/1/1-01", false), ("v/1/1-02", false)],
        &[],
        vec![],
    ));
    assert_eq!(found, [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 1)]);
}

#[test]
fn a_chain_counts_up() {
    let found = waves_of(&input(
        &[
            ("v/1/1-01", false),
            ("v/1/1-02", false),
            ("v/1/1-03", false),
        ],
        &[("v/1/1-02", "v/1/1-01"), ("v/1/1-03", "v/1/1-02")],
        vec![],
    ));
    assert_eq!(
        found,
        [
            ("v/1/1-01".into(), 1),
            ("v/1/1-02".into(), 2),
            ("v/1/1-03".into(), 3)
        ]
    );
}

#[test]
fn a_diamond_takes_the_longest_path() {
    // A; B and C on A; D on B and C: 1, 2, 2, 3. A shortcut D -> A does not lower D.
    let found = waves_of(&input(
        &[
            ("v/1/a", false),
            ("v/1/b", false),
            ("v/1/c", false),
            ("v/1/d", false),
        ],
        &[
            ("v/1/b", "v/1/a"),
            ("v/1/c", "v/1/a"),
            ("v/1/d", "v/1/b"),
            ("v/1/d", "v/1/c"),
            ("v/1/d", "v/1/a"),
        ],
        vec![],
    ));
    assert_eq!(
        found,
        [
            ("v/1/a".into(), 1),
            ("v/1/b".into(), 2),
            ("v/1/c".into(), 2),
            ("v/1/d".into(), 3)
        ]
    );
}

#[test]
fn a_wave0_plan_starts_the_phase_at_wave_0() {
    let found = waves_of(&input(
        &[("v/1/1-01", true), ("v/1/1-02", false), ("v/1/1-03", false)],
        &[("v/1/1-02", "v/1/1-01")],
        vec![],
    ));
    // The offset is the phase's: its other root is in wave 0 too (`levelOffset` in phase.cts).
    assert_eq!(
        found,
        [
            ("v/1/1-01".into(), 0),
            ("v/1/1-02".into(), 1),
            ("v/1/1-03".into(), 0)
        ]
    );
}

#[test]
fn a_dependency_on_another_phase_is_dropped() {
    let found = waves_of(&input(
        &[("v/1/1-01", false), ("v/2/2-01", false)],
        &[("v/2/2-01", "v/1/1-01")],
        vec![],
    ));
    assert_eq!(found, [("v/1/1-01".into(), 1), ("v/2/2-01".into(), 1)]);
}

#[test]
fn a_cycle_leaves_the_phase_without_waves() {
    let found = waves_of(&input(
        &[
            ("v/1/1-01", false),
            ("v/1/1-02", false),
            ("v/1/1-03", false),
            ("v/2/2-01", false),
        ],
        &[("v/1/1-01", "v/1/1-02"), ("v/1/1-02", "v/1/1-01")],
        vec![],
    ));
    // Not even the plan outside the cycle: GSD errors the whole phase. Phase 2 is unaffected.
    assert_eq!(found, [("v/2/2-01".into(), 1)]);
}

#[test]
fn a_quick_plan_has_no_wave() {
    let input = input(&[("261028-a1b", false), ("v/1/1-01", false)], &[], vec![]);
    assert_eq!(waves_of(&input), [("v/1/1-01".into(), 1)]);
    let values = compute(&input).values;
    assert!(
        values
            .iter()
            .any(|v| v.entity == "261028-a1b" && v.field == "autonomous")
    );
    assert!(
        !values
            .iter()
            .any(|v| v.entity == "261028-a1b" && v.field == "wave")
    );
}

#[test]
fn a_plan_with_a_checkpoint_is_not_autonomous() {
    let input = input(
        &[("v/1/1-01", false), ("v/1/1-02", false)],
        &[],
        vec![checkpoint("v/1/1-02/C1")],
    );
    let autonomous: Vec<(String, bool)> = compute(&input)
        .values
        .iter()
        .filter(|v| v.field == "autonomous")
        .map(|v| (v.entity.clone(), v.value.as_bool().expect("a bool")))
        .collect();
    assert_eq!(
        autonomous,
        [("v/1/1-01".into(), true), ("v/1/1-02".into(), false)]
    );
}
