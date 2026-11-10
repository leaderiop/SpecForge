//! A recorded commit's `items_changed`: its diff's lines, carried to the worktree, met with the item facts' spans.

use specforge_extension_sdk::{Fact, FactLocation};

use super::*;

/// The item `id` at 1-based lines `start..=end` of the project-relative `file`.
fn item(id: &str, file: &str, start: usize, end: usize) -> Fact {
    let mut fact = Fact::new("item", id);
    fact.at.push(FactLocation {
        file: file.into(),
        start_line: start,
        start_col: 1,
        end_line: end,
        end_col: 2,
    });
    fact
}

/// A hunk: old lines `before` became new lines `after` (0-based, end exclusive).
fn hunk(before: (u32, u32), after: (u32, u32)) -> GitHunk {
    GitHunk {
        before_start: before.0,
        before_end: before.1,
        after_start: after.0,
        after_end: after.1,
    }
}

/// `path` modified with `hunks`.
fn ranged(path: &str, hunks: Vec<GitHunk>) -> GitFileChange {
    GitFileChange {
        hunks: Some(hunks),
        ..changed(path, GitChange::Modified, 1, 1)
    }
}

/// `A` at lines 1 to 5 of `src/a.js`, `B` at 6 to 10, and a test fact the collector ignores.
fn facts() -> Vec<Fact> {
    vec![
        item("js:app/src/a.js::A", "src/a.js", 1, 5),
        item("js:app/src/a.js::B", "src/a.js", 6, 10),
        Fact::new("test", "js:app/test/a.test.js#t"),
    ]
}

/// The one commit `HASH`, of the plan, whose diff is `file` and whose diff against the worktree is `since`.
fn repo(file: GitFileChange, since: Vec<GitFileChange>) -> Repo {
    Repo {
        log: vec![logged(HASH, &[], 20, "feat(01-01): Store module", &[])],
        diffs: [(HASH.to_string(), vec![file])].into(),
        worktree: [(HASH.to_string(), since)].into(),
        ..Repo::default()
    }
}

/// The `items_changed` of the commit recorded over `repo` with `facts` served.
fn named(repo: Repo, facts: Vec<Fact>) -> Value {
    let input = CollectInput {
        graph: plan_graph(),
        facts,
        ..Default::default()
    };
    let made = created(&served(repo, &input).0.expect("an answer"));
    made[0].1.get("items_changed").cloned().unwrap_or_default()
}

#[test]
fn an_item_holding_a_written_line_is_changed() {
    // Line 7 rewritten: inside B.
    let file = ranged("src/a.js", vec![hunk((6, 7), (6, 7))]);
    assert_eq!(
        named(repo(file, vec![]), facts()),
        json!(["js:app/src/a.js::B"])
    );
    // Two lines added across the edge: both items.
    let file = ranged("src/a.js", vec![hunk((4, 4), (4, 6))]);
    assert_eq!(
        named(repo(file, vec![]), facts()),
        json!(["js:app/src/a.js::A", "js:app/src/a.js::B"])
    );
}

#[test]
fn a_deletion_inside_an_item_changes_it_and_one_at_its_edge_does_not() {
    // Lines deleted before line 4 (inside A), and before line 6 (between A's last line and B's first).
    let file = ranged("src/a.js", vec![hunk((3, 5), (3, 3)), hunk((7, 8), (5, 5))]);
    assert_eq!(
        named(repo(file, vec![]), facts()),
        json!(["js:app/src/a.js::A"])
    );
    // A deletion at the top of the file has no line before it.
    let file = ranged("src/a.js", vec![hunk((0, 2), (0, 0))]);
    assert_eq!(named(repo(file, vec![]), facts()), json!([]));
}

#[test]
fn a_line_rewritten_since_has_no_image() {
    // The commit wrote lines 2 and 7; since, five lines were inserted at the top and line 7 was rewritten.
    let file = ranged("src/a.js", vec![hunk((1, 2), (1, 2)), hunk((6, 7), (6, 7))]);
    let since = vec![ranged(
        "src/a.js",
        vec![hunk((0, 0), (0, 5)), hunk((6, 7), (11, 13))],
    )];
    // Line 2 is now line 7, inside B; line 7 has no image.
    assert_eq!(
        named(repo(file, since), facts()),
        json!(["js:app/src/a.js::B"])
    );
    // A file deleted since keeps no line.
    let file = ranged("src/a.js", vec![hunk((6, 7), (6, 7))]);
    let gone = vec![GitFileChange {
        hunks: Some(vec![hunk((0, 12), (0, 0))]),
        ..changed("src/a.js", GitChange::Deleted, 0, 12)
    }];
    assert_eq!(named(repo(file, gone), facts()), json!([]));
}

#[test]
fn a_project_below_the_repository_top_maps_its_paths() {
    let file = ranged("sub/src/a.js", vec![hunk((6, 7), (6, 7))]);
    let input = CollectInput {
        cwd: Some("/r/sub".into()),
        graph: plan_graph(),
        facts: facts(),
        ..Default::default()
    };
    let (output, repo) = served(repo(file, vec![]), &input);
    let made = created(&output.expect("an answer"));
    assert_eq!(made[0].1["items_changed"], json!(["js:app/src/a.js::B"]));
    let asked = repo.asked.borrow();
    assert_eq!(asked[1].head, GitDiffHead::Worktree);
    assert_eq!(asked[1].paths, ["sub/src/a.js"]);
}

