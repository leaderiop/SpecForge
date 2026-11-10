//! What gsd-git asks of the host's git port: one log, one diff per recorded commit, and project-relative paths.

use super::*;

/// Two new commits of the plan and one held: one log, and one diff for each commit recorded, from its first parent,
/// renames apart and with no text asked.
#[test]
fn a_commit_reads_one_log_and_one_diff_per_recorded_commit() {
    let (a, b, c) = ("a".repeat(40), "b".repeat(40), "c".repeat(40));
    let log = vec![
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
    ];
    let mut graph = plan_graph().expect("a graph");
    graph.nodes.push(node(&a[..12], "commit", "plan"));
    let input = CollectInput {
        graph: Some(graph),
        ..Default::default()
    };
    let (output, repo) = served(
        Repo {
            log,
            ..Repo::default()
        },
        &input,
    );
    assert_eq!(created(&output.expect("an answer")).len(), 2);
    assert_eq!(repo.logs.get(), 1);
    let asked = repo.asked.borrow();
    let bases: Vec<_> = asked.iter().map(|q| q.base.clone()).collect();
    assert_eq!(bases, [Some(a.clone()), Some(b.clone())], "oldest first");
    assert!(
        asked
            .iter()
            .all(|q| !q.renames && !q.patch && !q.hunks && q.paths.is_empty())
    );
}

#[test]
fn a_root_commit_counts_every_line_it_adds() {
    let repo = Repo {
        log: vec![logged(HASH, &[], 20, "feat(01-01): Store module", &[])],
        diffs: [(
            HASH.to_string(),
            vec![
                changed("src/a.js", GitChange::Added, 3, 0),
                changed("src/b.js", GitChange::Added, 4, 0),
            ],
        )]
        .into(),
        ..Repo::default()
    };
    let input = CollectInput {
        graph: plan_graph(),
        ..Default::default()
    };
    let (output, repo) = served(repo, &input);
    let made = created(&output.expect("an answer"));
    assert_eq!(
        repo.asked.borrow()[0].base,
        None,
        "a root commit diffs from no tree"
    );
    let fields = &made[0].1;
    assert_eq!(fields["files"], json!(["src/a.js", "src/b.js"]));
    assert_eq!(fields["created"], fields["files"]);
    assert_eq!(
        (&fields["insertions"], &fields["deletions"]),
        (&json!(7), &json!(0))
    );
}

/// The project in `sub/` of its repository: its files are project-relative, one outside it is left out with its
/// lines still counted, and a rename is a deletion and an addition.
#[test]
fn a_project_below_the_repository_top_writes_project_relative_files() {
    let repo = Repo {
        top: "/r".into(),
        log: vec![logged(HASH, &[], 20, "feat(01-01): Store module", &[])],
        diffs: [(
            HASH.to_string(),
            vec![
                changed("other/x.js", GitChange::Modified, 5, 5),
                changed("sub/src/new.js", GitChange::Added, 2, 0),
                changed("sub/src/old.js", GitChange::Deleted, 0, 2),
                changed("sub/src/tasks.js", GitChange::Modified, 1, 1),
                changed("subway/y.js", GitChange::Added, 1, 0),
            ],
        )]
        .into(),
        ..Repo::default()
    };
    let input = CollectInput {
        cwd: Some("/r/sub".into()),
        graph: plan_graph(),
        ..Default::default()
    };
    let made = created(&served(repo, &input).0.expect("an answer"));
    let fields = &made[0].1;
    assert_eq!(
        fields["files"],
        json!(["src/new.js", "src/old.js", "src/tasks.js"])
    );
    assert_eq!(fields["created"], json!(["src/new.js"]));
    assert_eq!(
        (&fields["insertions"], &fields["deletions"]),
        (&json!(9), &json!(8))
    );
}

#[test]
fn the_project_prefix_is_its_root_less_the_repository_top() {
    assert_eq!(prefix(Some("/r"), "/r"), "");
    assert_eq!(prefix(Some("/r/a/b"), "/r"), "a/b");
    assert_eq!(prefix(Some("C:\\r\\sub"), "C:\\r"), "sub");
    assert_eq!(prefix(None, "/r"), "");
    assert_eq!(in_project("sub", "sub/a.js").as_deref(), Some("a.js"));
    assert_eq!(in_project("sub", "subway/a.js"), None);
    assert_eq!(in_project("", "a.js").as_deref(), Some("a.js"));
}
