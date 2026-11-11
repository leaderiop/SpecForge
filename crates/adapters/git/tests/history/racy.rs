//! git's racy-clean rule across an index write: a file changed in the second its entry was staged keeps its stat (size,
//! seconds), so only the index's own timestamp tells a reader to compare its content. An index written in a later
//! second must smudge such an entry (size 0), as git does, or every reader sees the file unchanged. That holds for the
//! index libgit2 writes itself (a checkout, a reset, a merge) as for the one staging writes.

use std::fs::File;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use specforge_effects::git::{
    Git, GitChange, Merging, Removal, ResetMode, Restoring, Sha, Staging,
};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

/// A second long past, so every write below lands after it.
fn second() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

/// `rel`'s (a file under `repo`, or the index when `None`) modification time set to `nanos` into [`second`].
fn stamp(repo: &Repo, rel: Option<&str>, nanos: u64) {
    let path = rel.map_or_else(|| repo.git_dir().join("index"), |rel| repo.dir().join(rel));
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(second() + Duration::from_nanos(nanos))
        .unwrap();
}

/// `a.txt` staged at 100 ns into [`second`], the index written at 500 ns, then `a.txt` rewritten to the same size at
/// 900 ns: its stat matches its entry's to the second, and only the index's timestamp makes it racy. `c.txt` is
/// tracked, `b.txt` new.
fn raced() -> (tempfile::TempDir, Repo) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    // ctime cannot be set: left out of the stat, it cannot tell the rewrite either
    repo.config("[core]\n\ttrustctime = false");
    repo.write("a.txt", "k\n").write("c.txt", "c\n");
    stamp(&repo, Some("a.txt"), 100);
    repo.commit_all("first");
    stamp(&repo, None, 500);
    repo.write("a.txt", "l\n").write("b.txt", "b\n");
    stamp(&repo, Some("a.txt"), 900);
    (dir, repo)
}

/// The worktree change the status reads for `path`.
fn worktree_change(repo: &Repo, path: &str) -> Option<GitChange> {
    let status = Gix::default().status(&place(repo)).unwrap();
    status
        .entries
        .into_iter()
        .find(|e| e.path == path)
        .and_then(|e| e.worktree)
}

#[test]
fn staging_another_file_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
    let staging = Staging {
        add: vec!["b.txt".into()],
        ..Staging::default()
    };
    Gix::default().stage(&place(&repo), &staging).unwrap();
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}

#[test]
fn removing_another_file_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    let removal = Removal {
        cached: true,
        ..Removal::default()
    };
    Gix::default()
        .remove(&place(&repo), &["c.txt".into()], removal)
        .unwrap();
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}

#[test]
fn an_unchanged_racy_entry_is_not_smudged() {
    let (_dir, repo) = raced();
    repo.write("a.txt", "k\n");
    stamp(&repo, Some("a.txt"), 100);
    let staging = Staging {
        add: vec!["b.txt".into()],
        ..Staging::default()
    };
    Gix::default().stage(&place(&repo), &staging).unwrap();
    let index = repo.open().index().unwrap();
    let entry = index.entry_by_path("a.txt".into()).unwrap();
    assert_eq!(entry.stat.size, 2, "only a changed file is smudged");
    assert_eq!(worktree_change(&repo, "a.txt"), None);
}

/// `HEAD`'s commit with `d.txt` added, on the branch `side`; the worktree and the index are left as they are.
fn side(repo: &Repo) -> String {
    let git = repo.open();
    let head = git.head_commit().unwrap();
    let blob = git.write_blob("d\n").unwrap().detach();
    let mut tree = git.edit_tree(head.tree_id().unwrap()).unwrap();
    tree.upsert("d.txt", gix::objs::tree::EntryKind::Blob, blob)
        .unwrap();
    let tree = tree.write().unwrap().detach();
    let who = gix::actor::SignatureRef {
        name: "Test".into(),
        email: "test@example.com".into(),
        time: "1700000100 +0000",
    };
    git.commit_as(who, who, "refs/heads/side", "side\n", tree, [head.id])
        .unwrap()
        .to_string()
}

#[test]
fn a_mixed_reset_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    let head = Sha::new(repo.sha("HEAD"));
    Gix::default()
        .reset(&place(&repo), &head, ResetMode::Mixed)
        .unwrap();
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}

#[test]
fn a_switch_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    side(&repo);
    Gix::default().switch(&place(&repo), "side").unwrap();
    assert!(repo.dir().join("d.txt").exists(), "the checkout ran");
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}

#[test]
fn a_fast_forward_merge_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    let side = side(&repo);
    let merging = Merging {
        message: "m\n".into(),
        ..Merging::default()
    };
    let merged = Gix::default()
        .merge(&place(&repo), &Sha::new(side), &merging)
        .unwrap();
    assert!(merged.fast_forward);
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}

#[test]
fn restoring_another_file_keeps_a_racy_rewrite_seen() {
    let (_dir, repo) = raced();
    repo.write("c.txt", "changed\n");
    let restoring = Restoring {
        worktree: true,
        ..Restoring::default()
    };
    Gix::default()
        .restore(&place(&repo), &["c.txt".into()], &restoring)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.dir().join("c.txt")).unwrap(),
        "c\n"
    );
    assert_eq!(worktree_change(&repo, "a.txt"), Some(GitChange::Modified));
}
