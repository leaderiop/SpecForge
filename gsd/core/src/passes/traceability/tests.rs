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

/// A recorded commit of plan `v/1/1-01` with a subject of `kind`, naming `task` as `linked_by` says.
fn commit(kind: &str, task: Option<&str>, linked_by: Option<&str>) -> PassInput {
    let mut c = entity("0123456789ab", "commit");
    c.fields.insert("type".into(), kind.into());
    let mut edges = vec![edge("0123456789ab", "v/1/1-01", "plan")];
    if let Some(task) = task {
        edges.push(edge("0123456789ab", task, "task"));
    }
    if let Some(by) = linked_by {
        c.fields.insert("linked_by".into(), by.into());
    }
    PassInput {
        entities: vec![entity("v/1/1-01", "plan"), entity("v/1/1-01/T1", "task"), c],
        edges,
        ..Default::default()
    }
}

#[test]
fn a_title_linked_commit_is_e521() {
    let found = check(&commit("feat", Some("v/1/1-01/T1"), Some("title")));
    assert_eq!(codes_of(&found), [("E521", Some("0123456789ab"))]);
    assert_eq!(
        found[0].message,
        "commit '0123456789ab' names task 'v/1/1-01/T1' by its subject's title, not by a Gsd-Task trailer"
    );
    // The trailer the task's later commits add: its full id, or the plan-local form as written; this one stays.
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some(
            "end the message of each later commit of the task with the trailer 'Gsd-Task: v/1/1-01/T1' (its full id; \
             '1-01/T1' reads the same while one plan has that name), then collect; this commit is recorded and keeps \
             its fields: amending it records a new commit beside it, so its own error stays while \
             `specforge.task_trailers` is on"
        )
    );
}

#[test]
fn a_task_type_commit_naming_no_task_is_e521() {
    for kind in ["feat", "fix", "test", "refactor", "perf", "style", "chore"] {
        let found = check(&commit(kind, None, None));
        assert_eq!(codes_of(&found), [("E521", Some("0123456789ab"))], "{kind}");
        assert_eq!(
            found[0].message,
            "commit '0123456789ab' of plan 'v/1/1-01' is a task commit that names no task"
        );
        // The trailer the plan's later task commits add: its full id and one of its tasks, or the plan-local form.
        assert_eq!(
            found[0].suggestion.as_deref(),
            Some(
                "end the message of each later task commit of the plan with the trailer 'Gsd-Task: v/1/1-01/<task>', \
                 naming one of its tasks ('1-01/<task>' reads the same while one plan has that name), then collect; \
                 this commit is recorded and keeps its fields: amending it records a new commit beside it, so its own \
                 error stays while `specforge.task_trailers` is on"
            ),
            "{kind}"
        );
    }
}

#[test]
fn a_trailer_linked_or_docs_commit_is_not_e521() {
    assert!(check(&commit("feat", Some("v/1/1-01/T1"), Some("Gsd-Task"))).is_empty());
    assert!(check(&commit("docs", None, None)).is_empty());
    assert!(check(&commit("", None, None)).is_empty());
    // A docs commit whose subject matches a task's title is a plan's metadata commit too.
    assert!(check(&commit("docs", Some("v/1/1-01/T1"), Some("title"))).is_empty());
}

/// Complete plan `v/1/1-01` planning `src/a.js` and `src/lib` (its artifact's path), deleting `src/old.js`, whose
/// commits touched `touched` (unwritten when `None`); the host served `present` as existing.
fn paths(touched: Option<&[&str]>, present: &[&str]) -> PassInput {
    let mut plan = entity("v/1/1-01", "plan");
    plan.fields.insert("status".into(), "complete".into());
    plan.values
        .insert("files_modified".into(), serde_json::json!(["./src/a.js"]));
    plan.values
        .insert("files_deleted".into(), serde_json::json!(["src/old.js"]));
    if let Some(touched) = touched {
        plan.values
            .insert("files_touched".into(), serde_json::json!(touched));
    }
    let mut artifact = entity("v/1/1-01/A1", "artifact");
    artifact.fields.insert("path".into(), "src/lib/".into());
    let facts: Vec<serde_json::Value> = present
        .iter()
        .map(|p| serde_json::json!({ "path": p, "entity": "v/1/1-01", "exists": true }))
        .collect();
    let mut input: PassInput = serde_json::from_value(serde_json::json!({
        "entities": [],
        "files": { "deleted": { "facts": facts, "truncated": false } },
    }))
    .expect("a pass input");
    input.entities = vec![plan, artifact];
    input
}

