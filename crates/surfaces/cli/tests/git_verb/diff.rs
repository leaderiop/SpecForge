//! `specforge git diff`: the files two sides differ by, their line counts and totals, their patches, and the summary
//! git's `--stat` prints.

use serde_json::json;
use specforge_git::testing::Repo;

use super::{committed, git};
use crate::support::TestProject;

/// A project on branch `feature` off `main`: `a.txt` changed, `c.txt` added, `bin.dat` binary.
fn feature() -> (TestProject, Repo) {
    let (project, repo, first) = committed();
    repo.branch("feature", &first).head_on("feature");
    repo.write("a.txt", "one\nmore\n")
        .write("c.txt", "c\n")
        .write("bin.dat", "\0\u{1}\u{2}");
    repo.commit_all("feature");
    (project, repo)
}

#[test]
fn diff_lists_the_files_a_branch_changed_since_its_base() {
    let (project, _repo) = feature();
    assert_eq!(
        git(
            &project,
            &["diff", "--base", "main", "--pick", ".files[].path"]
        )
        .exits(0)
        .stdout,
        "a.txt\nbin.dat\nc.txt\n"
    );
    let diff = git(&project, &["diff", "--base", "main"]).exits(0).answer();
    assert_eq!(diff["count"], 3);
    assert_eq!(
        diff["files"],
        json!([
            {"path": "a.txt", "old_path": null, "status": "modified", "added": 1, "removed": 0, "binary": false, "spec": false, "spec_only": false, "spec_removed": false},
            {"path": "bin.dat", "old_path": null, "status": "added", "added": 0, "removed": 0, "binary": true, "spec": false, "spec_only": false, "spec_removed": false},
            {"path": "c.txt", "old_path": null, "status": "added", "added": 1, "removed": 0, "binary": false, "spec": false, "spec_only": false, "spec_removed": false},
        ])
    );
}

#[test]
fn diff_gives_the_patch_of_a_text_file() {
    let (project, _repo) = feature();
    assert_eq!(
        git(
            &project,
            &[
                "diff",
                "--base",
                "main",
                "--patch",
                "--pick",
                ".files[0].patch",
                "--",
                "a.txt"
            ]
        )
        .exits(0)
        .stdout,
        "@@ -1,1 +1,2 @@\n one\n+more\n\n"
    );
}

#[test]
fn diff_totals_its_lines_and_prints_them_as_git_stat_does() {
    let (project, _repo) = feature();
    let diff = git(&project, &["diff", "--base", "main"]).exits(0).answer();
    assert_eq!((&diff["added"], &diff["removed"]), (&json!(2), &json!(0)));
    let human = |args: &[&str]| {
        let mut all = vec!["diff", "--base", "main", "--format", "human"];
        all.extend_from_slice(args);
        git(&project, &all).exits(0).stdout
    };
    assert_eq!(
        human(&["--shortstat"]),
        " 3 files changed, 2 insertions(+)\n"
    );
    assert_eq!(
        human(&["--stat"]),
        "1\t0\ta.txt\n-\t-\tbin.dat\n1\t0\tc.txt\n 3 files changed, 2 insertions(+)\n"
    );
    assert_eq!(
        human(&["--patch", "--", "a.txt", "c.txt"]),
        "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,1 +1,2 @@\n one\n+more\n\
         diff --git a/c.txt b/c.txt\n--- /dev/null\n+++ b/c.txt\n@@ -1,0 +1,1 @@\n+c\n"
    );
}

