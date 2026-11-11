//! The variables git reads to find a repository, as the host's `locate` (what a guest's git calls and a collector's
//! repository detection ask) takes them: a ceiling as git does, and a `GIT_DIR` or `GIT_WORK_TREE` naming another
//! repository refused, never followed nor ignored (ADR 0352). Each test sets its variables, and `HOME` under its own
//! temporary directory, before any thread reads the environment: nextest runs every test in its own process.

use std::path::{Path, PathBuf};

use specforge_effects::git::{Git, Located};
use specforge_git::Gix;
use specforge_git::testing::Repo;

/// A temporary directory by its real path (a ceiling is compared once links are resolved), with a committed repository
/// at `repo/`, and `HOME` set under it so no host config takes part.
fn home() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().canonicalize().unwrap();
    std::fs::create_dir_all(real.join("home")).unwrap();
    set("HOME", &real.join("home"));
    let repo = Repo::init(&real.join("repo"));
    repo.write("sub/deep/a.txt", "a\n");
    repo.commit_all("first");
    (dir, real)
}

/// `name` set to `value` in this test's process.
#[allow(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn set(name: &str, value: &Path) {
    // SAFETY: nextest runs each test in its own process, and no thread has started yet.
    unsafe { std::env::set_var(name, value) };
}

/// The top `locate` answers for `dir`, or what else it answers.
fn top(dir: &Path) -> Result<PathBuf, Located> {
    match Gix::default().locate(dir) {
        Located::Repo(place) => Ok(place.toplevel),
        other => Err(other),
    }
}

/// As git: a search from below a ceiling never reads the ceiling nor above it; one from the ceiling itself, or from
/// beside every ceiling, goes on as without one.
/// `locate` of `dir` refuses, naming `variable`.
fn refused(dir: &Path, variable: &str) {
    let located = top(dir);
    assert!(
        matches!(&located, Err(Located::Unavailable(why)) if why.starts_with(variable)),
        "{located:?}"
    );
}

#[test]
fn a_ceiling_directory_is_never_searched_from_below() {
    let (_dir, real) = home();
    let ceilings = std::env::join_paths([real.join("repo"), real.join("elsewhere")]).unwrap();
    set("GIT_CEILING_DIRECTORIES", Path::new(&ceilings));
    for below in ["repo/sub", "repo/sub/deep"] {
        assert_eq!(
            top(&real.join(below)),
            Err(Located::NotARepository),
            "{below}"
        );
    }
    assert_eq!(top(&real.join("repo")), Ok(real.join("repo")));
    set("GIT_CEILING_DIRECTORIES", &real.join("elsewhere"));
    assert_eq!(top(&real.join("repo/sub/deep")), Ok(real.join("repo")));
}

#[test]
fn a_git_dir_naming_another_repository_refuses() {
    let (_dir, real) = home();
    let other = Repo::init(&real.join("other"));
    other.write("b.txt", "b\n");
    other.commit_all("other");
    set("GIT_DIR", &real.join("other/.git"));
    refused(&real.join("repo/sub"), "GIT_DIR");
}

#[test]
fn a_git_dir_with_no_repository_around_refuses() {
    let (_dir, real) = home();
    std::fs::create_dir_all(real.join("plain")).unwrap();
    set("GIT_DIR", &real.join("repo/.git"));
    refused(&real.join("plain"), "GIT_DIR");
}

#[test]
fn a_work_tree_naming_another_directory_refuses() {
    let (_dir, real) = home();
    std::fs::create_dir_all(real.join("elsewhere")).unwrap();
    set("GIT_WORK_TREE", &real.join("elsewhere"));
    refused(&real.join("repo/sub"), "GIT_WORK_TREE");
}

/// What a hook git runs in a linked worktree is given: the variables naming the repository it is in.
#[test]
fn a_git_dir_naming_the_repository_the_directory_is_in_is_kept() {
    let (_dir, real) = home();
    set("GIT_DIR", &real.join("repo/.git"));
    set("GIT_WORK_TREE", &real.join("repo"));
    assert_eq!(top(&real.join("repo/sub")), Ok(real.join("repo")));
}
