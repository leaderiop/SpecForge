//! `diff` (ADR 0376 D5): the one range parser both surfaces share, and the read over two snapshots, refused for a
//! revision the repository does not name and for a project in no repository.

use std::sync::Arc;

use specforge_git::testing::Repo;
use specforge_ops::read::diff::{
    DiffField, DiffRange, DiffRequest, DiffSummary, EMPTY_REVISION, KindCount, SYMMETRIC_RANGE,
    diff,
};
use specforge_ops::view::ProjectView;
use specforge_ops::{NOT_A_REPOSITORY, OpErrorKind, UNKNOWN_REVISION};
use specforge_project::testing::TestProject;
use specforge_test_macros::test as specforge_test;

const FIRST: &str = "thing x \"X\" {\n  note \"first\"\n}\n\nthing y \"Y\" {\n}\n";

/// A project of `a.spec` committed as [`FIRST`], with the real git port: the project and the commit.
fn committed() -> (TestProject, String) {
    let project = TestProject::new().files(&[("a.spec", FIRST)]);
    let sha = Repo::init(project.root()).commit_all("first");
    (
        project.with_git(Arc::new(specforge_git::Gix::default())),
        sha,
    )
}

fn request(range: &str) -> DiffRequest {
    DiffRequest {
        range: Some(range.to_string()),
        ..DiffRequest::default()
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_range_is_none_a_revision_or_two() {
    let parsed = |text| DiffRange::parse(text).map_err(|e| e.code.to_string());
    assert_eq!(parsed(None), Ok(DiffRange::HeadToWorkingTree));
    assert_eq!(
        parsed(Some("main")),
        Ok(DiffRange::ToWorkingTree("main".into()))
    );
    assert_eq!(
        parsed(Some("a..b")),
        Ok(DiffRange::Between("a".into(), "b".into()))
    );
    assert_eq!(
        parsed(Some("HEAD~2..HEAD")),
        Ok(DiffRange::Between("HEAD~2".into(), "HEAD".into()))
    );
    for symmetric in ["a...b", "...b", "a..."] {
        assert_eq!(
            parsed(Some(symmetric)),
            Err(SYMMETRIC_RANGE.into()),
            "{symmetric}"
        );
    }
    for empty in ["", "..b", "a..", ".."] {
        assert_eq!(parsed(Some(empty)), Err(EMPTY_REVISION.into()), "{empty:?}");
    }
    let refused = DiffRange::parse(Some("a...b")).unwrap_err();
    assert_eq!(refused.kind, OpErrorKind::InvalidInput);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_clean_tree_changed_nothing_since_head() {
    let (project, sha) = committed();
    let compiled = project.compile_bare();
    let answer = diff(&ProjectView::of(&compiled), &DiffRequest::default()).unwrap();
    assert_eq!(
        (answer.from.name.as_str(), answer.from.key.as_str()),
        ("HEAD", sha.as_str())
    );
    assert_eq!(answer.to.name, "working tree");
    assert!(
        answer.nodes.is_empty() && answer.edges.is_empty(),
        "{answer:?}"
    );
    project.write("a.spec", FIRST.replace("first", "edited"));
    let compiled = project.compile_bare();
    let answer = diff(&ProjectView::of(&compiled), &request(&sha)).unwrap();
    let changed: Vec<(&str, &str, &str)> = answer
        .nodes
        .iter()
        .map(|n| (n.change.as_str(), n.kind.as_str(), n.id.as_str()))
        .collect();
    assert_eq!(changed, [("modified", "thing", "x")]);
    assert_eq!(answer.nodes[0].fields, ["note"]);
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_unknown_revision_is_refused_on_the_range() {
    let (project, _) = committed();
    let compiled = project.compile_bare();
    for range in ["no-such-rev", "HEAD..nope", "--output=x"] {
        let refusal = diff(&ProjectView::of(&compiled), &request(range)).unwrap_err();
        assert_eq!(refusal.subject, Some(DiffField::Range), "{range}");
        assert_eq!(refusal.error.code, UNKNOWN_REVISION, "{range}");
        assert_eq!(refusal.error.kind, OpErrorKind::InvalidInput);
    }
    let refusal = diff(&ProjectView::of(&compiled), &request("a...b")).unwrap_err();
    assert_eq!(refusal.error.code, SYMMETRIC_RANGE);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_project_in_no_repository_is_refused() {
    let project = TestProject::new()
        .files(&[("a.spec", FIRST)])
        .with_git(Arc::new(specforge_git::Gix::default()));
    let compiled = project.compile_bare();
    let refusal = diff(&ProjectView::of(&compiled), &DiffRequest::default()).unwrap_err();
    assert_eq!(refusal.error.code, NOT_A_REPOSITORY);
    assert_eq!(refusal.error.kind, OpErrorKind::PreconditionFailed);
}

/// Two kinds at HEAD (`thing x`, `thing y`, `item a -refs-> x`); the working tree edits `x`, removes `y`, adds
/// `item b -refs-> x` and drops the edge of `a` (`a` is modified: its `refs` field went).
fn two_kinds() -> TestProject {
    let at_head = format!("{FIRST}\nitem a \"A\" {{\n  refs [x]\n}}\n");
    let project = TestProject::new().files(&[("a.spec", at_head.as_str())]);
    Repo::init(project.root()).commit_all("two kinds");
    let project = project.with_git(Arc::new(specforge_git::Gix::default()));
    project.write(
        "a.spec",
        "thing x \"X\" {\n  note \"edited\"\n}\n\nitem a \"A\" {\n}\n\nitem b \"B\" {\n  refs [x]\n}\n",
    );
    project
}

fn kinds(kinds: &[&str]) -> DiffRequest {
    DiffRequest {
        range: None,
        kinds: kinds.iter().map(ToString::to_string).collect(),
    }
}

#[specforge_test(requirement = "FEATURE-142")]
fn the_kinds_named_keep_their_nodes_and_the_edges_with_an_end_of_one() {
    let project = two_kinds();
    let compiled = project.compile_bare();
    let view = ProjectView::of(&compiled);
    let all = diff(&view, &kinds(&[])).unwrap();
    let named = |answer: &specforge_ops::read::diff::DiffAnswer| -> Vec<String> {
        answer
            .nodes
            .iter()
            .map(|n| format!("{} {} {}", n.change, n.kind, n.id))
            .collect()
    };
    assert_eq!(
        named(&all),
        [
            "modified item a",
            "added item b",
            "modified thing x",
            "removed thing y"
        ]
    );
    assert_eq!(all.edges.len(), 2, "{:?}", all.edges);
    let items = diff(&view, &kinds(&["item"])).unwrap();
    assert_eq!(named(&items), ["modified item a", "added item b"]);
    // both edges have an item end: `b -refs-> x` added, `a -refs-> x` removed
    let edges: Vec<(&str, &str)> = items
        .edges
        .iter()
        .map(|e| (e.change.as_str(), e.source.as_str()))
        .collect();
    assert_eq!(edges, [("added", "b"), ("removed", "a")]);
    let things = diff(&view, &kinds(&["thing"])).unwrap();
    assert_eq!(named(&things), ["modified thing x", "removed thing y"]);
    assert_eq!(things.edges.len(), 2, "x is an end of both edges");
    let both = diff(&view, &kinds(&["thing", "item"])).unwrap();
    assert_eq!(both.nodes, all.nodes);
    let unknown = diff(&view, &kinds(&["ghost"])).unwrap();
    assert!(unknown.nodes.is_empty() && unknown.edges.is_empty());
}

#[specforge_test(requirement = "FEATURE-142")]
fn the_summary_counts_what_the_diff_shows_by_change_and_kind() {
    let project = two_kinds();
    let compiled = project.compile_bare();
    let view = ProjectView::of(&compiled);
    let count = |kind: &str, added, removed, modified| KindCount {
        kind: kind.to_string(),
        added,
        removed,
        modified,
    };
    assert_eq!(
        diff(&view, &kinds(&[])).unwrap().summary,
        DiffSummary {
            nodes_added: 1,
            nodes_removed: 1,
            nodes_modified: 2,
            edges_added: 1,
            edges_removed: 1,
            by_kind: vec![count("item", 1, 0, 1), count("thing", 0, 1, 1)],
        }
    );
    let items = diff(&view, &kinds(&["item"])).unwrap().summary;
    assert_eq!(items.by_kind, [count("item", 1, 0, 1)]);
    assert_eq!((items.nodes_added, items.nodes_modified), (1, 1));
    assert_eq!((items.edges_added, items.edges_removed), (1, 1));
    let (project, _) = committed();
    let clean = project.compile_bare();
    let none = diff(&ProjectView::of(&clean), &DiffRequest::default()).unwrap();
    assert_eq!(none.summary, DiffSummary::default());
}
