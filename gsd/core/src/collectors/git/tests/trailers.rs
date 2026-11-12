//! The `Gsd-Task:` trailer: read from the log's typed trailers, ahead of the subject's title (ADR 0331 D3).

use super::*;

/// Commit `c` of the log with its `Gsd-Task:` values and its subject.
fn trailed(c: char, values: &[&str], subject: &str) -> GitLogCommit {
    logged(&c.to_string().repeat(40), &[], 10, subject, values)
}

/// Each recorded commit's name and fields.
type Made = Vec<(String, BTreeMap<String, Value>)>;

/// What `collect` makes of `log` over [`plan_graph`] with a second plan `01-02` (task T1 "Other"): each recorded
/// commit, and the plan's title.
fn collected(log: Vec<GitLogCommit>) -> (Made, String) {
    let mut graph = plan_graph().expect("a graph");
    graph
        .nodes
        .push(node("v1.0/01/01-02", "plan", "Other plan"));
    graph.nodes.push(node("v1.0/01/01-02/T1", "task", "Other"));
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let output = collected_over(log, input);
    let title = output
        .plan
        .as_ref()
        .and_then(|p| p.title.clone())
        .unwrap_or_default();
    (created(&output), title)
}

fn values(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}

#[test]
fn a_trailers_values_are_read() {
    assert_eq!(trailer(&values(&["01-01/T1"])), Trailer::Named("01-01/T1"));
    assert_eq!(trailer(&[]), Trailer::None);
    // Repeats and blanks count once.
    assert_eq!(
        trailer(&values(&[" 01-01/T1 ", "", "01-01/T1"])),
        Trailer::Named("01-01/T1")
    );
}

#[test]
fn a_lower_case_gsd_task_trailer_is_read() {
    let mut commit = trailed('a', &[], "chore: wire it");
    commit.trailers = vec![
        GitTrailer {
            token: "gsd-task".into(),
            values: values(&["01-01/T2"]),
        },
        GitTrailer {
            token: "Signed-off-by".into(),
            values: values(&["someone"]),
        },
    ];
    let (made, _) = collected(vec![commit]);
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T2");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Trailer.as_str());
}

#[test]
fn a_value_with_a_tab_a_unit_or_record_separator_a_space_or_no_slash_is_unreadable() {
    for value in [
        "01-01/\tT1",
        "01-01/T1\u{1f}x",
        "01-01/T1\u{0}",
        "01 01/T1",
        "01-01",
        "01-01/T1/T2",
        "/T1",
        "01-01/",
        "01-01/T\u{7f}",
        "1\t2\tforged.js",
    ] {
        assert_eq!(trailer(&values(&[value])), Trailer::Unreadable, "{value:?}");
    }
}

#[test]
fn two_different_trailers_name_no_task() {
    assert_eq!(
        trailer(&values(&["01-01/T1", "01-01/T2"])),
        Trailer::Unreadable
    );
    let (made, _) = collected(vec![trailed(
        'a',
        &["01-01/T1", "01-01/T2"],
        "chore: wire it",
    )]);
    assert!(made.is_empty(), "{made:?}");
}

#[test]
fn a_trailer_outweighs_the_title_and_the_scope() {
    // The subject names plan 01-01 and its task T1 by title; the trailer names 01-02's T1.
    let (made, _) = collected(vec![trailed(
        'a',
        &["01-02/T1"],
        "feat(01-01): Store module",
    )]);
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["task"], "v1.0/01/01-02/T1");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Trailer.as_str());
    // No trailer: the title finds the task, and says so.
    let (made, _) = collected(vec![trailed('b', &[], "feat(01-01): Store module")]);
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T1");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Title.as_str());
    // A subject with no `type:` header is recorded by its trailer, with no type.
    let (made, _) = collected(vec![trailed('c', &["01-01/T2"], "wire it")]);
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T2");
    assert!(!made[0].1.contains_key("type"));
}

#[test]
fn a_trailer_naming_no_task_falls_back_to_the_subject_and_is_noted() {
    let (made, title) = collected(vec![
        trailed('a', &["09-09/T9"], "feat(01-01): Store module"),
        trailed('b', &["01-01/T1", "01-02/T1"], "feat(01-01): Another"),
    ]);
    assert_eq!(made.len(), 2);
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T2");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Title.as_str());
    assert_eq!(made[1].1["task"], "v1.0/01/01-01/T1");
    assert!(
        title.contains("(trailer of bbbbbbbbbbbb names no task: unreadable)"),
        "{title}"
    );
    assert!(
        title.contains("(trailer of aaaaaaaaaaaa names no task: 09-09/T9)"),
        "{title}"
    );
    // A commit with no task and no plan in its subject is not recorded: its trailer named nothing either.
    let (made, _) = collected(vec![trailed('c', &["09-09/T9"], "chore: wire it")]);
    assert!(made.is_empty());
}

#[test]
fn a_revert_names_no_task_even_with_a_trailer() {
    let (made, _) = collected(vec![trailed(
        'a',
        &["01-02/T1"],
        "Revert \"feat(01-01): Store module\"",
    )]);
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["type"], "revert");
    assert!(!made[0].1.contains_key("task"));
    assert!(!made[0].1.contains_key("linked_by"));
}

#[test]
fn a_trailers_task_is_recorded_in_the_plan_its_plan_field_names() {
    let mut graph = plan_graph().expect("a graph");
    graph
        .nodes
        .push(node("v1.0/01/01-02", "plan", "Other plan"));
    // The task's `plan` reference names 01-02, though its id is under 01-01: the reference decides.
    let mut task = node("v1.0/01/01-01/T9", "task", "Moved");
    task.fields.insert("plan".into(), json!("v1.0/01/01-02"));
    graph.nodes.push(task);
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collected_over(
        vec![trailed('a', &["01-01/T9"], "chore: wire it")],
        input,
    ));
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T9");
}
