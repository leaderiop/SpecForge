use super::*;

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

fn summary_input(planned: &[&str], completed: &[&str]) -> PassInput {
    let mut edges = vec![edge("S", "plan", "P")];
    edges.extend(planned.iter().map(|r| edge("P", "requirements", r)));
    edges.extend(
        completed
            .iter()
            .map(|r| edge("S", "requirements_completed", r)),
    );
    PassInput {
        entities: vec![entity("P", "plan", &[]), entity("S", "summary", &[])],
        edges,
        ..Default::default()
    }
}

#[test]
fn e401_names_the_missing_and_the_extra_ids() {
    let found = requirements(&summary_input(&["A-1", "A-2"], &["A-2", "B-1"]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, "E401");
    assert!(
        found[0].message.contains("missing A-1")
            && found[0].message.contains("not in the plan: B-1"),
        "{}",
        found[0].message
    );
    assert_eq!(found[0].entity.as_deref(), Some("S"));
    // Order does not matter, and a quick plan with none on either side is clean.
    assert!(requirements(&summary_input(&["A-1", "A-2"], &["A-2", "A-1"])).is_empty());
    assert!(requirements(&summary_input(&[], &[])).is_empty());
}

fn commit(kind: &str, task: Option<&str>) -> Commit {
    Commit {
        id: format!("P/{kind}0123456789"),
        kind: kind.into(),
        plan: Some("P".into()),
        task: task.map(String::from),
        ..Default::default()
    }
}

fn tdd_input(task_types: &[&str]) -> PassInput {
    let mut entities = vec![entity("P", "plan", &[("type", "execute")])];
    for (i, kind) in task_types.iter().enumerate() {
        entities.push(entity(&format!("P/T{}", i + 1), "task", &[("type", kind)]));
    }
    PassInput {
        entities,
        ..Default::default()
    }
}

#[test]
fn e403_needs_a_red_commit_before_the_green_one() {
    let input = tdd_input(&["tdd"]);
    let green = red_before_green(&input, &[commit("feat", Some("P/T1"))]);
    assert_eq!(green.len(), 1, "{green:?}");
    assert_eq!(green[0].entity.as_deref(), Some("P/T1"));
    assert!(
        green[0].message.contains("GREEN commit 'feat0123456789'"),
        "{}",
        green[0].message
    );
    // RED first is clean; RED after GREEN is not; a task nobody committed is not judged.
    assert!(
        red_before_green(
            &input,
            &[commit("test", Some("P/T1")), commit("feat", Some("P/T1"))]
        )
        .is_empty()
    );
    assert_eq!(
        red_before_green(
            &input,
            &[commit("feat", Some("P/T1")), commit("test", Some("P/T1"))]
        )
        .len(),
        1
    );
    assert!(red_before_green(&input, &[commit("test", Some("P/T1"))]).is_empty());
    assert!(red_before_green(&input, &[]).is_empty());
}

#[test]
fn a_plan_commit_counts_only_for_a_sole_tdd_task() {
    // One tdd task and an auto one: a commit naming only the plan is the tdd task's.
    let sole = tdd_input(&["tdd", "auto"]);
    assert_eq!(red_before_green(&sole, &[commit("feat", None)]).len(), 1);
    // Two tdd tasks: it names neither.
    let two = tdd_input(&["tdd", "tdd"]);
    assert!(red_before_green(&two, &[commit("feat", None)]).is_empty());
    // A task of another kind is not judged at all.
    assert!(red_before_green(&tdd_input(&["auto"]), &[commit("feat", Some("P/T1"))]).is_empty());
}

#[test]
fn a_task_of_a_tdd_plan_is_judged() {
    let mut input = tdd_input(&["auto"]);
    input.entities[0] = entity("P", "plan", &[("type", "tdd")]);
    assert_eq!(
        red_before_green(&input, &[commit("fix", Some("P/T1"))]).len(),
        1
    );
}
