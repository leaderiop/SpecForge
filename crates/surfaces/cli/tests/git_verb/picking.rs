//! `specforge git revert -n` and `cherry-pick -n`, staged and left for one commit, and `--quit` / `--abort`.

use super::{committed, git};

#[test]
fn reverts_without_commit_stack_into_one_commit() {
    let (project, repo, _) = committed();
    repo.write("b.txt", "b\n");
    let added = repo.commit_all("add b");
    repo.write("a.txt", "two\n");
    let changed = repo.commit_all("change a");
    for sha in [&changed, &added] {
        let staged = git(&project, &["revert", "--no-commit", sha])
            .exits(0)
            .answer();
        assert_eq!(staged["sha"], serde_json::Value::Null);
    }
    git(&project, &["commit", "-m", "revert: undo two"]).exits(0);
    let log = git(
        &project,
        &["log", "--max", "1", "--pick", ".commits[0].subject"],
    )
    .exits(0);
    assert_eq!(log.stdout, "revert: undo two\n");
    let diff = git(&project, &["diff", "--base", "HEAD~3", "--pick", ".count"]).exits(0);
    assert_eq!(diff.stdout, "0\n", "the tree is the first commit's again");
    assert!(
        !repo.git_dir().join("REVERT_HEAD").exists(),
        "the commit concludes it"
    );
}

#[test]
fn quit_and_abort_end_what_is_in_progress() {
    let (project, repo, _) = committed();
    repo.write("b.txt", "b\n");
    let added = repo.commit_all("add b");
    let refused = git(&project, &["revert", "--abort"]).exits(1);
    assert!(
        refused.stdout.contains("no cherry-pick or revert"),
        "{}",
        refused.stdout
    );
    let quit = git(&project, &["cherry-pick", "--quit"]).exits(0).answer();
    assert_eq!(quit["ended"], false);
    git(&project, &["revert", "-n", &added]).exits(0);
    let aborted = git(&project, &["revert", "--abort", "--format", "human"]).exits(0);
    assert_eq!(aborted.stdout, "");
    assert!(project.root().join("b.txt").is_file());
    let refused = git(&project, &["revert", "HEAD", "--quit"]).exits(2);
    assert!(refused.stderr.contains("--quit"), "{}", refused.stderr);
}

#[test]
fn merge_squash_and_no_commit_leave_the_merge_for_a_commit() {
    let (project, repo, _first, _side) = super::writes::side("s.txt", "side\n");
    repo.write("m.txt", "main\n");
    repo.commit_all("on main");
    let squashed = git(&project, &["merge", "--squash", "side"])
        .exits(0)
        .answer();
    assert_eq!(squashed["sha"], serde_json::Value::Null);
    git(&project, &["commit", "-m", "feat: side"]).exits(0);
    let parents = git(
        &project,
        &["log", "--max", "1", "--pick", ".commits[0].parents"],
    )
    .exits(0);
    assert_eq!(parents.json().as_array().map(Vec::len), Some(1));
    git(&project, &["reset", "HEAD~1", "--hard", "--yes"]).exits(0);
    git(&project, &["merge", "--no-ff", "--no-commit", "side"]).exits(0);
    git(&project, &["commit", "-m", "Merge branch 'side'"]).exits(0);
    let parents = git(
        &project,
        &["log", "--max", "1", "--pick", ".commits[0].parents"],
    )
    .exits(0);
    assert_eq!(parents.json().as_array().map(Vec::len), Some(2));
    assert!(!repo.git_dir().join("MERGE_HEAD").exists());
}

#[test]
fn a_locked_worktree_is_kept_from_removal_until_unlocked() {
    let (project, _repo, _) = committed();
    let at = project.root().join("wt");
    let path = at.to_string_lossy().into_owned();
    git(
        &project,
        &["worktree", "add", &path, "--branch", "wt", "--create"],
    )
    .exits(0);
    let locked = git(&project, &["worktree", "lock", &path, "--reason", "busy"])
        .exits(0)
        .answer();
    assert_eq!(locked["locked"], true);
    let listed = git(
        &project,
        &["worktrees", "--pick", ".worktrees[main=false].locked"],
    )
    .exits(0);
    assert_eq!(listed.stdout, "true\n");
    let again = git(&project, &["worktree", "lock", &path]).exits(1);
    assert!(again.stdout.contains("already locked"), "{}", again.stdout);
    git(&project, &["worktree", "remove", &path, "--force"]).exits(1);
    git(&project, &["worktree", "unlock", &path]).exits(0);
    let not = git(&project, &["worktree", "unlock", &path]).exits(1);
    assert!(not.stdout.contains("is not locked"), "{}", not.stdout);
    git(&project, &["worktree", "remove", &path, "--force"]).exits(0);
    assert!(!at.exists());
}
