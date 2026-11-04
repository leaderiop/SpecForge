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
    Gix::default().stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), [".gitignore", "spec/a.spec"]);
    assert!(
        repo.dir().join("keep.txt").is_file(),
        "the file stays on disk"
    );
    let committed = Gix::default()
        .commit_index(&place, "second\n", false)
        .unwrap();
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
    Gix::default().stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), ["a.txt"]);
    let committed = Gix::default()
        .commit_index(&place, "tracked\n", false)
        .unwrap();
    assert_eq!(committed.files, ["a.txt", "b.txt"]);
    assert!(
        Gix::default()
            .commit_index(&place, "again\n", false)
            .is_err(),
        "nothing staged"
    );
}

/// git's state files of an operation in progress, written as git writes them.
fn in_progress(repo: &Repo, files: &[&str]) {
    for file in files {
        let at = repo.git_dir().join(file);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, "x\n").unwrap();
    }
}

/// As git: a commit at a rebase's `edit` stop continues the rebase, which this port does not, so it is refused and
/// the rebase's todo list survives; so does an `am` and a bisect.
#[test]
fn a_write_during_a_rebase_am_or_bisect_is_refused_and_keeps_its_state() {
    for state in [
        "rebase-merge/git-rebase-todo",
        "rebase-apply/applying",
        "BISECT_LOG",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repo::init(dir.path());
        repo.write("a.txt", "a");
        repo.commit_all("first");
        repo.write("a.txt", "b");
        in_progress(&repo, &[state]);
        let place = place(&repo);
        let staging = Staging {
            all: true,
            ..Staging::default()
        };
        let refused = Gix::default().stage(&place, &staging).unwrap_err();
        assert!(refused.0.contains("in progress"), "{state}: {refused}");
        let refused = Gix::default()
            .commit_index(&place, "m\n", true)
            .unwrap_err();
        assert!(refused.0.contains("in progress"), "{state}: {refused}");
        assert!(repo.git_dir().join(state).exists(), "{state} survives");
    }
}

/// As git: a stopped `cherry-pick A..B` keeps its `sequencer/`, which a commit here would have ended.
#[test]
fn a_commit_during_a_cherry_pick_of_several_commits_keeps_the_sequencer() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let head = repo.commit_all("first");
    in_progress(&repo, &["sequencer/todo"]);
    std::fs::write(repo.git_dir().join("CHERRY_PICK_HEAD"), format!("{head}\n")).unwrap();
    assert!(
        Gix::default()
            .commit_index(&place(&repo), "m\n", true)
            .is_err()
    );
    assert!(repo.git_dir().join("sequencer/todo").exists());
    assert!(repo.git_dir().join("CHERRY_PICK_HEAD").exists());
}

/// As git ("Cannot do a soft reset in the middle of a merge"): `reset --soft` refuses while `MERGE_HEAD` is there.
#[test]
fn a_soft_reset_in_the_middle_of_a_merge_is_refused() {
    use specforge_effects::git::{ResetMode, Sha};
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.write("a.txt", "b");
    let second = repo.commit_all("second");
    std::fs::write(repo.git_dir().join("MERGE_HEAD"), format!("{first}\n")).unwrap();
    let refused = Gix::default().reset(&place(&repo), &Sha::new(first), ResetMode::Soft);
    assert!(refused.is_err());
    assert_eq!(repo.sha("HEAD"), second);
}

/// A commit that concludes a merge removes the merge's own files and nothing else: a bisect's `BISECT_START` stays.
#[test]
fn a_commit_concluding_a_merge_removes_only_the_merges_files() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.write("a.txt", "b");
    repo.commit_all("second");
    std::fs::write(repo.git_dir().join("MERGE_HEAD"), format!("{first}\n")).unwrap();
    in_progress(&repo, &["MERGE_MSG", "BISECT_START"]);
    let committed = Gix::default().commit_index(&place(&repo), "merged\n", false);
    assert!(committed.is_ok(), "{committed:?}");
    assert!(!repo.git_dir().join("MERGE_HEAD").exists());
    assert!(!repo.git_dir().join("MERGE_MSG").exists());
    assert!(repo.git_dir().join("BISECT_START").exists());
}

/// As git, even with `--force`: a branch a linked worktree has checked out is not deleted.
#[test]
fn a_branch_another_worktree_has_checked_out_is_not_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    let linked = tempfile::tempdir().unwrap();
    repo.linked_worktree(&linked.path().join("wt"), "side", &first);
    let refused = Gix::default().branch_delete(&place(&repo), "side");
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(repo.sha("side"), first);
}
