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
    graph(
        plans
            .iter()
            .map(|(id, wave0)| plan(id, *wave0))
            .chain(extra)
            .collect(),
        depends,
    )
}

/// The entities, with `(plan, depends on)` pairs as `depends_on` edges.
fn graph(entities: Vec<PassEntity>, depends: &[(&str, &str)]) -> PassInput {
    PassInput {
        entities,
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

/// `entity` with the list of strings `field` set to `paths`.
fn with_paths(mut entity: PassEntity, field: &str, paths: &[&str]) -> PassEntity {
    entity.values.insert(field.into(), serde_json::json!(paths));
    entity
}

/// A plan with no `wave0` that modifies `paths`.
fn modifying(id: &str, paths: &[&str]) -> PassEntity {
    with_paths(plan(id, false), "files_modified", paths)
}

fn task(id: &str, files: &[&str]) -> PassEntity {
    let task = PassEntity {
        id: id.into(),
        kind: "task".into(),
        ..Default::default()
    };
    with_paths(task, "files", files)
}

fn waves_of(input: &PassInput) -> Vec<(String, i64)> {
    waves(input)
        .into_iter()
        .map(|(id, wave)| (id.to_string(), wave))
        .collect()
}

fn strict_of(input: &PassInput) -> Vec<(String, i64)> {
    strict_waves(input)
        .into_iter()
        .map(|(id, wave)| (id.to_string(), wave))
        .collect()
}

/// The wave of each plan that has one, by plan id.
fn waves(input: &PassInput) -> BTreeMap<&str, i64> {
    placed(input).into_iter().flat_map(|p| p.wave).collect()
}

/// The strict wave of each plan that has one, by plan id: exactly the plans that have a wave.
fn strict_waves(input: &PassInput) -> BTreeMap<&str, i64> {
    placed(input).into_iter().flat_map(|p| p.strict).collect()
}

fn paths(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|p| (*p).to_string()).collect()
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

#[test]
fn levels_count_a_chain_from_0_and_place_no_cycle() {
    let ids: BTreeSet<&str> = ["a", "b"].into();
    let chain = BTreeMap::from([("b", BTreeSet::from(["a"]))]);
    assert_eq!(
        levels(&ids, &chain),
        Some(BTreeMap::from([("a", 0), ("b", 1)]))
    );
    let cycle = BTreeMap::from([("a", BTreeSet::from(["b"])), ("b", BTreeSet::from(["a"]))]);
    assert_eq!(levels(&ids, &cycle), None);
}

#[test]
fn two_plans_of_one_wave_sharing_a_path_are_kept_apart() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js", "src/b.js"]),
        ],
        &[],
    );
    assert_eq!(
        waves_of(&input),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 1)]
    );
    assert_eq!(
        strict_of(&input),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 2)]
    );
    let placed = placed(&input);
    assert_eq!(placed.len(), 1);
    assert_eq!(placed[0].phase, "v/1");
    assert_eq!(
        placed[0].shared,
        BTreeMap::from([(("v/1/1-02", "v/1/1-01"), paths(&["src/a.js"]))])
    );
    assert_eq!(
        placed[0].before,
        BTreeMap::from([("v/1/1-02", BTreeSet::from(["v/1/1-01"]))])
    );
    assert!(placed[0].depends.is_empty());
}

#[test]
fn a_plan_that_depends_on_a_pushed_plan_is_pushed_after_it() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js"]),
            modifying("v/1/1-03", &["src/c.js"]),
        ],
        &[("v/1/1-03", "v/1/1-02")],
    );
    assert_eq!(
        waves_of(&input),
        [
            ("v/1/1-01".into(), 1),
            ("v/1/1-02".into(), 1),
            ("v/1/1-03".into(), 2)
        ]
    );
    assert_eq!(
        strict_of(&input),
        [
            ("v/1/1-01".into(), 1),
            ("v/1/1-02".into(), 2),
            ("v/1/1-03".into(), 3)
        ]
    );
    let placed = placed(&input);
    assert_eq!(
        placed[0].shared.keys().collect::<Vec<_>>(),
        [&("v/1/1-02", "v/1/1-01")]
    );
}

