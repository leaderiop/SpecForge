//! The Git port's writes through `Gix` (ADR 0339 D5): staging is libgit2's, the commit object and its ref move gix's.

use specforge_effects::git::{ConflictSide, Git, Recording, Removal, Restoring, Staging};
use specforge_git::Gix;
use specforge_git::testing::Repo;

use super::place;

/// A recording that allows an empty commit.
fn empty() -> Recording {
    Recording {
        allow_empty: true,
        ..Recording::default()
    }
}

/// The paths `repo`'s index holds.
fn indexed(repo: &Repo) -> Vec<String> {
    let index = repo.open().index().unwrap();
    index
        .entries()
        .iter()
        .map(|e| e.path(&index).to_string())
        .collect()
}

#[test]
fn staging_a_directory_keeps_its_ignore_rules_and_a_removal_keeps_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write(".gitignore", "*.log\n").write("keep.txt", "k");
    repo.commit_all("first");
    repo.write("spec/a.spec", "a")
        .write("spec/build.log", "noise");
    let place = place(&repo);
    let staging = Staging {
        add: vec!["spec".into()],
        remove: vec!["keep.txt".into()],
        ..Staging::default()
    };
    Gix::default().stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), [".gitignore", "spec/a.spec"]);
    assert!(
        repo.dir().join("keep.txt").is_file(),
        "the file stays on disk"
    );
    let committed = Gix::default()
        .commit_index(&place, "second\n", &Recording::default())
        .unwrap();
    assert_eq!(committed.branch.as_deref(), Some("main"));
    assert_eq!(committed.files, ["keep.txt", "spec/a.spec"]);
    assert_eq!(repo.sha("main"), committed.sha.as_str());
}

#[test]
fn staging_leaves_out_what_except_names_as_a_negative_pathspec_does() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("spec/old.spec", "o").write("keep.txt", "k");
    repo.commit_all("first");
    repo.delete("spec/old.spec")
        .write("spec/new.spec", "n")
        .write("specs/x.txt", "x")
        .write("src/b.txt", "b");
    let staging = Staging {
        add: vec!["*".into()],
        except: vec!["spec".into()],
        ..Staging::default()
    };
    Gix::default().stage(&place(&repo), &staging).unwrap();
    // spec/ is neither added to nor removed from; specs/ is another directory
    assert_eq!(
        indexed(&repo),
        ["keep.txt", "spec/old.spec", "specs/x.txt", "src/b.txt"]
    );
}

#[test]
fn staging_all_takes_tracked_changes_and_deletions_and_no_untracked_file() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a").write("b.txt", "b");
    repo.commit_all("first");
    repo.write("a.txt", "changed")
        .delete("b.txt")
        .write("new.txt", "n");
    let place = place(&repo);
    let staging = Staging {
        all: true,
        ..Staging::default()
    };
    Gix::default().stage(&place, &staging).unwrap();
    assert_eq!(indexed(&repo), ["a.txt"]);
    let committed = Gix::default()
        .commit_index(&place, "tracked\n", &Recording::default())
        .unwrap();
    assert_eq!(committed.files, ["a.txt", "b.txt"]);
    assert!(
        Gix::default()
            .commit_index(&place, "again\n", &Recording::default())
            .is_err(),
        "nothing staged"
    );
}

/// git's state files of an operation in progress, written as git writes them.
fn in_progress(repo: &Repo, files: &[&str]) {
    for file in files {
        let at = repo.git_dir().join(file);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(at, "x\n").unwrap();
    }
}

/// As git: a commit at a rebase's `edit` stop continues the rebase, which this port does not, so it is refused and
/// the rebase's todo list survives; so does an `am` and a bisect.
#[test]
fn a_write_during_a_rebase_am_or_bisect_is_refused_and_keeps_its_state() {
    for state in [
        "rebase-merge/git-rebase-todo",
        "rebase-apply/applying",
        "BISECT_LOG",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repo::init(dir.path());
        repo.write("a.txt", "a");
        repo.commit_all("first");
        repo.write("a.txt", "b");
        in_progress(&repo, &[state]);
        let place = place(&repo);
        let staging = Staging {
            all: true,
            ..Staging::default()
        };
        let refused = Gix::default().stage(&place, &staging).unwrap_err();
        assert!(refused.0.contains("in progress"), "{state}: {refused}");
        let refused = Gix::default()
            .commit_index(&place, "m\n", &empty())
            .unwrap_err();
        assert!(refused.0.contains("in progress"), "{state}: {refused}");
        assert!(repo.git_dir().join(state).exists(), "{state} survives");
    }
}

