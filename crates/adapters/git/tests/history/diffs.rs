//! A diff's typed line ranges, a root commit's diff from no tree, a diff with renames left out, and a signature's epoch
//! seconds: what a guest reading a commit's changed lines needs of the port.

use specforge_effects::git::{
    DiffQuery, DiffSide, DiffStatus, Git, GitFileDiff, GitHunk, LogQuery, Sha,
};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

/// The diff of `head` against `base` (none: the empty tree), with hunks and with `renames` as given.
fn diff(repo: &Repo, base: Option<&str>, head: &str, renames: bool) -> Vec<GitFileDiff> {
    Gix::default()
        .diff(
            &place(repo),
            &DiffQuery {
                base: base.map(Sha::new),
                head: DiffSide::Commit(Sha::new(head)),
                paths: Vec::new(),
                patch: false,
                hunks: true,
                renames,
                budget: None,
            },
        )
        .unwrap()
}

fn hunk(before: (u32, u32), after: (u32, u32)) -> GitHunk {
    GitHunk {
        before_start: before.0,
        before_end: before.1,
        after_start: after.0,
        after_end: after.1,
    }
}

/// `n` numbered lines, `line 1` to `line n`.
fn lines(n: u32) -> String {
    (1..=n).map(|i| format!("line {i}\n")).collect()
}

#[test]
fn a_root_commit_diffs_from_no_tree() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", lines(3)).write("d/b.txt", "b\n");
    let root = repo.commit_all("root");
    let files = diff(&repo, None, &root, false);
    let seen: Vec<_> = files
        .iter()
        .map(|f| (f.path.as_str(), f.status, f.added, f.removed))
        .collect();
    assert_eq!(
        seen,
        [
            ("a.txt", DiffStatus::Added, 3, 0),
            ("d/b.txt", DiffStatus::Added, 1, 0)
        ]
    );
    assert_eq!(files[0].hunks, Some(vec![hunk((0, 0), (0, 3))]));
}

#[test]
fn a_diff_names_its_changed_line_ranges_without_context() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("changed.txt", lines(10))
        .write("deleted.txt", lines(10))
        .write("appended.txt", lines(10));
    let base = repo.commit_all("base");
    let changed = lines(10).replace("line 5\n", "line five\n");
    let deleted = lines(10).replace("line 4\nline 5\n", "");
    let appended = format!("{}line 11\nline 12\n", lines(10));
    repo.write("changed.txt", changed)
        .write("deleted.txt", deleted)
        .write("appended.txt", appended);
    let head = repo.commit_all("head");
    let files = diff(&repo, Some(&base), &head, false);
    let hunks = |path: &str| {
        files
            .iter()
            .find(|f| f.path == path)
            .and_then(|f| f.hunks.clone())
            .unwrap()
    };
    // line 5 (index 4) became line 5; no line of context around it
    assert_eq!(hunks("changed.txt"), [hunk((4, 5), (4, 5))]);
    // lines 4 and 5 went: nothing written, placed before what is now line 4
    assert_eq!(hunks("deleted.txt"), [hunk((3, 5), (3, 3))]);
    assert_eq!(hunks("appended.txt"), [hunk((10, 10), (10, 12))]);
}

#[test]
fn a_diff_without_renames_lists_a_deletion_and_an_addition() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("old.txt", lines(20));
    let base = repo.commit_all("base");
    repo.delete("old.txt").write("new.txt", lines(20));
    let head = repo.commit_all("moved");
    let apart: Vec<_> = diff(&repo, Some(&base), &head, false)
        .into_iter()
        .map(|f| (f.path, f.status, f.added, f.removed))
        .collect();
    assert_eq!(
        apart,
        [
            ("new.txt".to_string(), DiffStatus::Added, 20, 0),
            ("old.txt".to_string(), DiffStatus::Deleted, 0, 20)
        ]
    );
    let paired = diff(&repo, Some(&base), &head, true);
    assert_eq!(paired.len(), 1, "{paired:?}");
    assert_eq!(paired[0].status, DiffStatus::Renamed);
    assert_eq!(paired[0].old_path.as_deref(), Some("old.txt"));
}

