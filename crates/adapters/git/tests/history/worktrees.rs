//! Where git and libgit2 or gix differ on finding a repository and adding a worktree: these hold git's behaviour, which a
//! workspace of several worktrees relies on, and say so where they hold a stricter one.

use specforge_effects::git::{Git, Located};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

/// A `.git` no repository answers for, inside a clean repository, stops the search: a search going on upwards would read
/// the outer one, clean. For a `.git` file this is git's own refusal. For an empty `.git` directory it is not: git goes
/// on upwards, and this refuses on purpose (ADR 0339).
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

#[test]
fn a_worktree_makes_its_parents_and_two_of_one_name_take_two_records() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.branch("one", &first).branch("two", &first);
    let linked = tempfile::tempdir().unwrap();
    let linked = linked.path().canonicalize().unwrap();
    let git = Gix::default();
    let place = place(&repo);
    git.worktree_add(&place, &linked.join("ws1/deep/repo"), "one")
        .unwrap();
    git.worktree_add(&place, &linked.join("ws2/deep/repo"), "two")
        .unwrap();
    let listed: Vec<_> = git.worktrees(&place).unwrap();
    assert!(
        listed
            .iter()
            .any(|w| w.path == linked.join("ws1/deep/repo"))
    );
    assert!(
        listed
            .iter()
            .any(|w| w.path == linked.join("ws2/deep/repo"))
    );
    assert!(linked.join("ws2/deep/repo/a.txt").is_file());
    git.worktree_remove(&place, &linked.join("ws1/deep/repo"))
        .unwrap();
    git.worktree_remove(&place, &linked.join("ws2/deep/repo"))
        .unwrap();
    assert!(!linked.join("ws1/deep/repo").exists());
    assert!(!linked.join("ws2/deep/repo").exists());
}

#[test]
fn a_local_clone_has_the_history_and_checks_out_head() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n").write("deep/b.txt", "two\n");
    let first = repo.commit_all("first");
    repo.write("a.txt", "changed, not committed\n");
    let into = tempfile::tempdir().unwrap();
    let into = into.path().canonicalize().unwrap().join("new/clone");
    let git = Gix::default();
    git.clone_local(&place(&repo), &into).unwrap();
    // A host's `core.autocrlf` may turn the line ending into CRLF on checkout; the content is what is checked.
    assert_eq!(
        std::fs::read_to_string(into.join("a.txt"))
            .unwrap()
            .trim_end(),
        "one"
    );
    assert!(into.join("deep/b.txt").is_file());
    let Located::Repo(made) = git.locate(&into) else {
        panic!("a repository")
    };
    let head = git.head(&made).unwrap();
    assert_eq!(head.head.as_deref(), Some("main"));
    assert_eq!(head.oid.as_deref(), Some(first.as_str()));
    let origin = git2::Repository::open(&into)
        .unwrap()
        .find_remote("origin")
        .unwrap()
        .url()
        .unwrap()
        .to_string();
    assert_eq!(origin, dir.path().canonicalize().unwrap().to_str().unwrap());
    assert!(!made.git_dir.join("hooks/post-checkout").exists());
}

/// git's local clone runs nothing of the source's: a hook planted there stays unrun, and a link planted under its
/// `objects/` is not followed.
#[test]
#[cfg(unix)]
fn a_clone_runs_no_hook_and_follows_no_link_in_the_source() {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let witness = outside.path().join("hook-ran");
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "one\n");
    repo.commit_all("first");
    let hook = repo.git_dir().join("hooks/post-checkout");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", witness.display())).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    // A filter driver the source's config names, and an attribute asking for it.
    repo.config(&format!(
        "[filter \"evil\"]\n\tsmudge = touch '{}'\n",
        witness.display()
    ));
    repo.write(".gitattributes", "*.txt filter=evil\n");
    repo.commit_all("attributes");
    let secret = outside.path().join("secret");
    std::fs::write(&secret, "TOKEN\n").unwrap();
    symlink(&secret, repo.git_dir().join("objects/planted")).unwrap();
    let into = tempfile::tempdir().unwrap().keep().join("clone");
    Gix::default().clone_local(&place(&repo), &into).unwrap();
    assert!(!witness.exists(), "something of the source's ran");
    let mut stack = vec![into.join(".git/objects")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let meta = entry.path().symlink_metadata().unwrap();
            assert!(!meta.file_type().is_symlink(), "{}", entry.path().display());
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                assert_eq!(meta.nlink(), 1, "{} is shared", entry.path().display());
            }
        }
    }
    std::fs::remove_dir_all(into.parent().unwrap()).unwrap();
}

#[test]
fn a_clone_of_an_empty_repository_and_of_a_linked_worktree() {
    let git = Gix::default();
    let dir = tempfile::tempdir().unwrap();
    let empty = Repo::init(dir.path());
    let into = tempfile::tempdir().unwrap();
    let clone = into.path().join("empty");
    git.clone_local(&place(&empty), &clone).unwrap();
    let Located::Repo(made) = git.locate(&clone) else {
        panic!("a repository")
    };
    assert_eq!(git.head(&made).unwrap().oid, None);

    let source = tempfile::tempdir().unwrap();
    let repo = Repo::init(source.path());
    repo.write("a.txt", "one\n");
    let first = repo.commit_all("first");
    let parent = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&parent.path().join("wt"), "side", &first);
    let clone = into.path().join("of-linked");
    git.clone_local(&place(&linked), &clone).unwrap();
    assert!(clone.join("a.txt").is_file());
}