#[test]
fn plans_of_different_waves_sharing_a_path_keep_their_waves() {
    // 1-03 is in wave 2 behind 1-02 and shares a path with 1-01 of wave 1, which nothing pushes.
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/b.js"]),
            modifying("v/1/1-03", &["src/a.js"]),
        ],
        &[("v/1/1-03", "v/1/1-02")],
    );
    let expected: Vec<(String, i64)> = vec![
        ("v/1/1-01".into(), 1),
        ("v/1/1-02".into(), 1),
        ("v/1/1-03".into(), 2),
    ];
    assert_eq!(waves_of(&input), expected);
    assert_eq!(strict_of(&input), expected);
    assert_eq!(
        placed(&input)[0].shared,
        BTreeMap::from([(("v/1/1-03", "v/1/1-01"), paths(&["src/a.js"]))])
    );
}

#[test]
fn a_later_wave_plan_sharing_a_path_with_a_pushed_plan_is_pushed() {
    // A and B of wave 1 share x (B is pushed to 2); C of wave 2, on A, shares y with B, so it runs after B.
    let (a, b, c) = ("v/1/1-01", "v/1/1-02", "v/1/1-03");
    let input = graph(
        vec![
            modifying(a, &["src/x.js"]),
            modifying(b, &["src/x.js", "src/y.js"]),
            modifying(c, &["src/y.js"]),
        ],
        &[(c, a)],
    );
    assert_eq!(
        waves_of(&input),
        [(a.into(), 1), (b.into(), 1), (c.into(), 2)]
    );
    assert_eq!(
        strict_of(&input),
        [(a.into(), 1), (b.into(), 2), (c.into(), 3)]
    );
    let placed = placed(&input);
    assert_eq!(
        placed[0].shared,
        BTreeMap::from([
            ((b, a), paths(&["src/x.js"])),
            ((c, b), paths(&["src/y.js"]))
        ])
    );
    assert_eq!(placed[0].before[c], BTreeSet::from([a, b]));
    assert_eq!(placed[0].depends[c], BTreeSet::from([a]));
}

#[test]
fn a_deleted_path_is_shared_like_a_modified_one() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            with_paths(plan("v/1/1-02", false), "files_deleted", &["src/a.js"]),
        ],
        &[],
    );
    assert_eq!(
        strict_of(&input),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 2)]
    );
}

#[test]
fn a_task_file_outside_its_plan_is_shared_too() {
    // gsd/W306 is only a warning, so a task may name a file its plan does not list.
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/b.js"]),
            task("v/1/1-02/T1", &["src/a.js"]),
        ],
        &[],
    );
    assert_eq!(
        strict_of(&input),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 2)]
    );
    assert_eq!(
        placed(&input)[0].shared,
        BTreeMap::from([(("v/1/1-02", "v/1/1-01"), paths(&["src/a.js"]))])
    );
}

