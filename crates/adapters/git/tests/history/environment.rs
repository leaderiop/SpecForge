//! The variables git reads to find a repository, as the host's `locate` (what a guest's git calls and a collector's
//! repository detection ask) takes them: a ceiling as git does, and a `GIT_DIR` or `GIT_WORK_TREE` naming another
//! repository refused, never followed nor ignored (ADR 0352). Each test gives `locate` its variables through
//! `testing::locate_in`, never the process's environment, which the tests of this binary share; `environment.rs`
//! holds the one test that reads the real one.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use specforge_effects::git::{Git, GitChange, Located};
use specforge_git::Gix;
use specforge_git::testing::{Repo, locate_in};
use specforge_test::prelude::*;

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
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a search from below a git ceiling directory never reads the ceiling nor above it"
)]
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

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a GIT_DIR or GIT_WORK_TREE naming another repository than the one a directory is in is refused"
)]
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

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a GIT_DIR or GIT_WORK_TREE naming another repository than the one a directory is in is refused"
)]
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

/// git looks for a `.git` only strictly below the nearest ceiling: a repository found at the ceiling is none, whatever
/// its `core.worktree` names.
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a search from below a git ceiling directory never reads the ceiling nor above it"
)]
fn a_repository_at_the_ceiling_is_none_whatever_its_worktree() {
    let (_dir, real) = home();
    std::fs::create_dir_all(real.join("wt")).unwrap();
    let config = real.join("repo/.git/config");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str(&format!(
        "[core]\n\tworktree = {}\n",
        real.join("wt").display()
    ));
    std::fs::write(&config, text).unwrap();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("repo"));
    assert_eq!(
        vars.top(&real.join("repo/sub")),
        Err(Located::NotARepository)
    );
}

/// What lies at the ceiling is never opened: a repository there whose config git cannot read is none, not unavailable.
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a search from below a git ceiling directory never reads the ceiling nor above it"
)]
fn a_repository_at_the_ceiling_is_never_opened() {
    let (_dir, real) = unreadable_ceiling();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("repo"));
    assert_eq!(
        vars.top(&real.join("repo/sub")),
        Err(Located::NotARepository)
    );
}

/// [`home`], its repository's config one git cannot read: `\U` is no escape git config knows, as in a Windows path
/// written unescaped.
fn unreadable_ceiling() -> (tempfile::TempDir, PathBuf) {
    let (dir, real) = home();
    let config = real.join("repo/.git/config");
    let mut text = std::fs::read_to_string(&config).unwrap();
    text.push_str("[core]\n\tworktree = C:\\Users\\wt\n");
    std::fs::write(&config, text).unwrap();
    (dir, real)
}

/// git stops at a `.git` it cannot use below the ceiling; the search never goes on past it to the ceiling.
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a search from below a git ceiling directory never reads the ceiling nor above it"
)]
fn a_broken_git_below_the_ceiling_stops_the_search_there() {
    let (_dir, real) = unreadable_ceiling();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("repo"));
    std::fs::create_dir_all(real.join("repo/sub/.git")).unwrap();
    assert_eq!(
        vars.top(&real.join("repo/sub/deep")),
        Err(Located::Unavailable(format!(
            "{} has a .git that is no repository",
            real.join("repo/sub").display()
        )))
    );
    std::fs::remove_dir(real.join("repo/sub/.git")).unwrap();
    std::fs::write(real.join("repo/sub/.git"), "gitdir: nowhere\n").unwrap();
    assert_eq!(
        vars.top(&real.join("repo/sub/deep")),
        Err(Located::Unavailable(format!(
            "{} has a .git that is no repository",
            real.join("repo/sub").display()
        )))
    );
}

/// A repository above the nearest ceiling is none either.
#[test]
fn a_repository_above_the_ceiling_is_none() {
    let (_dir, real) = home();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("repo/sub"));
    assert_eq!(
        vars.top(&real.join("repo/sub/deep")),
        Err(Located::NotARepository)
    );
    assert_eq!(vars.top(&real.join("repo/sub")), Ok(real.join("repo")));
}