/// As git: a stopped `cherry-pick A..B` keeps its `sequencer/`, which a commit here would have ended.
#[test]
fn a_commit_during_a_cherry_pick_of_several_commits_keeps_the_sequencer() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let head = repo.commit_all("first");
    in_progress(&repo, &["sequencer/todo"]);
    std::fs::write(repo.git_dir().join("CHERRY_PICK_HEAD"), format!("{head}\n")).unwrap();
    assert!(
        Gix::default()
            .commit_index(&place(&repo), "m\n", &empty())
            .is_err()
    );
    assert!(repo.git_dir().join("sequencer/todo").exists());
    assert!(repo.git_dir().join("CHERRY_PICK_HEAD").exists());
}

/// As git ("Cannot do a soft reset in the middle of a merge"): `reset --soft` refuses while `MERGE_HEAD` is there.
#[test]
fn a_soft_reset_in_the_middle_of_a_merge_is_refused() {
    use specforge_effects::git::{ResetMode, Sha};
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.write("a.txt", "b");
    let second = repo.commit_all("second");
    std::fs::write(repo.git_dir().join("MERGE_HEAD"), format!("{first}\n")).unwrap();
    let refused = Gix::default().reset(&place(&repo), &Sha::new(first), ResetMode::Soft);
    assert!(refused.is_err());
    assert_eq!(repo.sha("HEAD"), second);
}

/// A commit that concludes a merge removes the merge's own files and nothing else: a bisect's `BISECT_START` stays.
#[test]
fn a_commit_concluding_a_merge_removes_only_the_merges_files() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.write("a.txt", "b");
    repo.commit_all("second");
    std::fs::write(repo.git_dir().join("MERGE_HEAD"), format!("{first}\n")).unwrap();
    in_progress(&repo, &["MERGE_MSG", "BISECT_START"]);
    let committed = Gix::default().commit_index(&place(&repo), "merged\n", &Recording::default());
    assert!(committed.is_ok(), "{committed:?}");
    assert!(!repo.git_dir().join("MERGE_HEAD").exists());
    assert!(!repo.git_dir().join("MERGE_MSG").exists());
    assert!(repo.git_dir().join("BISECT_START").exists());
}

/// As git, even with `--force`: a branch a linked worktree has checked out is not deleted.
#[test]
fn a_branch_another_worktree_has_checked_out_is_not_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    let linked = tempfile::tempdir().unwrap();
    repo.linked_worktree(&linked.path().join("wt"), "side", &first);
    let refused = Gix::default().branch_delete(&place(&repo), "side");
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(repo.sha("side"), first);
}

/// A fast-forward is all or nothing: when the branch cannot move (here another git holds its lock, as one moving it
/// would), no file and no index entry changes, so the checkout never runs ahead of the ref.
#[test]
fn a_fast_forward_whose_branch_cannot_move_changes_no_file() {
    use specforge_effects::git::Sha;
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a");
    let first = repo.commit_all("first");
    repo.branch("side", &first).head_on("side");
    repo.write("a.txt", "b");
    let side = repo.commit_all("second");
    repo.head_on("main");
    repo.write("a.txt", "a");
    repo.stage_all();
    let lock = repo.git_dir().join("refs/heads/main.lock");
    std::fs::write(&lock, "").unwrap();
    let merging = specforge_effects::git::Merging {
        message: "m\n".into(),
        ..Default::default()
    };
    let merged = Gix::default().merge(&place(&repo), &Sha::new(side), &merging);
    assert!(merged.is_err(), "{merged:?}");
    assert_eq!(repo.sha("HEAD"), first);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "a"
    );
    assert_eq!(
        indexed_blob(&repo, "a.txt"),
        repo.sha(&format!("{first}:a.txt"))
    );
}

