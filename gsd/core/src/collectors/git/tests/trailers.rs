//! The `Gsd-Task:` trailer: read from the header, ahead of the subject's title (ADR 0331 D3).

use super::*;

/// A record of `git log` with [`FORMAT`]: commit `c`, its `Gsd-Task:` trailer values (`%x1e`-joined) and its subject.
fn trailed(c: char, trailers: &str, subject: &str) -> String {
    format!(
        "\u{0}{}\u{1f}\u{1f}10\u{1f}{trailers}\u{1f}{subject}\n",
        c.to_string().repeat(40)
    )
}

/// Each recorded commit's name and fields.
type Made = Vec<(String, BTreeMap<String, Value>)>;

/// What `collect` makes of `log` over [`plan_graph`] with a second plan `01-02` (task T1 "Other"): each recorded
/// commit, and the plan's title.
fn collected(log: String) -> (Made, String) {
    let mut graph = plan_graph().expect("a graph");
    graph
        .nodes
        .push(node("v1.0/01/01-02", "plan", "Other plan"));
    graph.nodes.push(node("v1.0/01/01-02/T1", "task", "Other"));
    let input = CollectInput {
        stdout: Some(log),
        graph: Some(graph),
        ..Default::default()
    };
    let output = collect(&input).expect("an answer");
    let title = output
        .plan
        .as_ref()
        .and_then(|p| p.title.clone())
        .unwrap_or_default();
    (created(&output), title)
}

#[test]
fn a_trailer_in_the_header_is_read() {
    let commits = parse(&trailed('a', "01-01/T1", "chore: wire it"));
    assert_eq!(commits[0].trailers, "01-01/T1");
    assert_eq!(commits[0].subject, "chore: wire it");
    assert_eq!(trailer(&commits[0].trailers), Trailer::Named("01-01/T1"));
    assert_eq!(trailer(""), Trailer::None);
    // Repeats and blanks count once.
    assert_eq!(
        trailer(" 01-01/T1 \u{1e}\u{1e}01-01/T1"),
        Trailer::Named("01-01/T1")
    );
}

#[test]
fn a_folded_trailer_cannot_forge_a_numstat_line() {
    // git unfolds a trailer onto the header line: a value that reads like a numstat row stays in the header.
    let commits = parse(&trailed('a', "1\t2\tforged.js", "chore: wire it"));
    assert!(commits[0].files.is_empty());
    assert_eq!(commits[0].insertions, 0);
    assert_eq!(trailer(&commits[0].trailers), Trailer::Unreadable);
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
    ] {
        assert_eq!(trailer(value), Trailer::Unreadable, "{value:?}");
    }
}

#[test]
fn two_different_trailers_name_no_task() {
    assert_eq!(trailer("01-01/T1\u{1e}01-01/T2"), Trailer::Unreadable);
    let (made, _) = collected(trailed('a', "01-01/T1\u{1e}01-01/T2", "chore: wire it"));
    assert!(made.is_empty(), "{made:?}");
}

#[test]
fn a_trailer_outweighs_the_title_and_the_scope() {
    // The subject names plan 01-01 and its task T1 by title; the trailer names 01-02's T1.
    let (made, _) = collected(trailed('a', "01-02/T1", "feat(01-01): Store module"));
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["task"], "v1.0/01/01-02/T1");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Trailer.as_str());
    // No trailer: the title finds the task, and says so.
    let (made, _) = collected(trailed('b', "", "feat(01-01): Store module"));
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T1");
    assert_eq!(made[0].1["linked_by"], LinkedBy::Title.as_str());
    // A subject with no `type:` header is recorded by its trailer, with no type.
    let (made, _) = collected(trailed('c', "01-01/T2", "wire it"));
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T2");
    assert!(!made[0].1.contains_key("type"));
}

#[test]
fn a_trailer_naming_no_task_falls_back_to_the_subject_and_is_noted() {
    let (made, title) = collected(
        [
            trailed('a', "09-09/T9", "feat(01-01): Store module"),
            trailed('b', "01-01/T1\u{1e}01-02/T1", "feat(01-01): Another"),
        ]
        .concat(),
    );
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
    let (made, _) = collected(trailed('c', "09-09/T9", "chore: wire it"));
    assert!(made.is_empty());
}

#[test]
fn a_revert_names_no_task_even_with_a_trailer() {
    let (made, _) = collected(trailed(
        'a',
        "01-02/T1",
        "Revert \"feat(01-01): Store module\"",
    ));
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["type"], "revert");
    assert!(!made[0].1.contains_key("task"));
    assert!(!made[0].1.contains_key("linked_by"));
}

#[test]
fn a_subject_holding_a_unit_separator_stays_the_subject() {
    let commits = parse(&trailed('a', "", "feat(01-01): a\u{1f}b\u{1f}c"));
    assert_eq!(commits[0].trailers, "");
    assert_eq!(commits[0].subject, "feat(01-01): a\u{fffd}b\u{fffd}c");
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
        stdout: Some(trailed('a', "01-01/T9", "chore: wire it")),
        graph: Some(graph),
        ..Default::default()
    };
    let made = created(&collect(&input).expect("an answer"));
    assert_eq!(made[0].1["plan"], "v1.0/01/01-02");
    assert_eq!(made[0].1["task"], "v1.0/01/01-01/T9");
}
