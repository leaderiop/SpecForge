//! Where git and libgit2 or gix differ on finding a repository and adding a worktree: these hold git's behaviour, which a
//! workspace of several worktrees relies on.

use specforge_effects::git::{Git, Located};
use specforge_git::Gix;
use specforge_git::testing::Repo;

/// A `.git` no repository answers for, inside a clean repository: `git status` refuses it, where a search going on
/// upwards reads the outer one, clean.
#[test]
fn a_broken_git_entry_stops_the_search_for_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    repo.commit_all("first");
    let git = Gix::default();
    for (name, broken) in [("file", true), ("dir", false)] {
        let inner = dir.path().join(name);
        std::fs::create_dir_all(inner.join("deep")).unwrap();
        if broken {
            std::fs::write(inner.join(".git"), "gitdir: /nonexistent\n").unwrap();
        } else {
            std::fs::create_dir(inner.join(".git")).unwrap();
        }
        for at in [inner.clone(), inner.join("deep")] {
            let located = git.locate(&at);
            assert!(
                matches!(&located, Located::Unavailable(why) if why.contains(".git")),
                "{}: {located:?}",
                at.display()
            );
        }
    }
    let plain = dir.path().join("plain");
    std::fs::create_dir(&plain).unwrap();
    assert!(
        matches!(git.locate(&plain), Located::Repo(_)),
        "a directory with no .git is still in the repository around it"
    );
}

#[test]
fn a_dangling_git_link_stops_the_search_for_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    repo.commit_all("first");
    let inner = dir.path().join("inner");
    std::fs::create_dir(&inner).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/nonexistent", inner.join(".git")).unwrap();
    #[cfg(windows)]
    std::fs::write(inner.join(".git"), "gitdir: /nonexistent\n").unwrap();
    assert!(matches!(
        Gix::default().locate(&inner),
        Located::Unavailable(_)
    ));
}