/// The blob the index holds for `path`.
fn indexed_blob(repo: &Repo, path: &str) -> String {
    let git = repo.open();
    let index = git.index_or_empty().unwrap();
    let entry = index.entry_by_path(path.into()).unwrap();
    entry.id.to_string()
}

/// `git add` refuses a path that matches nothing, and an ignored one unless `--force`; a deleted tracked file is
/// staged as its removal.
#[test]
fn staging_refuses_a_path_matching_nothing_and_an_ignored_one_without_force_as_git_add_does() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write(".gitignore", "*.log\nbuild/\n")
        .write("gone.txt", "g");
    repo.commit_all("first");
    repo.delete("gone.txt")
        .write("run.log", "l")
        .write("build/out.txt", "o");
    let place = place(&repo);
    let add = |paths: &[&str], force| Staging {
        add: paths.iter().map(|p| (*p).to_string()).collect(),
        force,
        ..Staging::default()
    };
    let refused = Gix::default()
        .stage(&place, &add(&["gone.txt", "missing.txt"], false))
        .unwrap_err();
    assert!(
        refused.0.contains("missing.txt did not match any files"),
        "{refused}"
    );
    for ignored in ["run.log", "build"] {
        let refused = Gix::default()
            .stage(&place, &add(&[ignored], false))
            .unwrap_err();
        assert!(refused.0.contains("is ignored"), "{ignored}: {refused}");
    }
    assert_eq!(
        indexed(&repo),
        [".gitignore", "gone.txt"],
        "a refusal stages nothing"
    );
    Gix::default()
        .stage(&place, &add(&["gone.txt", "run.log", "build"], true))
        .unwrap();
    assert_eq!(indexed(&repo), [".gitignore", "build/out.txt", "run.log"]);
}

/// libgit2 runs no filter driver, so a file whose attributes name one (`filter=lfs`) is refused, naming the driver,
/// rather than staged raw, whether it is named, under a directory named, or a tracked change `--all` takes.
#[test]
fn staging_a_file_whose_attributes_name_a_filter_driver_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write(
        ".gitattributes",
        "*.bin filter=lfs diff=lfs merge=lfs -text\n",
    )
    .write("old.bin", "tracked");
    repo.commit_all("first");
    repo.write("new.bin", "raw").write("old.bin", "changed");
    let place = place(&repo);
    for staging in [
        Staging {
            add: vec!["new.bin".into()],
            ..Staging::default()
        },
        Staging {
            add: vec!["*".into()],
            ..Staging::default()
        },
        Staging {
            all: true,
            ..Staging::default()
        },
    ] {
        let refused = Gix::default().stage(&place, &staging).unwrap_err();
        assert!(
            refused.0.contains("filter driver `lfs`") && refused.0.contains("stage it with git"),
            "{staging:?}: {refused}"
        );
    }
    assert_eq!(
        indexed_blob(&repo, "old.bin"),
        repo.open()
            .rev_parse_single("HEAD:old.bin")
            .unwrap()
            .to_string(),
        "nothing was staged"
    );
}

/// A path the config names is interpolated as git interpolates one: `~user/` is that user's home, a relative path
/// stays as written, and an unset key is none.
#[cfg(unix)]
#[test]
fn a_config_path_is_interpolated_as_git_does() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    let place = place(&repo);
    assert_eq!(
        Gix::default()
            .config_path(&place, "core.hooksPath")
            .unwrap(),
        None
    );
    repo.config("[core]\n\thooksPath = rel/hooks");
    assert_eq!(
        Gix::default()
            .config_path(&place, "core.hooksPath")
            .unwrap(),
        Some(std::path::PathBuf::from("rel/hooks"))
    );
    let (Ok(user), Some(home)) = (std::env::var("USER"), std::env::home_dir()) else {
        return; // no user to name here
    };
    repo.config(&format!("[core]\n\thooksPath = ~{user}/hooks"));
    assert_eq!(
        Gix::default()
            .config_path(&place, "core.hooksPath")
            .unwrap(),
        Some(home.join("hooks"))
    );
}