#[test]
fn an_untouched_planned_path_is_w520() {
    let found = check(&paths(Some(&["src/a.js"]), &[]));
    assert_eq!(codes_of(&found), [("W520", Some("v/1/1-01"))]);
    assert_eq!(
        found[0].message,
        "plan 'v/1/1-01' is complete, but none of its commits touched: src/lib/"
    );
    let mut quick = paths(Some(&["src/a.js"]), &[]);
    quick.entities[0]
        .fields
        .insert("status".into(), "executing".into());
    assert!(check(&quick).is_empty(), "an incomplete plan is not judged");
}

#[test]
fn a_directory_path_is_touched_by_a_file_under_it() {
    assert!(check(&paths(Some(&["src/a.js", "src/lib/b.js"]), &[])).is_empty());
    let found = check(&paths(Some(&["src/a.js", "src/library.js"]), &[]));
    assert_eq!(codes_of(&found), [("W520", Some("v/1/1-01"))]);
}

#[test]
fn a_deleted_file_still_present_is_w520() {
    let found = check(&paths(Some(&["src/a.js", "src/lib/b.js"]), &["src/old.js"]));
    assert_eq!(codes_of(&found), [("W520", Some("v/1/1-01"))]);
    assert_eq!(
        found[0].message,
        "plan 'v/1/1-01' is complete, but files it deletes are still present: src/old.js"
    );
}

#[test]
fn a_plan_with_no_recorded_commit_is_not_judged_for_w520() {
    assert!(check(&paths(None, &["src/old.js"])).is_empty());
}

#[test]
fn an_unserved_deleted_path_is_not_judged() {
    let mut input = paths(Some(&["src/a.js", "src/lib/b.js"]), &[]);
    input.files.clear();
    assert!(check(&input).is_empty());
}

/// The host serves a deleted path lexically normalised (`src/x/../old.js` as `src/old.js`): the rule finds the fact
/// by its plan and names the path as the host wrote it. Another plan's fact of the same path is not this plan's.
#[test]
fn a_deleted_path_the_host_normalised_is_w520() {
    let mut input = paths(Some(&["src/a.js", "src/lib/b.js"]), &["src/old.js"]);
    input.entities[0].values.insert(
        "files_deleted".into(),
        serde_json::json!(["src/x/../old.js"]),
    );
    let found = check(&input);
    assert_eq!(codes_of(&found), [("W520", Some("v/1/1-01"))]);
    assert_eq!(
        found[0].message,
        "plan 'v/1/1-01' is complete, but files it deletes are still present: src/old.js"
    );
    let deleted = input.files.get_mut("deleted").expect("the deleted input");
    deleted.facts[0].entity = Some("v/1/1-02".into());
    assert!(check(&input).is_empty(), "another plan's fact");
}

const ITEM: &str = "js:app/src/a.js::A";

/// The recorded commit of plan `v/1/1-01` that [`anchored`] holds.
const COMMIT: &str = "0123456789ab";

/// Phase `v/1`, its plan `v/1/1-01` (claiming REQ-01 when `claims`) with its task T1, the quick plan `q/1`, and the
/// recorded commit [`COMMIT`] of `v/1/1-01` that changed [`ITEM`]; REQ-01 is a requirement.
fn anchored(claims: bool) -> PassInput {
    let mut edges = vec![
        edge("v/1/1-01", "v/1", "phase"),
        edge("v/1/1-01/T1", "v/1/1-01", "plan"),
    ];
    if claims {
        edges.push(edge("v/1/1-01", "REQ-01", "requirements"));
    }
    let mut input = PassInput {
        entities: vec![
            entity("v/1", "phase"),
            entity("v/1/1-01", "plan"),
            entity("v/1/1-01/T1", "task"),
            entity("q/1", "plan"),
            entity("REQ-01", "requirement"),
        ],
        edges,
        ..Default::default()
    };
    changing(&mut input, COMMIT, "v/1/1-01");
    input
}

