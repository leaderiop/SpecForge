use serde_json::json;

use super::*;

#[test]
fn a_commit_is_read_under_its_twelve_hex_name() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [{"id": "0123456789ab", "kind": "commit", "fields": {"subject": "feat(01-01): x"}}],
    }))
    .expect("a pass input");
    let found = commits(&input);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "0123456789ab");
    assert_eq!(found[0].subject, "feat(01-01): x");
}

#[test]
fn a_commit_reads_its_linked_by() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [
            {"id": "0123456789ab", "kind": "commit", "fields": {"subject": "feat(01-01): x", "type": "feat", "linked_by": "Gsd-Task"}},
            {"id": "0123456789ac", "kind": "commit", "fields": {"subject": "docs(01-01): y", "type": "docs"}},
        ],
    }))
    .expect("a pass input");
    let found = commits(&input);
    assert_eq!(found[0].linked_by, Some(LinkedBy::Trailer));
    assert_eq!(found[1].linked_by, None);
    assert_eq!(found[0].kind, Some(CommitType::Feat));
    assert_eq!(found[1].kind, Some(CommitType::Docs));
}

/// The host sends a written list as its text in `fields` (`""` when empty, protocol 1.1.0) and its items in `values`;
/// an unwritten one in neither.
#[test]
fn an_unwritten_items_changed_is_none_and_a_written_empty_one_is_empty() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [
            {"id": "0123456789aa", "kind": "commit", "fields": {"at": "2026-10-28T09:00:00Z"}},
            {"id": "0123456789ab", "kind": "commit", "fields": {"at": "2026-10-28T09:00:01Z", "items_changed": ""},
             "values": {"items_changed": []}},
            {"id": "0123456789ac", "kind": "commit",
             "fields": {"at": "2026-10-28T09:00:02Z", "items_changed": "js:a/x.js::A"},
             "values": {"items_changed": ["js:a/x.js::A"]}},
        ],
    }))
    .expect("a pass input");
    let found: Vec<Option<Vec<String>>> = commits(&input)
        .into_iter()
        .map(|c| c.items_changed)
        .collect();
    assert_eq!(
        found,
        [None, Some(vec![]), Some(vec!["js:a/x.js::A".to_string()])]
    );
}

#[test]
fn a_commit_reads_whether_it_is_waived() {
    let input: PassInput = serde_json::from_value(json!({
        "entities": [
            {"id": "0123456789ab", "kind": "commit",
             "fields": {"at": "2026-10-28T09:00:00Z", "waiver": "reason, by, at"}},
            {"id": "0123456789ac", "kind": "commit", "fields": {"at": "2026-10-28T09:00:01Z"}},
        ],
    }))
    .expect("a pass input");
    let found: Vec<bool> = commits(&input).into_iter().map(|c| c.waived).collect();
    assert_eq!(found, [true, false]);
}

/// A row of the rule's table: a commit's plan, task, type and `linked_by`, and what the rule answers.
type Row = (
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    Option<LinkedBy>,
    Option<Unlinked<'static>>,
);

/// The rule of what gsd/E521 judges, row by row.
#[test]
fn the_rule_of_what_e521_judges() {
    const PLAN: Option<&str> = Some("v/1/1-01");
    const TASK: Option<&str> = Some("v/1/1-01/T1");
    let by_title = Some(Unlinked::ByTitle {
        task: "v/1/1-01/T1",
    });
    let no_task = Some(Unlinked::NoTask { plan: "v/1/1-01" });
    let title = Some(LinkedBy::Title);
    let trailer = Some(LinkedBy::Trailer);
    let mut rows: Vec<Row> = vec![
        // A task found by the subject's title.
        (PLAN, TASK, Some("feat"), title, by_title),
        // A task found by the trailer is linked.
        (PLAN, TASK, Some("feat"), trailer, None),
        // A docs commit is a plan's metadata commit, whatever it names.
        (PLAN, TASK, Some("docs"), title, None),
        (PLAN, None, Some("docs"), None, None),
        // No type, or another word: a task by title is still judged, no task is not.
        (PLAN, None, None, None, None),
        (PLAN, None, Some("wip"), None, None),
        (PLAN, TASK, None, title, by_title),
    ];
    for kind in ["feat", "fix", "test", "refactor", "perf", "style", "chore"] {
        rows.push((PLAN, None, Some(kind), None, no_task));
    }
    // With no plan nothing is judged: the collector records only a commit that names one.
    for (_, task, kind, linked_by, _) in rows.clone() {
        rows.push((None, task, kind, linked_by, None));
    }
    for (plan, task, kind, linked_by, expected) in rows {
        let kind = kind.and_then(CommitType::parse);
        let row = format!("{plan:?} {task:?} {kind:?} {linked_by:?}");
        assert_eq!(unlinked(plan, task, kind, linked_by), expected, "{row}");
        let commit = Commit {
            plan: plan.map(String::from),
            task: task.map(String::from),
            kind,
            linked_by,
            ..Commit::default()
        };
        assert_eq!(commit.unlinked(), expected, "Commit::unlinked {row}");
    }
}