#[test]
fn removing_takes_a_directory_with_r_and_its_files_off_the_disk() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("keep.txt", "k")
        .write("spec/a.spec", "a")
        .write("spec/deep/b.spec", "b");
    repo.commit_all("first");
    let place = place(&repo);
    let git = Gix::default();
    let refused = git
        .remove(&place, &["spec".into()], Removal::default())
        .unwrap_err();
    assert!(refused.0.contains("recursively without -r"), "{refused}");
    let refused = git
        .remove(&place, &["gone".into()], Removal::default())
        .unwrap_err();
    assert!(
        refused.0.contains("gone did not match any files"),
        "{refused}"
    );
    repo.write("spec/untracked.txt", "u");
    let recursive = Removal {
        recursive: true,
        ..Removal::default()
    };
    let removed = git.remove(
        &place,
        &["spec".into(), "gone".into()],
        Removal {
            ignore_unmatch: true,
            ..recursive
        },
    );
    assert_eq!(removed.unwrap(), ["spec/a.spec", "spec/deep/b.spec"]);
    assert_eq!(indexed(&repo), ["keep.txt"]);
    assert!(
        !repo.dir().join("spec/deep").exists(),
        "an emptied directory goes"
    );
    assert!(
        repo.dir().join("spec/untracked.txt").is_file(),
        "an untracked file stays"
    );
}

#[test]
fn a_glob_pathspec_removes_and_restores_the_files_it_names_at_any_depth() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.spec", "base\n")
        .write("sub/b.spec", "base\n")
        .write("keep.txt", "k\n");
    repo.commit_all("first");
    repo.write("a.spec", "changed\n")
        .write("sub/b.spec", "changed\n")
        .write("new.spec", "n\n")
        .write("src/x.js", "x\n");
    let place = place(&repo);
    let git = Gix::default();
    let everything = Staging {
        add: vec!["*".into()],
        ..Staging::default()
    };
    git.stage(&place, &everything).unwrap();
    let spec = vec![":(glob)**/*.spec".to_string()];
    let forced = Removal {
        recursive: true,
        force: true,
        ..Removal::default()
    };
    let removed = git.remove(&place, &spec, forced).unwrap();
    assert_eq!(removed, ["a.spec", "new.spec", "sub/b.spec"]);
    assert_eq!(indexed(&repo), ["keep.txt", "src/x.js"]);
    let from_head = Restoring {
        source: Some(specforge_effects::git::Sha::new(repo.sha("HEAD"))),
        staged: true,
        worktree: true,
        ..Restoring::default()
    };
    git.restore(&place, &spec, &from_head).unwrap();
    assert_eq!(
        indexed(&repo),
        ["a.spec", "keep.txt", "src/x.js", "sub/b.spec"]
    );
    let read = |rel: &str| std::fs::read_to_string(repo.dir().join(rel)).unwrap();
    assert_eq!(
        (read("a.spec"), read("sub/b.spec")),
        ("base\n".into(), "base\n".into())
    );
    assert!(!repo.dir().join("new.spec").exists());
    let none = vec![":(glob)**/*.md".to_string()];
    let refused = git.restore(&place, &none, &from_head).unwrap_err();
    assert!(refused.0.contains("did not match any files"), "{refused}");
}

/// `a.txt` at the top, `dir/keep`, `dir/x` and `dir/sub/y` under `dir`, all committed.
fn tree_with_dir() -> (tempfile::TempDir, Repo) {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a\n")
        .write("dir/keep", "k\n")
        .write("dir/x", "x\n")
        .write("dir/sub/y", "y\n");
    repo.commit_all("first");
    (dir, repo)
}