/// A recorded commit `id` of `plan` whose `items_changed` is [`ITEM`].
fn changing(input: &mut PassInput, id: &str, plan: &str) {
    let mut commit = entity(id, "commit");
    // As the host sends a written list: its text in `fields`, its items in `values`.
    commit.fields.insert("items_changed".into(), ITEM.into());
    commit
        .values
        .insert("items_changed".into(), serde_json::json!([ITEM]));
    input.entities.push(commit);
    input.edges.push(edge(id, plan, "plan"));
}

/// Plan `v/1/1-02` of phase `v/1`, claiming nothing.
fn second_plan(input: &mut PassInput) {
    input.entities.push(entity("v/1/1-02", "plan"));
    input.edges.push(edge("v/1/1-02", "v/1", "phase"));
}

#[test]
fn an_item_of_a_plan_with_no_requirement_is_w522() {
    let found = check(&anchored(false));
    assert_eq!(codes_of(&found), [("W522", Some(COMMIT))]);
    assert_eq!(
        found[0].message,
        "item `js:app/src/a.js::A` is anchored only by plan 'v/1/1-01', which reaches no requirement"
    );
    assert_eq!(
        found[0].suggestion.as_deref(),
        Some("name the requirement it delivers in the plan's requirements")
    );
    // A requirement that resolves to no requirement reaches none.
    let mut input = anchored(false);
    input.edges.push(edge("v/1/1-01", "v/1", "requirements"));
    assert_eq!(codes_of(&check(&input)), [("W522", Some(COMMIT))]);
}

#[test]
fn an_item_changed_by_a_commit_of_a_plan_with_a_requirement_is_not_w522() {
    assert!(check(&anchored(true)).is_empty());
}

#[test]
fn only_a_commits_items_changed_anchor_an_item() {
    let mut input = anchored(false);
    // A commit with `items_changed` unwritten anchors nothing.
    let commit = input
        .entities
        .iter_mut()
        .find(|e| e.id == COMMIT)
        .expect("the commit");
    commit.fields.remove("items_changed");
    commit.values.remove("items_changed");
    // Nor does a task's list of items, were one sent.
    let task = input
        .entities
        .iter_mut()
        .find(|e| e.id == "v/1/1-01/T1")
        .expect("the task");
    task.values
        .insert("items".into(), serde_json::json!([ITEM]));
    assert!(check(&input).is_empty());
}

#[test]
fn a_quick_plans_item_is_exempt() {
    let mut input = anchored(false);
    let commit = input
        .edges
        .iter_mut()
        .find(|e| e.source == COMMIT)
        .expect("the commit's plan");
    commit.target = "q/1".into();
    assert!(check(&input).is_empty());
    // A quick plan's commit beside the phase's commit does not lift it.
    let mut input = anchored(false);
    changing(&mut input, "fedcba987654", "q/1");
    assert_eq!(codes_of(&check(&input)), [("W522", Some(COMMIT))]);
}

#[test]
fn the_warning_sits_on_the_first_anchoring_commit_by_id() {
    let mut input = anchored(false);
    second_plan(&mut input);
    changing(&mut input, "00aa00aa00aa", "v/1/1-02");
    let found = check(&input);
    assert_eq!(codes_of(&found), [("W522", Some("00aa00aa00aa"))]);
    assert_eq!(
        found[0].message,
        "item `js:app/src/a.js::A` is anchored only by plans 'v/1/1-01', 'v/1/1-02', which reach no requirement"
    );
}

#[test]
fn an_item_with_one_reaching_anchor_is_not_w522() {
    let mut input = anchored(true);
    second_plan(&mut input);
    changing(&mut input, "00aa00aa00aa", "v/1/1-02");
    assert!(check(&input).is_empty());
}

/// The plan-local form is the id's last segments; a quick plan's id is already its own, so none is added.
#[test]
fn the_plan_local_form_is_the_last_segments_of_the_id() {
    assert_eq!(plan_local("v1.0/01/01-01/T1", 2), Some("01-01/T1"));
    assert_eq!(plan_local("ws/v1.0/01/01-01", 1), Some("01-01"));
    assert_eq!(plan_local("260321-a1b/T1", 2), None);
    assert_eq!(plan_local("260321-a1b", 1), None);
}
