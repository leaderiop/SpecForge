//! `diff` (ADR 0376 D5): the one range parser both surfaces share, and the read over two snapshots, refused for a
//! revision the repository does not name and for a project in no repository.

use std::sync::Arc;

use specforge_git::testing::Repo;
use specforge_ops::read::diff::{
    DiffField, DiffRange, DiffRequest, EMPTY_REVISION, SYMMETRIC_RANGE, diff,
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
