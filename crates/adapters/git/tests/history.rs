//! The gix adapter reads a repository's commits through the Git port (ADR 0287, ADR 0335, ADR 0339 D5). Each test
//! builds its own repository with gix: no test needs the git program.

#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::path::{Path, PathBuf};

use specforge_effects::git::{
    Git, GitBranch, GitChange, GitEntry, GitEntryKind, GitPlace, Located, Sha,
};
use specforge_git::Gix;
use specforge_git::testing::Repo;
use specforge_test::prelude::*;

#[path = "history/spike.rs"]
mod spike;
#[path = "history/writes.rs"]
mod writes;

/// What a baseline asks the files of: the specs and the harness ledger.
fn baseline(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "spec")
        || path.file_name().is_some_and(|n| n == "specforge-harness")
}

/// Where `repo` is, as the port locates it.
fn place(repo: &Repo) -> GitPlace {
    let Located::Repo(place) = Gix.locate(repo.dir()) else {
        panic!("a repository");
    };
    place
}

#[specforge_test(port = "Git", verify = "Git contract is satisfied")]
fn gix_keeps_the_git_contract() {
    let dir = tempfile::tempdir().unwrap();
    specforge_effects::testing::git_contract(&Gix, dir.path());
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads the commits files through a port"
)]
fn history_reads_the_commits_files() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("spec/a.spec", "one\n")
        .write("spec/deep/b é.spec", "two\n")
        .write("other.txt", "x");
    let place = place(&repo);
    assert_eq!(Gix.commit(&place, "HEAD").unwrap(), None, "unborn HEAD");
    repo.commit_all("c1");
    repo.write("spec/a.spec", "changed\n");
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    let mut files = Gix
        .files(&place, &head, Path::new("spec"), &baseline)
        .unwrap();
    files.sort();
    assert_eq!(
        files,
        vec![
            ("spec/a.spec".into(), b"one\n".to_vec()),
            ("spec/deep/b é.spec".into(), b"two\n".to_vec()),
        ]
    );
    assert_eq!(Gix.merge_base(&place, &head, &head).unwrap(), Some(head));
    assert_eq!(Gix.commit(&place, "no-such-ref").unwrap(), None);
    let outside = tempfile::tempdir().unwrap();
    assert_eq!(Gix.locate(outside.path()), Located::NotARepository);
    // A directory that does not exist holds no repository.
    assert_eq!(
        Gix.locate(&repo.dir().join("gone")),
        Located::NotARepository
    );
    // A bare repository has no working tree to judge.
    let bare = tempfile::tempdir().unwrap();
    gix::init_bare(bare.path()).unwrap();
    assert!(matches!(Gix.locate(bare.path()), Located::Unavailable(_)));
    // Inside the git directory there is no working tree either, as git says ("must be run in a work tree"): a hook
    // reads a `.git/` path as a repository's internals, not as its worktree.
    for inside in [repo.dir().join(".git"), repo.dir().join(".git/refs")] {
        assert!(
            matches!(Gix.locate(&inside), Located::Unavailable(_)),
            "{}",
            inside.display()
        );
    }
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads only the specs and the ledger of a commit"
)]
fn history_reads_only_the_specs_and_the_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.spec", "one\n")
        .write("specforge-harness", "# specforge-harness 1\n")
        .write("logo.png", vec![0u8; 1 << 20])
        .write("node_modules/x/index.js", "x");
    repo.commit_all("c1");
    let place = place(&repo);
    let head = Gix.commit(&place, "HEAD").unwrap().unwrap();
    let mut files = Gix.files(&place, &head, Path::new(""), &baseline).unwrap();
    files.sort();
    let names: Vec<_> = files.iter().map(|(p, _)| p.to_string_lossy()).collect();
    assert_eq!(names, ["a.spec", "specforge-harness"]);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads named files at a commit in one question"
)]
fn history_reads_named_files_at_a_commit() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("src/a.ts", "one\n");
    repo.commit_all("c1");
    repo.write("src/a.ts", "changed\n");
    let place = place(&repo);
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    let asked: Vec<PathBuf> = ["src/a.ts", "gone.txt", "src"]
        .iter()
        .map(Into::into)
        .collect();
    // The commit's bytes, not the working tree's; a missing path and a directory are no file.
    assert_eq!(
        Gix.blobs(&place, &head, &asked).unwrap(),
        vec![
            ("src/a.ts".into(), Some(b"one\n".to_vec())),
            ("gone.txt".into(), None),
            ("src".into(), None),
        ]
    );
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history lists a commit's tree with no content and no size"
)]
fn history_lists_a_commits_tree() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("spec/a.spec", "one\n")
        .write("src/a.ts", "four\n");
    #[cfg(unix)]
    std::os::unix::fs::symlink("a.ts", repo.dir().join("src/l.ts")).unwrap();
    repo.commit_all("c1");
    let place = place(&repo);
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    let mut all = Gix.tree(&place, &head, Path::new("")).unwrap();
    all.sort();
    let mut expected = vec![("spec/a.spec".into(), true), ("src/a.ts".into(), true)];
    #[cfg(unix)]
    expected.push(("src/l.ts".into(), false));
    assert_eq!(all, expected);
    let mut under = Gix.tree(&place, &head, Path::new("src")).unwrap();
    under.sort();
    assert_eq!(under, expected[1..]);
    assert_eq!(Gix.tree(&place, &head, Path::new("nowhere")).unwrap(), []);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history gives the sizes of the named files, and a missing blob is not fetched"
)]
fn history_gives_the_sizes_of_the_named_files() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("src/a.ts", "four\n").write("src/b c.ts", "xy");
    repo.commit_all("c1");
    let place = place(&repo);
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    let asked: Vec<PathBuf> = ["src/a.ts", "gone.txt", "src", "src/b c.ts", "src/x\ny"]
        .iter()
        .map(Into::into)
        .collect();
    let sizes: Vec<Option<u64>> = Gix
        .sizes(&place, &head, &asked)
        .unwrap()
        .into_iter()
        .map(|(_, size)| size)
        .collect();
    assert_eq!(sizes, [Some(5), None, None, Some(2), None]);
}