#[test]
fn removing_combines_the_pathspecs_an_exclude_taking_back_what_an_include_names() {
    let (_dir, repo) = tree_with_dir();
    let place = place(&repo);
    let forced = Removal {
        recursive: true,
        force: true,
        ..Removal::default()
    };
    let removed = Gix::default()
        .remove(&place, &["dir".into(), ":!dir/keep".into()], forced)
        .unwrap();
    assert_eq!(removed, ["dir/sub/y", "dir/x"]);
    assert_eq!(indexed(&repo), ["a.txt", "dir/keep"]);
    assert!(repo.dir().join("dir/keep").is_file());
}

#[test]
fn removing_the_whole_tree_needs_r_whether_named_or_left_by_an_exclude() {
    let (_dir, repo) = tree_with_dir();
    let place = place(&repo);
    let git = Gix::default();
    for spec in [".", ":!a.txt"] {
        let refused = git
            .remove(&place, &[spec.into()], Removal::default())
            .unwrap_err();
        assert!(
            refused.0.contains("not removing . recursively without -r"),
            "{spec}: {refused}"
        );
    }
    let refused = git
        .remove(
            &place,
            &["nomatch".into(), ":!dir".into()],
            Removal::default(),
        )
        .unwrap_err();
    assert!(
        refused.0.contains("nomatch did not match any files"),
        "{refused}"
    );
    assert_eq!(indexed(&repo).len(), 4, "nothing removed");
    let recursive = Removal {
        recursive: true,
        ..Removal::default()
    };
    let removed = git.remove(&place, &[":!a.txt".into()], recursive).unwrap();
    assert_eq!(removed, ["dir/keep", "dir/sub/y", "dir/x"]);
    assert_eq!(indexed(&repo), ["a.txt"]);
}

#[test]
fn restoring_combines_the_pathspecs_an_exclude_keeping_its_file_as_it_is() {
    let (_dir, repo) = tree_with_dir();
    for rel in ["dir/keep", "dir/x", "dir/sub/y"] {
        repo.write(rel, "changed\n");
    }
    let place = place(&repo);
    Gix::default()
        .restore(
            &place,
            &["dir".into(), ":!dir/keep".into()],
            &Restoring::default(),
        )
        .unwrap();
    let read = |rel: &str| std::fs::read_to_string(repo.dir().join(rel)).unwrap();
    assert_eq!(
        (read("dir/keep"), read("dir/x"), read("dir/sub/y")),
        ("changed\n".into(), "x\n".into(), "y\n".into())
    );
    Gix::default()
        .restore(
            &place,
            &["dir".into(), ":!dir".into()],
            &Restoring::default(),
        )
        .unwrap();
    assert_eq!(
        read("dir/keep"),
        "changed\n",
        "nothing named: nothing restored"
    );
}

#[test]
fn removing_a_changed_file_needs_force_and_cached_keeps_it_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a").write("b.txt", "b");
    repo.commit_all("first");
    repo.write("a.txt", "changed");
    let place = place(&repo);
    let git = Gix::default();
    let refused = git
        .remove(&place, &["a.txt".into()], Removal::default())
        .unwrap_err();
    assert!(refused.0.contains("local modifications"), "{refused}");
    let cached = Removal {
        cached: true,
        ..Removal::default()
    };
    // the index still matches HEAD, so --cached loses nothing
    assert_eq!(
        git.remove(&place, &["a.txt".into()], cached).unwrap(),
        ["a.txt"]
    );
    assert!(repo.dir().join("a.txt").is_file());
    repo.write("b.txt", "changed");
    let forced = Removal {
        force: true,
        ..Removal::default()
    };
    assert_eq!(
        git.remove(&place, &["b.txt".into()], forced).unwrap(),
        ["b.txt"]
    );
    assert!(!repo.dir().join("b.txt").exists());
    assert_eq!(indexed(&repo), Vec::<String>::new());
}

#[test]
fn removing_a_conflicted_path_takes_every_stage() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("keep.txt", "k");
    repo.commit_all("first");
    repo.conflict("c.txt", "base\n", "ours\n", "theirs\n");
    let removed = Gix::default()
        .remove(&place(&repo), &["c.txt".into()], Removal::default())
        .unwrap();
    assert_eq!(removed, ["c.txt"]);
    assert_eq!(indexed(&repo), ["keep.txt"]);
}

