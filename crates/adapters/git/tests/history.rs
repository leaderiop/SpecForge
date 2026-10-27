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
