//! The git adapter reads a repository's commits through the History port (ADR 0287). Each test makes its own
//! repository; one without a `git` on `PATH` is skipped.

#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use std::path::Path;
use std::process::Command;

use specforge_git::GitCli;
use specforge_project::history::{History, Located, Sha};
use specforge_test::prelude::*;

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

fn has_git() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads the commits files through a port"
)]
fn history_reads_the_commits_files() {
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("spec/deep")).unwrap();
    std::fs::write(root.join("spec/a.spec"), "one\n").unwrap();
    std::fs::write(root.join("spec/deep/b é.spec"), "two\n").unwrap();
    std::fs::write(root.join("other.txt"), "x").unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    assert_eq!(cli.commit(&place, "HEAD").unwrap(), None, "unborn HEAD");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    std::fs::write(root.join("spec/a.spec"), "changed\n").unwrap();
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let mut files = cli.files(&place, &head, Path::new("spec")).unwrap();
    files.sort();
    assert_eq!(
        files,
        vec![
            ("spec/a.spec".into(), b"one\n".to_vec()),
            ("spec/deep/b é.spec".into(), b"two\n".to_vec()),
        ]
    );
    assert_eq!(cli.merge_base(&place, &head, &head).unwrap(), Some(head));
    assert_eq!(cli.commit(&place, "no-such-ref").unwrap(), None);
    let outside = tempfile::tempdir().unwrap();
    assert_eq!(cli.locate(outside.path()), Located::NotARepository);
    assert!(matches!(
        GitCli::with_program("no-such-git-program").locate(root),
        Located::Unavailable(_)
    ));
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads only the specs and the ledger of a commit"
)]
fn history_reads_only_the_specs_and_the_ledger() {
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
    std::fs::write(root.join("a.spec"), "one\n").unwrap();
    std::fs::write(root.join("specforge-harness"), "# specforge-harness 1\n").unwrap();
    std::fs::write(root.join("logo.png"), vec![0u8; 1 << 20]).unwrap();
    std::fs::write(root.join("node_modules/x/index.js"), "x").unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    let head = cli.commit(&place, "HEAD").unwrap().unwrap();
    let mut files = cli.files(&place, &head, Path::new("")).unwrap();
    files.sort();
    let names: Vec<_> = files.iter().map(|(p, _)| p.to_string_lossy()).collect();
    assert_eq!(names, ["a.spec", "specforge-harness"]);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history reads named files at a commit in one question"
)]
fn history_reads_named_files_at_a_commit() {
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "one\n").unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    std::fs::write(root.join("src/a.ts"), "changed\n").unwrap();
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let asked: Vec<std::path::PathBuf> = ["src/a.ts", "gone.txt", "src"]
        .iter()
        .map(Into::into)
        .collect();
    let found = cli.blobs(&place, &head, &asked).unwrap();
    // The commit's bytes, not the working tree's; a missing path and a directory are no file.
    assert_eq!(
        found,
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
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("spec")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("spec/a.spec"), "one\n").unwrap();
    std::fs::write(root.join("src/a.ts"), "four\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("a.ts", root.join("src/l.ts")).unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let mut all = cli.tree(&place, &head, Path::new("")).unwrap();
    all.sort();
    let mut expected = vec![("spec/a.spec".into(), true), ("src/a.ts".into(), true)];
    #[cfg(unix)]
    expected.push(("src/l.ts".into(), false));
    assert_eq!(all, expected);
    let mut under = cli.tree(&place, &head, Path::new("src")).unwrap();
    under.sort();
    assert_eq!(under, expected[1..]);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history gives the sizes of the named files, and a missing blob is not fetched"
)]
fn history_gives_the_sizes_of_the_named_files() {
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.ts"), "four\n").unwrap();
    std::fs::write(root.join("src/b c.ts"), "xy").unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let asked: Vec<std::path::PathBuf> = ["src/a.ts", "gone.txt", "src", "src/b c.ts", "src/x\ny"]
        .iter()
        .map(Into::into)
        .collect();
    let sizes: Vec<Option<u64>> = cli
        .sizes(&place, &head, &asked)
        .unwrap()
        .into_iter()
        .map(|(_, size)| size)
        .collect();
    assert_eq!(sizes, [Some(5), None, None, Some(2), None]);
}

