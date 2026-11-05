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

/// `a.txt` "one", then "two" (`c2`), then "three" (`c3`): two picks of one file.
fn one_file() -> (tempfile::TempDir, Repo, String, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    repo.write("a.txt", "two\n");
    let c2 = repo.commit_all("two");
    repo.write("a.txt", "three\n");
    let c3 = repo.commit_all("three");
    (dir, repo, first, c2, c3)
}

#[test]
fn two_reverts_without_commit_of_one_file_stack_as_git_stacks_them() {
    let (_dir, repo, _, c2, c3) = one_file();
    let place = place(&repo);
    for sha in [&c3, &c2] {
        let merged = Gix::default()
            .pick(&place, &Sha::new(sha.as_str()), true, false)
            .unwrap();
        assert_eq!((merged.sha, merged.conflicts.len()), (None, 0));
    }
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("one\n"));
    let status = Gix::default().status(&place).unwrap();
    assert_eq!(status.entries.len(), 1, "{status:?}");
    assert!(status.entries[0].index.is_some() && status.entries[0].worktree.is_none());
}

#[test]
fn two_cherry_picks_without_commit_of_one_file_stack_as_git_stacks_them() {
    let (_dir, repo, first, c2, c3) = one_file();
    repo.branch("old", &first)
        .head_on("old")
        .write("a.txt", "one\n");
    repo.stage_all();
    let place = place(&repo);
    for sha in [&c2, &c3] {
        let merged = Gix::default()
            .pick(&place, &Sha::new(sha.as_str()), false, false)
            .unwrap();
        assert_eq!((merged.sha, merged.conflicts.len()), (None, 0));
    }
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("three\n"));
    let status = Gix::default().status(&place).unwrap();
    assert_eq!(status.entries.len(), 1, "{status:?}");
    assert!(status.entries[0].index.is_some() && status.entries[0].worktree.is_none());
}

#[test]
fn a_pick_writes_the_files_as_the_repositorys_autocrlf_says_and_status_reads_them_back_clean() {
    let (_dir, repo, _, _, c3) = one_file();
    repo.config("[core]\n\tautocrlf = true");
    let place = place(&repo);
    Gix::default()
        .pick(&place, &Sha::new(c3.as_str()), true, false)
        .unwrap();
    assert_eq!(
        read(&repo, "a.txt").as_deref(),
        Some("two\r\n"),
        "as git checks it out"
    );
    let status = Gix::default().status(&place).unwrap();
    assert_eq!(status.entries.len(), 1, "{status:?}");
    assert!(
        status.entries[0].index.is_some() && status.entries[0].worktree.is_none(),
        "gix reads the file through the same autocrlf: {status:?}"
    );
}

#[test]
fn a_file_one_pick_added_and_the_next_removed_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    repo.write("x.txt", "x\n");
    let added = repo.commit_all("add x");
    repo.delete("x.txt");
    let removed = repo.commit_all("drop x");
    repo.branch("old", &first).head_on("old");
    repo.stage_all();
    let place = place(&repo);
    for sha in [&added, &removed] {
        Gix::default()
            .pick(&place, &Sha::new(sha.as_str()), false, false)
            .unwrap();
    }
    assert_eq!(read(&repo, "x.txt"), None);
    assert!(Gix::default().status(&place).unwrap().entries.is_empty());
}

#[test]
fn a_pick_without_commit_refuses_to_overwrite_an_unstaged_change() {
    let (_dir, repo, _, c2, c3) = one_file();
    let place = place(&repo);
    let git = Gix::default();
    git.pick(&place, &Sha::new(c3.as_str()), true, false)
        .unwrap();
    repo.write("a.txt", "mine\n");
    let refused = git
        .pick(&place, &Sha::new(c2.as_str()), true, false)
        .unwrap_err();
    assert!(refused.0.contains("a.txt"), "{refused:?}");
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("mine\n"));
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
    repo.stage_all();
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
fn abort_refuses_when_a_path_it_puts_back_has_unstaged_changes() {
    let (_dir, repo, _, changed) = history();
    let place = place(&repo);
    let git = Gix::default();
    git.pick(&place, &Sha::new(changed.as_str()), true, false)
        .unwrap();
    repo.write("a.txt", "mine\n");
    let refused = git.pick_end(&place, true).unwrap_err();
    assert!(refused.0.contains("a.txt"), "{refused:?}");
    assert_eq!(read(&repo, "a.txt").as_deref(), Some("mine\n"), "kept");
    assert!(
        repo.git_dir().join("REVERT_HEAD").exists(),
        "still in progress"
    );
}

#[test]
fn abort_refuses_a_sequence_git_started_and_names_the_commands_that_end_it() {
    let (_dir, repo, _, changed) = history();
    let place = place(&repo);
    let git = Gix::default();
    git.pick(&place, &Sha::new(changed.as_str()), true, false)
        .unwrap();
    std::fs::create_dir_all(repo.git_dir().join("sequencer")).unwrap();
    let refused = git.pick_end(&place, true).unwrap_err();
    assert!(
        refused.0.contains("specforge git reset --hard HEAD"),
        "{refused:?}"
    );
    assert!(repo.git_dir().join("sequencer").is_dir(), "left as it was");
    assert!(git.pick_end(&place, false).unwrap(), "--quit ends it");
    assert!(!repo.git_dir().join("sequencer").exists());
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

#[test]
fn a_worktree_is_locked_and_unlocked_once_each() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    let elsewhere = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&elsewhere.path().join("wt"), "wt", &first);
    let place = place(&repo);
    let git = Gix::default();
    git.worktree_lock(&place, linked.dir(), true, Some("busy"))
        .unwrap();
    let reason = std::fs::read_to_string(repo.git_dir().join("worktrees/wt/locked")).unwrap();
    assert_eq!(reason, "busy");
    assert!(git.worktrees(&place).unwrap().iter().any(|w| w.locked));
    assert!(git.worktree_lock(&place, linked.dir(), true, None).is_err());
    git.worktree_lock(&place, linked.dir(), false, None)
        .unwrap();
    assert!(git.worktrees(&place).unwrap().iter().all(|w| !w.locked));
    assert!(
        git.worktree_lock(&place, linked.dir(), false, None)
            .unwrap_err()
            .0
            .contains("not locked")
    );
}

#[cfg(unix)]
#[test]
fn a_worktree_named_through_a_link_is_the_same_worktree() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    let elsewhere = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&elsewhere.path().join("wt"), "wt", &first);
    let link = elsewhere.path().join("link");
    std::os::unix::fs::symlink(linked.dir(), &link).unwrap();
    let git = Gix::default();
    git.worktree_lock(&place(&repo), &link, true, None).unwrap();
    assert!(
        git.worktrees(&place(&repo))
            .unwrap()
            .iter()
            .any(|w| w.locked)
    );
}

#[test]
fn init_makes_a_repository_once() {
    use specforge_effects::git::Located;
    let dir = tempfile::tempdir().unwrap();
    let git = Gix::default();
    assert!(git.init(dir.path()).unwrap());
    assert!(
        !git.init(dir.path()).unwrap(),
        "one is there: left as it is"
    );
    let Located::Repo(place) = git.locate(dir.path()) else {
        panic!("a repository");
    };
    assert_eq!(git.head(&place).unwrap().head.as_deref(), Some("main"));
    let missing = dir.path().join("a/b");
    assert!(git.init(&missing).unwrap(), "a missing directory is made");
    assert!(missing.join(".git").is_dir());
}