#[test]
fn no_item_facts_leave_items_changed_unwritten() {
    let file = ranged("src/a.js", vec![hunk((6, 7), (6, 7))]);
    let only_tests = vec![Fact::new("test", "js:app/test/a.test.js#t")];
    let input = CollectInput {
        graph: plan_graph(),
        facts: only_tests,
        ..Default::default()
    };
    let (output, repo) = served(repo(file, vec![]), &input);
    let made = created(&output.expect("an answer"));
    assert!(!made[0].1.contains_key("items_changed"), "{:?}", made[0].1);
    let asked = repo.asked.borrow();
    assert_eq!(asked.len(), 1, "no diff against the worktree");
    assert!(!asked[0].hunks, "no line ranges asked");
}

/// The setting op `items_changed` of each commit a plan sets.
fn set(output: &CollectOutput) -> Vec<(String, Value)> {
    output
        .plan
        .iter()
        .flat_map(|plan| &plan.ops)
        .filter_map(|op| match op {
            PlanOp::Set {
                entity,
                field,
                value,
                ..
            } if field == "items_changed" => Some((entity.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_commit_recorded_before_items_gets_them_once() {
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node(&HASH[..12], "commit", "plan"));
    let input = |graph: GraphWire| CollectInput {
        graph: Some(graph),
        facts: facts(),
        ..Default::default()
    };
    let file = || ranged("src/a.js", vec![hunk((6, 7), (6, 7))]);
    let output = served(repo(file(), vec![]), &input(graph.clone()))
        .0
        .expect("an answer");
    assert!(created(&output).is_empty(), "it is held");
    assert_eq!(
        set(&output),
        [(HASH[..12].to_string(), json!(["js:app/src/a.js::B"]))]
    );
    let plan = output.plan.expect("a plan");
    assert_eq!(
        plan.title.as_deref(),
        Some("Name the items 1 recorded commit changed")
    );
    // Once written, never again.
    let last = graph.nodes.last_mut().expect("the commit");
    last.fields
        .insert("items_changed".into(), json!(["js:app/src/a.js::B"]));
    let (output, repo) = served(repo(file(), vec![]), &input(graph));
    assert!(output.expect("an answer").plan.is_none());
    assert!(repo.asked.borrow().is_empty(), "no diff is asked");
}

/// Counted: two diffs per recorded commit with items (its own, and against the worktree), none for a held one.
#[test]
fn two_diffs_per_recorded_commit_with_items() {
    let (a, b, c) = ("a".repeat(40), "b".repeat(40), "c".repeat(40));
    let file = || ranged("src/a.js", vec![hunk((6, 7), (6, 7))]);
    let repo = Repo {
        log: vec![
            logged(
                &c,
                std::slice::from_ref(&b),
                30,
                "feat(01-01): Another",
                &[],
            ),
            logged(
                &b,
                std::slice::from_ref(&a),
                20,
                "feat(01-01): Store module",
                &[],
            ),
            logged(&a, &[], 10, "docs(01-01): plan", &[]),
        ],
        diffs: [(b.clone(), vec![file()]), (c.clone(), vec![file()])].into(),
        ..Repo::default()
    };
    let mut graph = plan_graph().expect("a graph");
    let mut held = node(&a[..12], "commit", "plan");
    held.fields.insert("items_changed".into(), json!([]));
    graph.nodes.push(held);
    let input = CollectInput {
        graph: Some(graph),
        facts: facts(),
        ..Default::default()
    };
    let (output, repo) = served(repo, &input);
    assert_eq!(created(&output.expect("an answer")).len(), 2);
    let heads: Vec<_> = repo.asked.borrow().iter().map(|q| q.head.clone()).collect();
    assert_eq!(
        heads,
        [
            GitDiffHead::Commit(b.clone()),
            GitDiffHead::Worktree,
            GitDiffHead::Commit(c.clone()),
            GitDiffHead::Worktree
        ]
    );
    assert!(repo.asked.borrow().iter().all(|q| q.hunks && !q.patch));
}

/// A commit whose diff is past one answer: the port drops the hunks of the files after its cap.
#[test]
fn a_diff_past_one_answer_writes_items_changed_short() {
    let repo = Repo {
        log: vec![logged(HASH, &[], 20, "feat(01-01): Store module", &[])],
        diffs: [(
            HASH.to_string(),
            vec![
                ranged("src/0.js", vec![hunk((0, 1), (0, 1))]),
                ranged("src/a.js", vec![hunk((6, 7), (6, 7))]),
            ],
        )]
        .into(),
        room: Some(1),
        ..Repo::default()
    };
    // B holds the line the commit wrote, but its file's hunks were dropped: today no item is named.
    assert_eq!(named(repo, facts()), json!([]));
}