#[test]
fn a_wave0_phase_offsets_both_and_a_lone_plan_keeps_its_wave() {
    let input = graph(
        vec![
            with_paths(plan("v/1/1-01", true), "files_modified", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js"]),
            modifying("v/2/2-01", &["src/a.js"]),
        ],
        &[],
    );
    assert_eq!(
        waves_of(&input),
        [
            ("v/1/1-01".into(), 0),
            ("v/1/1-02".into(), 0),
            ("v/2/2-01".into(), 1)
        ]
    );
    // A plan of another phase sharing the path is not compared.
    assert_eq!(
        strict_of(&input),
        [
            ("v/1/1-01".into(), 0),
            ("v/1/1-02".into(), 1),
            ("v/2/2-01".into(), 1)
        ]
    );
}

#[test]
fn a_cycle_leaves_the_phase_without_strict_waves() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js"]),
            modifying("v/2/2-01", &["src/a.js"]),
            modifying("v/2/2-02", &["src/a.js"]),
            modifying("261028-a1b", &["src/a.js"]),
        ],
        &[("v/1/1-01", "v/1/1-02"), ("v/1/1-02", "v/1/1-01")],
    );
    assert_eq!(
        strict_of(&input),
        [("v/2/2-01".into(), 1), ("v/2/2-02".into(), 2)]
    );
    let placed = placed(&input);
    assert_eq!(placed.iter().map(|p| p.phase).collect::<Vec<_>>(), ["v/2"]);
    let values = compute(&input).values;
    assert!(
        !values.iter().any(|v| v.field == "strict_wave"
            && (v.entity.starts_with("v/1/") || v.entity == "261028-a1b"))
    );
}

#[test]
fn a_path_holding_a_comma_is_one_path() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js", "src/b.js"]),
            modifying("v/1/1-02", &["src/a.js,src/b.js"]),
        ],
        &[],
    );
    assert_eq!(
        strict_of(&input),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 1)]
    );
    assert!(placed(&input)[0].shared.is_empty());
}

#[test]
fn an_implied_edge_runs_from_the_earlier_wave_not_the_smaller_id() {
    // 1-01 depends on 1-02 and both share a path: by id the edge would close a cycle, by wave it repeats depends_on.
    // 1-03 of wave 1 shares the path too: it runs after 1-02, and 1-01 after it.
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js"]),
            modifying("v/1/1-03", &["src/a.js"]),
        ],
        &[("v/1/1-01", "v/1/1-02")],
    );
    assert_eq!(
        waves_of(&input),
        [
            ("v/1/1-01".into(), 2),
            ("v/1/1-02".into(), 1),
            ("v/1/1-03".into(), 1)
        ]
    );
    assert_eq!(
        strict_of(&input),
        [
            ("v/1/1-01".into(), 3),
            ("v/1/1-02".into(), 1),
            ("v/1/1-03".into(), 2)
        ]
    );
}

#[test]
fn every_pair_sharing_a_path_is_ordered_without_a_cycle() {
    // Six plans that all modify one path, with dependencies that run against id order: the strict placement exists
    // whenever the waves do, and it orders every plan apart.
    let ids: Vec<String> = (1..=6).map(|n| format!("v/1/1-0{n}")).collect();
    let mut depends = Vec::new();
    for (i, later) in ids.iter().enumerate() {
        for (j, earlier) in ids.iter().enumerate().skip(i + 1) {
            if (i * 7 + j * 3) % 4 == 0 {
                depends.push((later.as_str(), earlier.as_str()));
            }
        }
    }
    let input = graph(
        ids.iter().map(|id| modifying(id, &["src/a.js"])).collect(),
        &depends,
    );
    assert_eq!(waves(&input).len(), 6);
    let mut strict: Vec<i64> = strict_waves(&input).into_values().collect();
    strict.sort_unstable();
    assert_eq!(strict, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn compute_answers_strict_wave_beside_wave() {
    let input = graph(
        vec![
            modifying("v/1/1-01", &["src/a.js"]),
            modifying("v/1/1-02", &["src/a.js"]),
            modifying("261028-a1b", &["src/a.js"]),
        ],
        &[],
    );
    let of = |field: &str| -> Vec<(String, i64)> {
        compute(&input)
            .values
            .iter()
            .filter(|v| v.field == field)
            .map(|v| (v.entity.clone(), v.value.as_i64().expect("an int")))
            .collect()
    };
    assert_eq!(of("wave"), [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 1)]);
    assert_eq!(
        of("strict_wave"),
        [("v/1/1-01".into(), 1), ("v/1/1-02".into(), 2)]
    );
}