/// A partial clone that lacks `src/a.ts`'s blob: the repository names a promisor remote that cannot be reached, and
/// the loose object is gone.
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history gives the sizes of the named files, and a missing blob is not fetched"
)]
fn a_blob_a_partial_clone_lacks_reads_missing_and_is_never_fetched() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(&dir.path().join("clone"));
    repo.write("src/a.ts", "one\n").write("src/b.ts", "three\n");
    repo.commit_all("c1");
    let lost = repo.sha("HEAD:src/a.ts");
    let promisor = dir.path().join("promisor-that-is-not-there");
    repo.remote("origin", &promisor.display().to_string())
        .config("[remote \"origin\"]\n\tpromisor = true\n\tpartialclonefilter = blob:none")
        .config("[extensions]\n\tpartialClone = origin");
    let object = repo
        .git_dir()
        .join("objects")
        .join(&lost[..2])
        .join(&lost[2..]);
    std::fs::remove_file(object).unwrap();
    let place = place(&repo);
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    // a listing reads no blob, and names both files
    let mut tree = Gix.tree(&place, &head, Path::new("")).unwrap();
    tree.sort();
    assert_eq!(tree, [("src/a.ts".into(), true), ("src/b.ts".into(), true)]);
    let asked: Vec<PathBuf> = vec!["src/a.ts".into(), "src/b.ts".into()];
    assert_eq!(
        Gix.sizes(&place, &head, &asked).unwrap(),
        [("src/a.ts".into(), None), ("src/b.ts".into(), Some(6))]
    );
    assert_eq!(
        Gix.blobs(&place, &head, &asked).unwrap(),
        [
            ("src/a.ts".into(), None),
            ("src/b.ts".into(), Some(b"three\n".to_vec()))
        ]
    );
    Gix.files(&place, &head, Path::new("src"), &|_| true)
        .expect_err("a wanted file the clone lacks cannot be given");
    assert!(!promisor.exists(), "nothing was fetched");
}

#[cfg(unix)]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "git refuses a project directory that is not valid UTF-8"
)]
fn a_directory_that_is_not_utf8_is_refused() {
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.spec", "one\n");
    repo.commit_all("c1");
    let place = place(&repo);
    let head: Sha = Gix.commit(&place, "HEAD").unwrap().unwrap();
    // a mangled path would list nothing, and every file would read as absent
    let odd = Path::new(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    let why = Gix
        .files(&place, &head, odd, &baseline)
        .expect_err("refused");
    assert!(why.0.contains("UTF-8"), "{why}");
    Gix.tree(&place, &head, odd).expect_err("refused");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "history answers the branch and the typed status of a repository"
)]
fn history_answers_the_branch_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("c1");
    repo.write("n.txt", "x");
    let place = place(&repo);
    assert_eq!(Gix.branch(&place).unwrap(), "main");
    let status = Gix.status(&place).unwrap();
    assert!(status.dirty(), "an untracked file");
    assert_eq!(status.branch.head.as_deref(), Some("main"));
    assert_eq!(status.branch.oid.as_deref(), Some(first.as_str()));
    assert!(
        status
            .entries
            .iter()
            .any(|e| e.kind == GitEntryKind::Untracked && e.path == "n.txt"),
        "{status:?}"
    );
    repo.detach(&first);
    assert_eq!(Gix.branch(&place).unwrap(), "");
    assert_eq!(Gix.status(&place).unwrap().branch.head, None);
}

