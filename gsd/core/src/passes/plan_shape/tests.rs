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
    names(&PassInput {
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

fn with_values(mut entity: PassEntity, values: &[(&str, serde_json::Value)]) -> PassEntity {
    for (name, value) in values {
        entity.values.insert(name.to_string(), value.clone());
    }
    entity
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

const PLAN: &str = "v/1/1-01";

/// A plan with a truth and an artifact, so that W309 stays silent, and `more`.
fn input(more: Vec<PassEntity>, edges: Vec<PassEdge>) -> PassInput {
    let mut entities = vec![
        entity(PLAN, "plan", &[("phase", "v/1")]),
        entity("v/1/1-01/TR1", "truth", &[]),
        entity("v/1/1-01/AR1", "artifact", &[]),
    ];
    entities.extend(more);
    let mut all = vec![edge(PLAN, "v/1", "phase")];
    all.extend(edges);
    PassInput {
        entities,
        edges: all,
        ..Default::default()
    }
}

fn task(name: &str, kind: &str, files: bool, verify: bool) -> PassEntity {
    let mut task = entity(&format!("{PLAN}/{name}"), "task", &[("type", kind)]);
    if files {
        task.values
            .insert("files".into(), serde_json::json!(["src/a.js"]));
    }
    if verify {
        task.values.insert(
            "verify".into(),
            serde_json::json!([{ "automated": "npm test", "fails_when": "x" }]),
        );
    }
    task
}

#[test]
fn e303_fires_on_an_auto_task_without_files_or_verify() {
    let found = check(&input(
        vec![
            task("T1", "auto", false, true),
            task("T2", "tdd", true, false),
            task("T3", "tracer", false, false),
        ],
        vec![],
    ));
    assert_eq!(
        codes_of(&found),
        [
            ("E303", Some("v/1/1-01/T1")),
            ("E303", Some("v/1/1-01/T2")),
            ("E303", Some("v/1/1-01/T3"))
        ]
    );
    assert!(
        found[2].message.contains("no files and no a verify"),
        "{found:?}"
    );
}

#[test]
fn e303_is_silent_on_a_manual_task_and_on_any_verify_form() {
    let mut text_only = task("T2", "auto", true, false);
    text_only
        .fields
        .insert("verify_text".into(), "npm test".into());
    let mut human = task("T3", "auto", true, false);
    human.fields.insert("verify_human".into(), "look".into());
    let found = check(&input(
        vec![task("T1", "manual", false, false), text_only, human],
        vec![],
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn w304_is_four_tasks_and_e322_is_five_never_both() {
    let tasks = |n: usize| -> Vec<PassEntity> {
        (1..=n)
            .map(|i| task(&format!("T{i}"), "manual", false, false))
            .collect()
    };
    assert!(check(&input(tasks(3), vec![])).is_empty());
    assert_eq!(
        codes_of(&check(&input(tasks(4), vec![]))),
        [("W304", Some(PLAN))]
    );
    assert_eq!(
        codes_of(&check(&input(tasks(5), vec![]))),
        [("E322", Some(PLAN))]
    );
    // A checkpoint counts, as GSD's task_count does.
    let mut three_and_one = tasks(3);
    three_and_one.push(entity(
        "v/1/1-01/C1",
        "checkpoint",
        &[("kind", "human_verify")],
    ));
    assert_eq!(
        codes_of(&check(&input(three_and_one, vec![]))),
        [("W304", Some(PLAN))]
    );
}

fn decision(options: &[&str]) -> PassEntity {
    with_values(
        entity("v/1/1-01/C1", "checkpoint", &[("kind", "decision")]),
        &[(
            "options",
            serde_json::Value::Array(
                options
                    .iter()
                    .map(|n| serde_json::json!({ "name": n }))
                    .collect(),
            ),
        )],
    )
}

#[test]
fn e307_names_the_available_options() {
    let mut checkpoint = decision(&["a", "b"]);
    checkpoint.fields.insert("auto_select".into(), "c".into());
    let found = check(&input(vec![checkpoint], vec![]));
    assert_eq!(codes_of(&found), [("E307", Some("v/1/1-01/C1"))]);
    assert!(
        found[0]
            .message
            .contains("auto_select=\"c\" does not match any option (available: a, b)"),
        "{found:?}"
    );
}

#[test]
fn e307_is_silent_on_a_matching_auto_select_and_answer() {
    let mut checkpoint = with_values(
        decision(&["a", "b"]),
        &[("answer", serde_json::json!({ "option": "b" }))],
    );
    checkpoint.fields.insert("auto_select".into(), "a".into());
    assert!(check(&input(vec![checkpoint], vec![])).is_empty());
}

#[test]
fn e307_reports_an_answer_outside_the_options_a_duplicate_and_an_empty_auto_select() {
    let mut checkpoint = with_values(
        decision(&["a", "a"]),
        &[("answer", serde_json::json!({ "option": "z" }))],
    );
    checkpoint.fields.insert("auto_select".into(), " ".into());
    let found = check(&input(vec![checkpoint], vec![]));
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found[0].message.contains("two options named 'a'"));
    assert!(found[1].message.contains("auto_select is empty"));
    assert!(found[2].message.contains("answer.option=\"z\""));
}

fn one_way(name: &str, settled: bool) -> PassEntity {
    let mut door = serde_json::json!({ "rating": "one_way" });
    if settled {
        door["settled_by"] = "D-01".into();
    }
    with_values(
        task(name, "manual", false, false),
        &[("reversibility", door)],
    )
}

#[test]
fn w308_fires_on_a_one_way_task_with_no_decision_before_it() {
    let found = check(&input(vec![one_way("T1", false)], vec![]));
    assert_eq!(codes_of(&found), [("W308", Some("v/1/1-01/T1"))]);
    assert!(
        found[0]
            .message
            .contains("has no preceding checkpoint:decision"),
        "{found:?}"
    );
}

#[test]
fn w308_is_silent_when_settled_or_after_a_decision_checkpoint() {
    assert!(check(&input(vec![one_way("T1", true)], vec![])).is_empty());
    // A decision "after T1" precedes T2 but not T1.
    let after_t1 = || {
        input(
            vec![one_way("T1", false), one_way("T2", false), decision(&["a"])],
            vec![edge("v/1/1-01/C1", "v/1/1-01/T1", "after")],
        )
    };
    let found = check(&after_t1());
    assert_eq!(codes_of(&found), [("W308", Some("v/1/1-01/T1"))]);
    // With no `after` it stands before the first task.
    let before_all = input(vec![one_way("T1", false), decision(&["a"])], vec![]);
    assert!(check(&before_all).is_empty());
}

#[test]
fn w309_needs_both_a_truth_and_an_artifact() {
    let bare = PassInput {
        entities: vec![entity(PLAN, "plan", &[("phase", "v/1")])],
        edges: vec![edge(PLAN, "v/1", "phase")],
        ..Default::default()
    };
    let found = check(&bare);
    assert_eq!(codes_of(&found), [("W309", Some(PLAN))]);
    assert!(
        found[0].message.contains("no truth and no artifact"),
        "{found:?}"
    );
    assert!(check(&input(vec![], vec![])).is_empty());
}