#[cfg(unix)]
#[test]
fn removing_never_follows_a_link_out_of_the_worktree() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("x.txt"), "precious").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("d/x.txt", "x");
    repo.commit_all("first");
    std::fs::remove_dir_all(repo.dir().join("d")).unwrap();
    std::os::unix::fs::symlink(outside.path(), repo.dir().join("d")).unwrap();
    let refused = Gix::default()
        .remove(&place(&repo), &["d/x.txt".into()], Removal::default())
        .unwrap_err();
    assert!(refused.0.contains("beyond a symbolic link"), "{refused}");
    assert!(outside.path().join("x.txt").is_file());
    assert_eq!(indexed(&repo), ["d/x.txt"], "nothing changed");
}

#[test]
fn restoring_takes_a_side_of_a_conflict_or_head_into_the_index_and_the_files() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a\n");
    repo.commit_all("first");
    repo.conflict("c.txt", "base\n", "ours\n", "theirs\n");
    let place = place(&repo);
    let git = Gix::default();
    let theirs = Restoring {
        side: Some(ConflictSide::Theirs),
        ..Restoring::default()
    };
    git.restore(&place, &["c.txt".into()], &theirs).unwrap();
    let read = |rel: &str| std::fs::read_to_string(repo.dir().join(rel)).unwrap();
    assert_eq!(read("c.txt"), "theirs\n");
    // the conflict stays for the add that resolves it, as git checkout --theirs leaves it
    let status = git.status(&place).unwrap();
    assert_eq!(
        status.entries[0].kind,
        specforge_effects::git::GitEntryKind::Unmerged
    );
    git.stage(
        &place,
        &Staging {
            add: vec!["c.txt".into()],
            ..Staging::default()
        },
    )
    .unwrap();
    assert_eq!(indexed(&repo), ["a.txt", "c.txt"]);
    let resolved = git.status(&place).unwrap();
    assert_eq!(
        resolved.entries[0].kind,
        specforge_effects::git::GitEntryKind::Changed
    );
    repo.write("a.txt", "changed\n");
    git.stage(
        &place,
        &Staging {
            add: vec!["a.txt".into()],
            ..Staging::default()
        },
    )
    .unwrap();
    let both = Restoring {
        source: Some(specforge_effects::git::Sha::new(repo.sha("HEAD"))),
        staged: true,
        worktree: true,
        ..Restoring::default()
    };
    git.restore(&place, &["a.txt".into()], &both).unwrap();
    assert_eq!(read("a.txt"), "a\n");
    assert!(
        git.status(&place)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.path != "a.txt")
    );
}

#[test]
fn a_commit_of_some_paths_leaves_the_rest_staged_and_takes_an_author() {
    use specforge_effects::git::Sha;
    let dir = tempfile::tempdir().unwrap();
    let repo = Repo::init(dir.path());
    repo.write("a.txt", "a").write("b.txt", "b");
    let first = repo.commit_all("first");
    repo.write("a.txt", "a2")
        .write("b.txt", "b2")
        .write("d/c.txt", "c");
    repo.stage_all();
    let place = place(&repo);
    let only = Recording {
        only: vec!["a.txt".into(), "d".into()],
        author_of: Some(Sha::new(first.as_str())),
        ..Recording::default()
    };
    let committed = Gix::default()
        .commit_index(&place, "some\n", &only)
        .unwrap();
    assert_eq!(committed.files, ["a.txt", "d/c.txt"]);
    let status = Gix::default().status(&place).unwrap();
    let staged: Vec<_> = status.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(staged, ["b.txt"], "b.txt stays staged");
    let log = repo.open();
    let head = log.head_commit().unwrap();
    let author = head.author().unwrap();
    let theirs = log
        .find_commit(gix::ObjectId::from_hex(first.as_bytes()).unwrap())
        .unwrap();
    assert_eq!(
        author.time,
        theirs.author().unwrap().time,
        "the author's time is the reused commit's"
    );
}
