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