/// A blobless clone of a repository with `src/a.ts` and `src/b.ts`, with the blobs of `HEAD` removed from the checkout
/// (`--no-checkout`): its objects count and git's version.
fn blobless_clone(parent: &Path) -> Option<std::path::PathBuf> {
    let origin = parent.join("origin");
    std::fs::create_dir_all(origin.join("src")).unwrap();
    std::fs::write(origin.join("src/a.ts"), "one\n").unwrap();
    std::fs::write(origin.join("src/b.ts"), "three\n").unwrap();
    git(&origin, &["init", "-q", "-b", "main"]);
    git(&origin, &["config", "uploadpack.allowFilter", "true"]);
    git(&origin, &["add", "-A"]);
    git(&origin, &["commit", "-q", "-m", "c1"]);
    let clone = parent.join("clone");
    let url = format!("file://{}", origin.display());
    let status = Command::new("git")
        .args(["clone", "-q", "--no-checkout", "--filter=blob:none", &url])
        .arg(&clone)
        .status()
        .ok()?;
    status.success().then_some(clone)
}

fn missing_objects(repo: &Path) -> usize {
    let out = Command::new("git")
        .args(["rev-list", "--objects", "--missing=print", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with('?'))
        .count()
}

/// A `git` that forgets `GIT_NO_LAZY_FETCH` (the adapter sets it), so any git behaves as one older than 2.44.
#[cfg(unix)]
fn old_git(dir: &Path) -> GitCli {
    use std::os::unix::fs::PermissionsExt;
    let script = dir.join("old-git");
    std::fs::write(
        &script,
        "#!/bin/sh\nunset GIT_NO_LAZY_FETCH\nexec git \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    GitCli::with_program(script)
}

#[cfg(unix)]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "history gives the sizes of the named files, and a missing blob is not fetched"
)]
fn a_listing_never_fetches_a_missing_blob() {
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let Some(clone) = blobless_clone(dir.path()) else {
        return; // a git that cannot filter a clone (or file:// is refused): nothing to test
    };
    let before = missing_objects(&clone);
    assert!(before >= 2, "the clone is blobless: {before}");
    // an old git may fetch lazily: the listing must still ask for no blob (it has no `-l`)
    let cli = old_git(dir.path());
    let Located::Repo(place) = cli.locate(&clone) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let mut tree = cli.tree(&place, &head, Path::new("")).unwrap();
    tree.sort();
    assert_eq!(tree, [("src/a.ts".into(), true), ("src/b.ts".into(), true)]);
    assert_eq!(missing_objects(&clone), before);
    // asking a size looks one blob up; an old git fetches that file and only that one (ADR 0300 D5)
    let sizes = cli.sizes(&place, &head, &["src/a.ts".into()]).unwrap();
    assert_eq!(sizes.len(), 1);
    assert_eq!(missing_objects(&clone), before - 1);
    // with the adapter's own GIT_NO_LAZY_FETCH (git 2.44) it answers "missing" and fetches nothing
    let dir = tempfile::tempdir().unwrap();
    let Some(clone) = blobless_clone(dir.path()) else {
        return;
    };
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(&clone) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    let sizes = cli.sizes(&place, &head, &["src/a.ts".into()]).unwrap();
    if git_forbids_lazy_fetch() {
        assert_eq!(sizes, [("src/a.ts".into(), None)]);
        assert_eq!(missing_objects(&clone), before);
    }
}

/// Whether `git` knows `GIT_NO_LAZY_FETCH` (2.44).
fn git_forbids_lazy_fetch() -> bool {
    let out = Command::new("git").arg("--version").output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut parts = text.trim().trim_start_matches("git version ").split('.');
    let number = |p: Option<&str>| p.and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    (number(parts.next()), number(parts.next())) >= (2, 44)
}

#[cfg(unix)]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "git refuses a project directory that is not valid UTF-8"
)]
fn a_directory_that_is_not_utf8_is_refused() {
    use std::os::unix::ffi::OsStrExt;
    if !has_git() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("a.spec"), "one\n").unwrap();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "c1"]);
    let cli = GitCli::default();
    let Located::Repo(place) = cli.locate(root) else {
        panic!("a repository");
    };
    let head: Sha = cli.commit(&place, "HEAD").unwrap().unwrap();
    // a mangled pathspec would list nothing, and every file would read as absent
    let odd = Path::new(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    let why = cli.files(&place, &head, odd).expect_err("refused");
    assert!(why.0.contains("UTF-8"), "{why}");
    cli.tree(&place, &head, odd).expect_err("refused");
}
