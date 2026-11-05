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
