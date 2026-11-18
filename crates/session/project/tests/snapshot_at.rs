//! A project's graph at a git revision (ADR 0376 D1, D2): read through the git port with no checkout, built with
//! the current environment, and keyed by the revision it was named by.

use std::sync::Arc;

use specforge_effects::SharedGit;
use specforge_git::testing::Repo;
use specforge_project::CompiledProject;
use specforge_project::snapshot::{AtError, Snapshot, SnapshotKey};
use specforge_project::testing::TestProject;
use specforge_test::prelude::*;

const FIRST: &str = "thing x \"X\" {\n  note \"first\"\n}\n\nthing y \"Y\" {\n}\n";
const SECOND: &str = "thing x \"X\" {\n  note \"second\"\n}\n\nthing z \"Z\" {\n}\n";

/// The git port a surface installs.
fn gix() -> SharedGit {
    Arc::new(specforge_git::Gix::default())
}

/// A project of `a.spec` committed as [`FIRST`], then as [`SECOND`]: the project, compiled with no extension and
/// the real git port, and the two commits.
fn two_commits() -> (TestProject, CompiledProject, String, String) {
    let project = TestProject::new().files(&[("a.spec", FIRST)]);
    let repo = Repo::init(project.root());
    let first = repo.commit_all("first");
    repo.write("a.spec", SECOND);
    let second = repo.commit_all("second");
    let project = project.with_git(gix());
    let compiled = project.compile_bare();
    (project, compiled, first, second)
}

/// The value `note` of node `id` in `graph`, as the source wrote it.
fn note(graph: &specforge_graph::Graph, id: &str) -> Option<String> {
    let node = graph.node(id)?;
    let entry = node
        .fields
        .entries()
        .iter()
        .find(|e| e.key.as_str() == "note")?;
    Some(format!("{entry:?}"))
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_revision_holds_the_graph_its_commit_wrote() {
    let (_project, compiled, first, _) = two_commits();
    let env = compiled.environment();
    let at = Snapshot::at_revision(env, &first).unwrap();
    assert_eq!(
        at.key,
        SnapshotKey::Revision {
            name: first.clone(),
            sha: specforge_effects::git::Sha::new(first.clone()),
        }
    );
    assert!(at.graph.node("x").is_some(), "x at the first commit");
    assert!(at.graph.node("y").is_some(), "y at the first commit");
    assert!(at.graph.node("z").is_none(), "z is not written yet");
    assert!(note(&at.graph, "x").unwrap().contains("first"));
    // the working tree holds the second commit's
    let now = Snapshot::of_project(&compiled);
    assert!(now.graph.node("y").is_none());
    assert!(now.graph.node("z").is_some());
    assert!(note(&now.graph, "x").unwrap().contains("second"));
    assert!(matches!(now.key, SnapshotKey::WorkingTree { .. }));
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_revision_by_name_resolves_to_its_commit() {
    let (_project, compiled, _, second) = two_commits();
    let at = Snapshot::at_revision(compiled.environment(), "HEAD").unwrap();
    let SnapshotKey::Revision { name, sha } = &at.key else {
        panic!("a revision: {:?}", at.key)
    };
    assert_eq!(name, "HEAD");
    assert_eq!(sha.as_str(), second);
    assert!(at.graph.node("z").is_some());
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_unknown_revision_is_refused() {
    let (_project, compiled, _, _) = two_commits();
    let env = compiled.environment();
    assert_eq!(
        Snapshot::at_revision(env, "no-such-rev").unwrap_err(),
        AtError::UnknownRevision("no-such-rev".to_string())
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_option_like_revision_is_an_unknown_revision() {
    let (_project, compiled, _, _) = two_commits();
    let env = compiled.environment();
    // the git port resolves it in process: an option is a name no commit has, never an argument to a program
    assert_eq!(
        Snapshot::at_revision(env, "--upload-pack=x").unwrap_err(),
        AtError::UnknownRevision("--upload-pack=x".to_string())
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_project_outside_a_repository_has_no_revision() {
    let project = TestProject::new()
        .files(&[("a.spec", FIRST)])
        .with_git(gix());
    let compiled = project.compile_bare();
    assert_eq!(
        Snapshot::at_revision(compiled.environment(), "HEAD").unwrap_err(),
        AtError::NotARepository
    );
}

#[specforge_test(requirement = "FEATURE-142")]
fn no_git_port_is_unreadable() {
    let project = TestProject::new().files(&[("a.spec", FIRST)]);
    Repo::init(project.root()).commit_all("c");
    let compiled = project.compile_bare();
    assert!(matches!(
        Snapshot::at_revision(compiled.environment(), "HEAD"),
        Err(AtError::Unreadable(_))
    ));
}
