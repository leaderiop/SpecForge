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
