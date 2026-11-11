//! The variables git reads to find a repository, as the host's `locate` (what a guest's git calls and a collector's
//! repository detection ask) takes them: a ceiling as git does, and a `GIT_DIR` or `GIT_WORK_TREE` naming another
//! repository refused, never followed nor ignored (ADR 0352). Each test gives `locate` its variables through
//! `testing::locate_in`, never the process's environment, which the tests of this binary share; `environment.rs`
//! holds the one test that reads the real one.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use specforge_effects::git::Located;
use specforge_git::testing::{Repo, locate_in};

/// A temporary directory by its real path (a ceiling is compared once links are resolved), with a committed repository
/// at `repo/`.
fn home() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().canonicalize().unwrap();
    let repo = Repo::init(&real.join("repo"));
    repo.write("sub/deep/a.txt", "a\n");
    repo.commit_all("first");
    (dir, real)
}

/// git's variables as a test sets them.
#[derive(Default)]
struct Vars(BTreeMap<&'static str, OsString>);

impl Vars {
    fn set(mut self, name: &'static str, value: impl Into<OsString>) -> Self {
        self.0.insert(name, value.into());
        self
    }

    /// The top `locate` answers for `dir` under these variables, or what else it answers.
    fn top(&self, dir: &Path) -> Result<PathBuf, Located> {
        match locate_in(dir, &|name| self.0.get(name).cloned()) {
            Located::Repo(place) => Ok(place.toplevel),
            other => Err(other),
        }
    }

    /// `locate` of `dir` refuses, naming `variable`.
    fn refused(&self, dir: &Path, variable: &str) {
        let located = self.top(dir);
        assert!(
            matches!(&located, Err(Located::Unavailable(why)) if why.starts_with(variable)),
            "{located:?}"
        );
    }
}

/// As git: a search from below a ceiling never reads the ceiling nor above it; one from the ceiling itself, or from
/// beside every ceiling, goes on as without one.
#[test]
fn a_ceiling_directory_is_never_searched_from_below() {
    let (_dir, real) = home();
    let ceilings = std::env::join_paths([real.join("repo"), real.join("elsewhere")]).unwrap();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", ceilings);
    for below in ["repo/sub", "repo/sub/deep"] {
        assert_eq!(
            vars.top(&real.join(below)),
            Err(Located::NotARepository),
            "{below}"
        );
    }
    assert_eq!(vars.top(&real.join("repo")), Ok(real.join("repo")));
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("elsewhere"));
    assert_eq!(vars.top(&real.join("repo/sub/deep")), Ok(real.join("repo")));
}

#[test]
fn a_git_dir_naming_another_repository_refuses() {
    let (_dir, real) = home();
    let other = Repo::init(&real.join("other"));
    other.write("b.txt", "b\n");
    other.commit_all("other");
    Vars::default()
        .set("GIT_DIR", real.join("other/.git"))
        .refused(&real.join("repo/sub"), "GIT_DIR");
}

#[test]
fn a_git_dir_with_no_repository_around_refuses() {
    let (_dir, real) = home();
    std::fs::create_dir_all(real.join("plain")).unwrap();
    Vars::default()
        .set("GIT_DIR", real.join("repo/.git"))
        .refused(&real.join("plain"), "GIT_DIR");
}

#[test]
fn a_work_tree_naming_another_directory_refuses() {
    let (_dir, real) = home();
    std::fs::create_dir_all(real.join("elsewhere")).unwrap();
    Vars::default()
        .set("GIT_WORK_TREE", real.join("elsewhere"))
        .refused(&real.join("repo/sub"), "GIT_WORK_TREE");
}

/// What a hook git runs in a linked worktree is given: the variables naming the repository it is in.
#[test]
fn a_git_dir_naming_the_repository_the_directory_is_in_is_kept() {
    let (_dir, real) = home();
    let vars = Vars::default()
        .set("GIT_DIR", real.join("repo/.git"))
        .set("GIT_WORK_TREE", real.join("repo"));
    assert_eq!(vars.top(&real.join("repo/sub")), Ok(real.join("repo")));
}
