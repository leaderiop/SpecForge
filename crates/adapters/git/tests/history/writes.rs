//! The Git port's writes through `Gix` (ADR 0339 D5): staging is libgit2's, the commit object and its ref move gix's.

use specforge_effects::git::{Git, Staging};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

/// The paths `repo`'s index holds.
fn indexed(repo: &Repo) -> Vec<String> {
    let index = repo.open().index().unwrap();
    index
        .entries()
        .iter()
        .map(|e| e.path(&index).to_string())
        .collect()
}

#[test]
fn staging_a_directory_keeps_its_ignore_rules_and_a_removal_keeps_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write(".gitignore", "*.log\n").write("keep.txt", "k");
    repo.commit_all("first");
    repo.write("spec/a.spec", "a")
        .write("spec/build.log", "noise");
    let place = place(&repo);
    let staging = Staging {
        add: vec!["spec".into()],
        remove: vec!["keep.txt".into()],
        all: false,
    };
    Gix.stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), [".gitignore", "spec/a.spec"]);
    assert!(
        repo.dir().join("keep.txt").is_file(),
        "the file stays on disk"
    );
    let committed = Gix.commit_index(&place, "second\n", false).unwrap();
    assert_eq!(committed.branch.as_deref(), Some("main"));
    assert_eq!(committed.files, ["keep.txt", "spec/a.spec"]);
    assert_eq!(repo.sha("main"), committed.sha.as_str());
}

#[test]
fn staging_all_takes_tracked_changes_and_deletions_and_no_untracked_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a").write("b.txt", "b");
    repo.commit_all("first");
    repo.write("a.txt", "changed")
        .delete("b.txt")
        .write("new.txt", "n");
    let place = place(&repo);
    let staging = Staging {
        all: true,
        ..Staging::default()
    };
    Gix.stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), ["a.txt"]);
    let committed = Gix.commit_index(&place, "tracked\n", false).unwrap();
    assert_eq!(committed.files, ["a.txt", "b.txt"]);
    assert!(
        Gix.commit_index(&place, "again\n", false).is_err(),
        "nothing staged"
    );
}
