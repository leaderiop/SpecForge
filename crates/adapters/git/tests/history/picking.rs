//! A cherry-pick or a revert left staged (`-n`), and the end of one in progress (`--quit`, `--abort`), as git
//! leaves the index, the files and the state files.

use specforge_effects::git::{Git, Sha};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

fn read(repo: &Repo, rel: &str) -> Option<String> {
    std::fs::read_to_string(repo.dir().join(rel)).ok()
}

/// `main` at `a.txt` "one", then a commit adding `b.txt` and one changing `a.txt` to "two".
fn history() -> (tempfile::TempDir, Repo, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    repo.commit_all("first");
    repo.write("b.txt", "b\n");
    let added = repo.commit_all("add b");
    repo.write("a.txt", "two\n");
    let changed = repo.commit_all("change a");
    (dir, repo, added, changed)
}

#[test]
fn two_reverts_without_commit_stack_in_the_index_and_keep_revert_head() {
    let (_dir, repo, added, changed) = history();
    let place = place(&repo);
    let head = repo.sha("HEAD");
    for sha in [&changed, &added] {
        let merged = Gix::default()
            .pick(&place, &Sha::new(sha.as_str()), true, false)
            .unwrap();
        assert_eq!((merged.sha, merged.conflicts.len()), (None, 0));
    }
    assert_eq!(repo.sha("HEAD"), head, "nothing is recorded");
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("one\n"));
    assert_eq!(read(&repo, "b.txt"), None, "the added file is gone");
    let status = Gix::default().status(&place).unwrap();
    let staged: Vec<_> = status.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(staged, ["a.txt", "b.txt"]);
    assert!(
        status
            .entries
            .iter()
            .all(|e| e.index.is_some() && e.worktree.is_none())
    );
    let revert_head = std::fs::read_to_string(repo.git_dir().join("REVERT_HEAD")).unwrap();
    assert_eq!(revert_head.trim(), added);
    let message = std::fs::read_to_string(repo.git_dir().join("MERGE_MSG")).unwrap();
    assert!(message.starts_with("Revert \"add b\""), "{message}");
}

#[test]
fn a_cherry_pick_without_commit_leaves_no_cherry_pick_head() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    repo.branch("side", &first).head_on("side");
    repo.write("a.txt", "two\n");
    let side = repo.commit_all("side");
    // back on main, its files with it
    repo.head_on("main").write("a.txt", "one\n");
    let place = place(&repo);
    let merged = Gix::default()
        .pick(&place, &Sha::new(side.as_str()), false, false)
        .unwrap();
    assert_eq!((merged.sha, merged.conflicts.len()), (None, 0));
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("two\n"));
    assert_eq!(repo.sha("HEAD"), first);
    assert!(!repo.git_dir().join("CHERRY_PICK_HEAD").exists());
    let message = std::fs::read_to_string(repo.git_dir().join("MERGE_MSG")).unwrap();
    assert_eq!(message, "side");
}

#[test]
fn abort_puts_back_what_a_revert_staged_and_quit_keeps_it() {
    let (_dir, repo, added, _) = history();
    let place = place(&repo);
    let git = Gix::default();
    assert!(
        git.pick_end(&place, true)
            .unwrap_err()
            .0
            .contains("no cherry-pick or revert")
    );
    assert!(
        !git.pick_end(&place, false).unwrap(),
        "--quit with none is no error"
    );
    git.pick(&place, &Sha::new(added.as_str()), true, false)
        .unwrap();
    repo.write("untracked.txt", "kept\n");
    assert!(git.pick_end(&place, true).unwrap());
    assert_eq!(read(&repo, "b.txt").as_deref(), Some("b\n"));
    assert!(
        git.status(&place)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.path == "untracked.txt")
    );
    assert!(!repo.git_dir().join("REVERT_HEAD").exists());
    git.pick(&place, &Sha::new(added.as_str()), true, false)
        .unwrap();
    assert!(git.pick_end(&place, false).unwrap());
    assert_eq!(read(&repo, "b.txt"), None, "--quit keeps what was staged");
    assert!(!repo.git_dir().join("REVERT_HEAD").exists());
}

#[test]
fn abort_resolves_a_conflict_a_revert_left_back_to_head() {
    let (_dir, repo, _, changed) = history();
    let place = place(&repo);
    repo.write("a.txt", "three\n");
    repo.commit_all("change a again");
    let git = Gix::default();
    let merged = git
        .pick(&place, &Sha::new(changed.as_str()), true, false)
        .unwrap();
    assert_eq!(merged.conflicts.len(), 1, "{merged:?}");
    assert!(read(&repo, "a.txt").unwrap().contains("<<<<<<<"));
    assert!(git.pick_end(&place, true).unwrap());
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("three\n"));
    assert!(git.status(&place).unwrap().entries.is_empty());
}

/// `main` and `side` each one commit past `first`, on different files; `HEAD` on `main`, its files with it.
fn diverged() -> (tempfile::TempDir, Repo, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    repo.branch("side", &first).head_on("side");
    repo.write("s.txt", "side\n");
    let side = repo.commit_all("on side");
    repo.head_on("main")
        .delete("s.txt")
        .write("m.txt", "main\n");
    repo.commit_all("on main");
    (dir, repo, side)
}

#[test]
fn a_squash_stages_the_merge_with_no_second_parent_and_a_commit_ends_it() {
    use specforge_effects::git::{Merging, Recording};
    let (_dir, repo, side) = diverged();
    let place = place(&repo);
    let git = Gix::default();
    let head = repo.sha("HEAD");
    let squash = Merging {
        squash: true,
        ..Merging::default()
    };
    let merged = git
        .merge(&place, &Sha::new(side.as_str()), &squash)
        .unwrap();
    assert_eq!((merged.sha, merged.conflicts.len()), (None, 0));
    assert_eq!(repo.sha("HEAD"), head);
    assert_eq!(read(&repo, "s.txt").as_deref(), Some("side\n"));
    assert!(!repo.git_dir().join("MERGE_HEAD").exists());
    let message = std::fs::read_to_string(repo.git_dir().join("SQUASH_MSG")).unwrap();
    assert!(message.contains(&format!("commit {side}")), "{message}");
    let committed = git
        .commit_index(&place, "squashed\n", &Recording::default())
        .unwrap();
    assert_eq!(committed.parents.len(), 1);
    assert!(
        !repo.git_dir().join("SQUASH_MSG").exists(),
        "the commit ends it"
    );
}

#[test]
fn a_merge_without_commit_keeps_merge_head_for_the_commit_that_records_it() {
    use specforge_effects::git::{Merging, Recording};
    let (_dir, repo, side) = diverged();
    let place = place(&repo);
    let git = Gix::default();
    let no_commit = Merging {
        fast_forward: Some(false),
        no_commit: true,
        ..Merging::default()
    };
    let merged = git
        .merge(&place, &Sha::new(side.as_str()), &no_commit)
        .unwrap();
    assert_eq!(merged.sha, None);
    assert!(repo.git_dir().join("MERGE_HEAD").exists());
    let committed = git
        .commit_index(&place, "Merge side\n", &Recording::default())
        .unwrap();
    assert_eq!(committed.parents.len(), 2);
    assert_eq!(committed.parents[1].as_str(), side);
    assert!(!repo.git_dir().join("MERGE_HEAD").exists());
}
