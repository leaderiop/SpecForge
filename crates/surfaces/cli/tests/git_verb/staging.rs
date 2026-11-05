//! `specforge git add`: the index as git's own staging verb leaves it, with no git program.

use super::{committed, git};

#[test]
fn add_stages_the_paths_named_and_answers_the_status() {
    let (project, repo, _) = committed();
    repo.write("a.txt", "changed\n")
        .write("b.txt", "b\n")
        .write("c.txt", "c\n");
    let added = git(&project, &["add", "a.txt", "b.txt"]).exits(0).answer();
    assert_eq!(added["tracked"], serde_json::json!(["a.txt", "b.txt"]));
    assert_eq!(added["untracked"], serde_json::json!(["c.txt"]));
    // silent, as git add is
    let human = git(&project, &["add", "c.txt", "--format", "human"]).exits(0);
    assert_eq!(human.stdout, "");
    let refused = git(&project, &["add", "missing.txt"]).exits(1);
    assert!(
        refused
            .stdout
            .contains("missing.txt did not match any files"),
        "{}",
        refused.stdout
    );
}

#[test]
fn add_all_stages_everything_but_what_except_names() {
    let (project, repo, _) = committed();
    repo.write("spec/plan.spec", "p\n")
        .write("src/b.txt", "b\n");
    let added = git(&project, &["add", "-A", "--except", "spec"])
        .exits(0)
        .answer();
    assert_eq!(added["tracked"], serde_json::json!(["src/b.txt"]));
    assert_eq!(added["untracked"], serde_json::json!(["spec/"]));
    let refused = git(&project, &["add"]).exits(2);
    assert!(refused.stderr.contains("PATH"), "{}", refused.stderr);
}

#[test]
fn rm_takes_files_out_of_the_index_and_off_the_disk() {
    let (project, repo, _) = committed();
    repo.write("spec/a.spec", "a\n").write("spec/b.spec", "b\n");
    repo.commit_all("specs");
    let human = git(
        &project,
        &[
            "rm",
            "-r",
            "-f",
            "--ignore-unmatch",
            "--format",
            "human",
            "spec",
            "gone",
        ],
    )
    .exits(0);
    assert_eq!(human.stdout, "rm 'spec/a.spec'\nrm 'spec/b.spec'\n");
    assert!(!project.root().join("spec").exists());
    let status = git(&project, &["status", "--pick", ".tracked"])
        .exits(0)
        .json();
    assert_eq!(status, serde_json::json!(["spec/a.spec", "spec/b.spec"]));
    let refused = git(&project, &["rm", "gone"]).exits(1);
    assert!(
        refused.stdout.contains("gone did not match any files"),
        "{}",
        refused.stdout
    );
    let cached = git(&project, &["rm", "--cached", "a.txt"])
        .exits(0)
        .answer();
    assert_eq!(cached["removed"], serde_json::json!(["a.txt"]));
    assert!(project.root().join("a.txt").is_file());
}

#[test]
fn restore_takes_their_side_of_a_conflict_or_head_into_both() {
    let (project, repo, _) = committed();
    repo.conflict("c.txt", "base\n", "ours\n", "theirs\n");
    git(&project, &["restore", "--theirs", "c.txt"]).exits(0);
    let read = |rel: &str| std::fs::read_to_string(project.root().join(rel)).unwrap();
    assert_eq!(read("c.txt"), "theirs\n");
    git(&project, &["add", "c.txt"]).exits(0);
    repo.write("a.txt", "changed\n");
    git(&project, &["add", "a.txt"]).exits(0);
    git(
        &project,
        &[
            "restore",
            "--source",
            "HEAD",
            "--staged",
            "--worktree",
            "a.txt",
        ],
    )
    .exits(0);
    assert_eq!(read("a.txt"), "one\n");
    let tracked = git(&project, &["status", "--pick", ".tracked"])
        .exits(0)
        .json();
    assert_eq!(tracked, serde_json::json!(["c.txt"]));
    let refused = git(&project, &["restore", "--theirs", "--staged", "c.txt"]).exits(2);
    assert!(refused.stderr.contains("--staged"), "{}", refused.stderr);
}
