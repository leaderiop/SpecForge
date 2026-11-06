//! `specforge git` where several repositories meet: a repository with no identity, and a machine root.

use super::{committed, git};

/// A new machine has no `user.name`: git makes a name up for a branch's reflog, and so must the verb.
#[test]
fn a_worktree_on_a_new_branch_needs_no_identity() {
    let (project, repo, sha) = committed();
    std::fs::write(repo.git_dir().join("config"), "[core]\n\tbare = false\n").unwrap();
    git(&project, &["branch", "create", "workspace/x"]).exits(0);
    let added = git(
        &project,
        &[
            "worktree",
            "add",
            "wt/deep/repo",
            "--branch",
            "workspace/y",
            "--create",
        ],
    )
    .exits(0)
    .answer();
    assert_eq!(added["branch"], "workspace/y");
    assert_eq!(added["sha"], sha);
}