/// gsd-core 1.19.0 (#5048, #5699): a read-only status must not take the `.git/index.lock` a concurrent `git commit`
/// holds. gix's status writes no index, so it answers beside the lock and leaves it.
#[specforge_test(
    behavior = "run_a_hook",
    verify = "history answers the branch and the typed status of a repository"
)]
fn status_takes_no_index_lock() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    repo.commit_all("c1");
    repo.write("a.txt", "changed");
    let lock = repo.git_dir().join("index.lock");
    std::fs::write(&lock, "held by a concurrent commit").unwrap();
    let status = Gix.status(&place(&repo)).unwrap();
    assert!(status.dirty());
    assert_eq!(
        std::fs::read_to_string(&lock).unwrap(),
        "held by a concurrent commit"
    );
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "history answers the branch and the typed status of a repository"
)]
fn a_tree_is_dirty_by_a_tracked_change_a_conflict_or_an_untracked_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write(".gitignore", "target/\n").write("a.txt", "a");
    repo.commit_all("c1");
    let place = place(&repo);
    // an ignored file is not dirty (ADR 0327 D3)
    repo.write("target/out.bin", "built");
    assert!(!Gix.status(&place).unwrap().dirty(), "ignored only");
    repo.write("a.txt", "changed");
    assert!(Gix.status(&place).unwrap().dirty(), "an unstaged change");
    repo.stage(&["a.txt"]);
    assert!(Gix.status(&place).unwrap().dirty(), "a staged change");
    repo.commit("c2");
    assert!(!Gix.status(&place).unwrap().dirty(), "committed");
    repo.write("n.txt", "n");
    assert!(Gix.status(&place).unwrap().dirty(), "an untracked file");
    repo.delete("n.txt");
    repo.conflict("a.txt", "base\n", "ours\n", "theirs\n");
    assert!(Gix.status(&place).unwrap().dirty(), "a conflict");
}

#[specforge_test(
    behavior = "run_a_hook",
    verify = "history answers the branch and the typed status of a repository"
)]
fn gix_fills_a_git_status_for_every_entry_kind() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("staged.txt", "s\n")
        .write("unstaged.txt", "u\n")
        .write(
            "from.txt",
            "a file long enough to be found again under its new name\n",
        )
        .write("both.txt", "b\n")
        .write("gone.txt", "g\n");
    let base = repo.commit_all("c1");
    repo.write("remote.txt", "r\n");
    let theirs = repo.commit_all("on the remote");
    repo.branch("main", &base);
    repo.delete("remote.txt");
    repo.write("ahead.txt", "a\n");
    let ours = repo.commit_all("local");
    repo.remote("origin", "https://example.com/acme/widgets.git")
        .upstream("main", &theirs);
    repo.write("staged.txt", "s2\n");
    repo.stage(&["staged.txt"]);
    repo.write("unstaged.txt", "u2\n");
    std::fs::rename(repo.dir().join("from.txt"), repo.dir().join("to.txt")).unwrap();
    repo.stage(&["from.txt", "to.txt"]);
    repo.delete("gone.txt");
    repo.conflict("both.txt", "b\n", "ours\n", "theirs\n");
    repo.write("new/inner.txt", "n").write("z.txt", "z");
    let status = Gix.status(&place(&repo)).unwrap();
    assert_eq!(
        status.branch,
        GitBranch {
            oid: Some(ours),
            head: Some("main".into()),
            upstream: Some("origin/main".into()),
            ahead: 1,
            behind: 1,
        }
    );
    let entry = |kind, path: &str, orig: Option<&str>, index, worktree| GitEntry {
        kind,
        path: path.into(),
        orig_path: orig.map(Into::into),
        index,
        worktree,
    };
    use GitChange::{Deleted, Modified, Renamed, Unmerged};
    use GitEntryKind::{Changed, Untracked};
    assert_eq!(
        status.entries,
        [
            entry(
                GitEntryKind::Unmerged,
                "both.txt",
                None,
                Some(Unmerged),
                Some(Unmerged)
            ),
            entry(Changed, "gone.txt", None, None, Some(Deleted)),
            entry(Changed, "staged.txt", None, Some(Modified), None),
            entry(
                GitEntryKind::Renamed,
                "to.txt",
                Some("from.txt"),
                Some(Renamed),
                None
            ),
            entry(Changed, "unstaged.txt", None, None, Some(Modified)),
            entry(Untracked, "new/", None, None, None),
            entry(Untracked, "z.txt", None, None, None),
        ]
    );
}