/// As git: an entry after an empty one is taken as written, its links not resolved, so a link to the repository there
/// is no ceiling of the real path searched from.
#[cfg(unix)]
#[test]
fn a_ceiling_after_an_empty_entry_is_not_resolved() {
    let (_dir, real) = home();
    std::os::unix::fs::symlink(real.join("repo"), real.join("link")).unwrap();
    let linked = Vars::default().set("GIT_CEILING_DIRECTORIES", real.join("link"));
    assert_eq!(
        linked.top(&real.join("repo/sub")),
        Err(Located::NotARepository)
    );
    let after_empty = std::env::join_paths([PathBuf::new(), real.join("link")]).unwrap();
    let vars = Vars::default().set("GIT_CEILING_DIRECTORIES", after_empty);
    assert_eq!(vars.top(&real.join("repo/sub")), Ok(real.join("repo")));
}

/// The variables that name a repository's other parts refuse one naming another than the repository's own, and keep
/// the repository's own; in no repository git reads none of them, and neither does `locate`.
#[test]
fn a_common_dir_object_store_or_alternate_elsewhere_refuses() {
    let (_dir, real) = home();
    let other = Repo::init(&real.join("other"));
    other.write("b.txt", "b\n");
    other.commit_all("other");
    let sub = real.join("repo/sub");
    for (name, own, elsewhere) in [
        ("GIT_COMMON_DIR", "repo/.git", "other/.git"),
        (
            "GIT_OBJECT_DIRECTORY",
            "repo/.git/objects",
            "other/.git/objects",
        ),
        (
            "GIT_ALTERNATE_OBJECT_DIRECTORIES",
            "repo/.git/objects",
            "other/.git/objects",
        ),
    ] {
        Vars::default()
            .set(name, real.join(elsewhere))
            .refused(&sub, name);
        let kept = Vars::default().set(name, real.join(own));
        assert_eq!(kept.top(&sub), Ok(real.join("repo")), "{name}");
        std::fs::create_dir_all(real.join("plain")).unwrap();
        let outside = Vars::default().set(name, real.join(elsewhere));
        assert_eq!(
            outside.top(&real.join("plain")),
            Err(Located::NotARepository),
            "{name}"
        );
    }
}

/// git hands a pre-commit hook of `commit -a` or `commit <paths>` a temporary index in the git directory, the one it
/// commits: `status` reads that index; one named anywhere else refuses.
#[test]
fn an_index_file_in_the_git_directory_is_read_and_one_elsewhere_refuses() {
    let (_dir, real) = home();
    let repo = Repo::at(&real.join("repo"));
    let git_dir = real.join("repo/.git");
    let committed = std::fs::read(git_dir.join("index")).unwrap();
    repo.write("sub/deep/a.txt", "changed\n");
    repo.stage_all();
    // the staged index becomes the hook's temporary one; the repository's own is left as committed
    std::fs::rename(git_dir.join("index"), git_dir.join("index.lock")).unwrap();
    std::fs::write(git_dir.join("index"), committed).unwrap();
    let staged = |vars: &Vars| {
        let Located::Repo(place) = locate_in(&real.join("repo"), &|name| vars.0.get(name).cloned())
        else {
            panic!("a repository");
        };
        let status = Gix::default().status(&place).unwrap();
        status
            .entries
            .iter()
            .map(|entry| entry.index)
            .collect::<Vec<_>>()
    };
    assert_eq!(staged(&Vars::default()), [None]);
    let hook = Vars::default().set("GIT_INDEX_FILE", git_dir.join("index.lock"));
    assert_eq!(staged(&hook), [Some(GitChange::Modified)]);
    std::fs::copy(git_dir.join("index.lock"), real.join("stray-index")).unwrap();
    Vars::default()
        .set("GIT_INDEX_FILE", real.join("stray-index"))
        .refused(&real.join("repo"), "GIT_INDEX_FILE");
}

/// A relative value is read from the working directory, as git reads it: the refusal shows where it lands.
#[test]
fn a_relative_git_dir_refusal_shows_where_it_lands() {
    let (_dir, real) = home();
    let written = "elsewhere/other.git";
    let landed = std::path::absolute(written).unwrap();
    let located = Vars::default()
        .set("GIT_DIR", written)
        .top(&real.join("repo/sub"));
    let shown = format!("GIT_DIR names {written} ({})", landed.display());
    assert!(
        matches!(&located, Err(Located::Unavailable(why)) if why.starts_with(&shown)),
        "{located:?}"
    );
}