#[test]
fn a_signature_carries_its_epoch_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a\n");
    let sha = repo.commit_all("first");
    let log = Gix::default()
        .log(
            &place(&repo),
            &LogQuery {
                tips: vec![Sha::new(sha)],
                ..LogQuery::default()
            },
        )
        .unwrap();
    let author = &log.commits[0].author;
    // the test clock's first commit
    assert_eq!(author.seconds, 1_700_000_000);
    assert_eq!(author.time, "2023-11-14T22:13:20+00:00");
    assert_eq!(log.commits[0].committer.seconds, 1_700_000_000);
}

/// A guest's log never reads a withheld file: its pickaxe finds no commit whose only match is in a credential,
/// renamed or not; the user's own log does.
#[test]
fn a_withheld_log_reads_no_credential_with_its_pickaxe() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "plain\n")
        .write(".env", "TOKEN=abc123\n");
    let sha = repo.commit_all("secret");
    let query = |withhold| LogQuery {
        tips: vec![Sha::new(sha.clone())],
        pickaxe: Some("abc123".into()),
        withhold,
        ..LogQuery::default()
    };
    let found = |withhold| {
        Gix::default()
            .log(&place(&repo), &query(withhold))
            .unwrap()
            .commits
            .len()
    };
    assert_eq!((found(false), found(true)), (1, 0));
}

/// A tracked file whose directory became a link to a place outside the repository: git reads it as gone (it never
/// reads through a link a path leads under), a conflicted entry too, which the status gives whatever the disk holds.
#[cfg(unix)]
#[test]
fn a_worktree_diff_reads_no_file_through_a_linked_directory() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("a.txt"), "outside\n").unwrap();
    let repo = Repo::init(dir.path());
    repo.write("d/a.txt", "inside\n");
    let base = repo.commit_all("base");
    repo.conflict("d/a.txt", "b\n", "o\n", "t\n");
    std::fs::remove_dir_all(dir.path().join("d")).unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("d")).unwrap();
    let files = Gix::default()
        .diff(
            &place(&repo),
            &DiffQuery {
                base: Some(Sha::new(base)),
                head: DiffSide::Worktree,
                paths: Vec::new(),
                patch: true,
                hunks: false,
                renames: false,
                budget: None,
            },
        )
        .unwrap();
    let [file] = &files[..] else {
        panic!("one file: {files:?}");
    };
    assert_eq!(file.status, DiffStatus::Deleted);
    assert!(
        !file.patch.as_deref().unwrap().contains("outside"),
        "{file:?}"
    );
}

/// A bounded worktree diff reads files from disk until its bound is spent, the first always; it neither reads nor
/// hashes the rest: each is listed as changed, with no counts, `truncated` and `unread`.
#[test]
fn a_bounded_worktree_diff_reads_no_file_past_its_bound() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    for i in 0..5 {
        repo.write(&format!("f{i}.txt"), "x\n".repeat(2048));
    }
    let base = repo.commit_all("base");
    for i in 0..5 {
        repo.write(&format!("f{i}.txt"), "y\n".repeat(2048));
    }
    let files = Gix::default()
        .diff(
            &place(&repo),
            &DiffQuery {
                base: Some(Sha::new(base)),
                head: DiffSide::Worktree,
                paths: Vec::new(),
                patch: false,
                hunks: false,
                renames: false,
                // three files: from disk, and from the base
                budget: Some(3 * 4096 + 32),
            },
        )
        .unwrap();
    let read: Vec<_> = files.iter().filter(|f| f.added > 0).collect();
    let unread: Vec<_> = files.iter().filter(|f| f.unread).collect();
    assert_eq!(
        (files.len(), read.len(), unread.len()),
        (5, 3, 2),
        "{files:?}"
    );
    assert!(
        unread
            .iter()
            .all(|f| f.truncated && f.status == DiffStatus::Modified && f.removed == 0)
    );
}