#[test]
fn diff_merge_base_compares_from_where_the_two_meet() {
    let (project, repo, first) = committed();
    repo.write("m.txt", "main moved on\n");
    repo.commit_all("main");
    repo.branch("feature", &first).head_on("feature");
    repo.delete("m.txt").write("c.txt", "c\n");
    repo.commit_all("feature");
    let paths = |args: &[&str]| {
        let mut all = vec!["diff", "--base", "main"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--pick", ".files[].path"]);
        git(&project, &all).exits(0).stdout
    };
    assert_eq!(paths(&[]), "c.txt\nm.txt\n");
    assert_eq!(paths(&["--merge-base"]), "c.txt\n");
    let refused = git(&project, &["diff", "--merge-base"]).exits(2);
    assert!(refused.stderr.contains("--base"), "{}", refused.stderr);
}

#[test]
fn diff_compares_with_the_index_and_the_worktree() {
    let (project, repo, _) = committed();
    repo.write("a.txt", "staged\n").stage(&["a.txt"]);
    repo.write("a.txt", "on disk\n").write("untracked.txt", "u");
    let side = |args: &[&str]| {
        let mut all = vec!["diff"];
        all.extend_from_slice(args);
        all.extend_from_slice(&["--patch", "--pick", ".files"]);
        git(&project, &all).exits(0).json()
    };
    assert_eq!(
        side(&["--staged"])[0]["patch"],
        "@@ -1,1 +1,1 @@\n-one\n+staged\n"
    );
    // the untracked file is in no diff, as git leaves it out
    let worktree = side(&[]);
    assert_eq!(worktree.as_array().map(Vec::len), Some(1), "{worktree}");
    assert_eq!(worktree[0]["patch"], "@@ -1,1 +1,1 @@\n-one\n+on disk\n");
    assert_eq!(side(&["--worktree"]), worktree);
}

#[test]
fn diff_names_a_rename_and_its_old_path() {
    let (project, repo, _) = committed();
    repo.write("a.txt", "one\ntwo\nthree\nfour\n");
    repo.commit_all("longer");
    repo.delete("a.txt")
        .write("moved.txt", "one\ntwo\nthree\nfour\n");
    repo.commit_all("moved");
    assert_eq!(
        git(&project, &["diff", "--base", "HEAD~1", "--pick", ".files"])
            .exits(0)
            .json(),
        json!([{"path": "moved.txt", "old_path": "a.txt", "status": "renamed", "spec": false, "spec_only": false, "spec_removed": false, "added": 0, "removed": 0, "binary": false}])
    );
}

/// Added, modified, deleted and renamed on each side of the spec root, read end to end through `--pick`'s selectors.
#[test]
fn diff_flags_spec_files_on_each_side() {
    let (project, repo, first) = committed();
    let config = serde_json::json!({"name": "demo", "version": "0.1.0", "spec_root": "spec"});
    repo.write("specforge.json", config.to_string())
        .write("spec/kept.spec", "spec kept \"K\" {}\n")
        .write("spec/gone.spec", "spec gone \"G\" {}\n")
        .write("spec/moved.spec", "moved within\nline\nline\nline\n")
        .write("spec/out.spec", "moved out\nline\nline\nline\n")
        .write("src/in.txt", "moved in\nline\nline\nline\n")
        .write("src/code.txt", "code\n");
    let base = repo.commit_all("base");
    assert_ne!(base, first);
    std::fs::rename(
        project.root().join("spec/moved.spec"),
        project.root().join("spec/there.spec"),
    )
    .unwrap();
    std::fs::rename(
        project.root().join("spec/out.spec"),
        project.root().join("src/out.spec"),
    )
    .unwrap();
    std::fs::rename(
        project.root().join("src/in.txt"),
        project.root().join("spec/in.txt"),
    )
    .unwrap();
    std::fs::remove_file(project.root().join("spec/gone.spec")).unwrap();
    repo.write("spec/kept.spec", "spec kept \"Kept\" {}\n")
        .write("spec/new.spec", "spec new \"N\" {}\n")
        .write("src/code.txt", "changed\n")
        .write("specforge-harness", "ledger\n");
    repo.commit_all("change");
    let pick = |selector: &str| {
        let wanted = format!(".files[{selector}].path");
        let run = git(
            &project,
            &["diff", "--base", &base, "--head", "HEAD", "--pick", &wanted],
        )
        .exits(0);
        let mut lines: Vec<String> = run.stdout.lines().map(str::to_string).collect();
        lines.sort();
        lines
    };
    assert_eq!(
        pick("spec=true"),
        [
            "spec/gone.spec",
            "spec/in.txt",
            "spec/kept.spec",
            "spec/new.spec",
            "spec/there.spec",
            "specforge-harness",
        ]
    );
    assert_eq!(
        pick("spec_removed=true"),
        ["spec/gone.spec", "spec/there.spec", "src/out.spec"]
    );
    assert_eq!(
        pick("spec_only=false"),
        ["spec/in.txt", "src/code.txt", "src/out.spec"]
    );
}
