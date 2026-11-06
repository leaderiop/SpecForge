//! `specforge git repos <dir>`: the repositories directly in a directory, what a prompt would loop over a glob for.

use serde_json::json;
use specforge_git::testing::Repo;

use super::{committed, git};
use crate::support::TestProject;

fn repo_in(dir: &std::path::Path, dirty: bool) {
    std::fs::create_dir_all(dir).unwrap();
    let repo = Repo::init(dir);
    repo.write("a.txt", "a\n");
    repo.commit_all("first");
    if dirty {
        repo.write("a.txt", "b\n");
    }
}

fn workspace(project: &TestProject) -> std::path::PathBuf {
    let wk = project.root().join("wk");
    repo_in(&wk.join("clean"), false);
    repo_in(&wk.join("dirty"), true);
    std::fs::create_dir_all(wk.join("plain")).unwrap();
    std::fs::write(wk.join("file"), "f\n").unwrap();
    std::fs::create_dir_all(wk.join("broken/.git")).unwrap();
    std::fs::create_dir_all(wk.join("gone")).unwrap();
    std::fs::write(wk.join("gone/.git"), "gitdir: /nonexistent\n").unwrap();
    wk
}

#[test]
fn repos_lists_the_children_with_a_git_entry_sorted_and_judged() {
    let (project, _, _) = committed();
    let wk = workspace(&project);
    let top = project.root().canonicalize().unwrap();
    let answer = git(&project, &["repos", "wk"]).exits(0).json();
    let names: Vec<_> = answer["repos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names, ["broken", "clean", "dirty", "gone"], "{answer}");
    let row = |name: &str| {
        answer["repos"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .cloned()
            .unwrap()
    };
    assert_eq!(row("clean")["clean"], true);
    assert_eq!(row("clean")["linked"], false);
    assert_eq!(row("clean")["unreadable"], false);
    assert_eq!(
        row("clean")["path"],
        json!(format!("{}/wk/clean", top.display()))
    );
    assert_eq!(row("dirty")["clean"], false);
    assert_eq!(row("dirty")["unreadable"], false);
    // Fail closed: what cannot be read is neither clean nor anything else.
    for broken in ["broken", "gone"] {
        assert_eq!(row(broken)["clean"], false, "{broken}");
        assert_eq!(row(broken)["unreadable"], true, "{broken}");
    }

    // A worktree of a repository in the roots is one, and it reads as linked.
    git(
        &project,
        &[
            "--in", "wk/clean", "worktree", "add", "../wt", "--branch", "w", "--create",
        ],
    )
    .exits(0);
    let linked = git(
        &project,
        &["repos", "wk", "--pick", ".repos[linked=true].name"],
    )
    .exits(0);
    assert_eq!(linked.stdout.trim(), "wt");
    let dirty = git(
        &project,
        &["repos", "wk", "--pick", ".repos[clean=false].name"],
    )
    .exits(0);
    assert_eq!(dirty.stdout.trim().replace('\n', " "), "broken dirty gone");
    let count = git(
        &project,
        &["repos", "wk", "--pick", ".repos[].name", "--count"],
    )
    .exits(0);
    assert_eq!(count.stdout.trim(), "5");
    assert!(wk.exists());
}

#[test]
fn a_planted_git_reads_as_unreadable_and_lets_nothing_out() {
    let (project, _, _) = committed();
    let outside = tempfile::tempdir().unwrap();
    repo_in(outside.path(), true);
    let fake = project.root().join("wk/fake");
    std::fs::create_dir_all(&fake).unwrap();
    std::fs::write(
        fake.join(".git"),
        format!("gitdir: {}/.git\n", outside.path().display()),
    )
    .unwrap();
    let answer = git(&project, &["repos", "wk"]).exits(0).json();
    assert_eq!(answer["repos"][0]["name"], "fake", "{answer}");
    assert_eq!(answer["repos"][0]["clean"], false, "{answer}");
    assert_eq!(answer["repos"][0]["unreadable"], true, "{answer}");
}

#[cfg(unix)]
#[test]
fn a_child_link_out_of_the_roots_is_unreadable() {
    let (project, _, _) = committed();
    let outside = tempfile::tempdir().unwrap();
    repo_in(outside.path(), false);
    std::fs::create_dir_all(project.root().join("wk")).unwrap();
    std::os::unix::fs::symlink(outside.path(), project.root().join("wk/out")).unwrap();
    let answer = git(&project, &["repos", "wk"]).exits(0).json();
    assert_eq!(answer["repos"][0]["name"], "out", "{answer}");
    assert_eq!(answer["repos"][0]["unreadable"], true, "{answer}");
    assert_eq!(answer["repos"][0]["clean"], false, "{answer}");
}

#[test]
fn repos_answers_empty_and_refuses_what_no_root_holds() {
    let (project, _, _) = committed();
    std::fs::create_dir_all(project.root().join("empty")).unwrap();
    let none = git(&project, &["repos", "empty", "--pick", ".repos[].name"]).exits(0);
    assert_eq!(none.stdout.trim(), "");
    let count = git(
        &project,
        &["repos", "empty", "--pick", ".repos[].name", "--count"],
    )
    .exits(0);
    assert_eq!(count.stdout.trim(), "0");
    let outside = tempfile::tempdir().unwrap();
    let refused = git(&project, &["repos", outside.path().to_str().unwrap()]).exits(1);
    assert_eq!(refused.json()["code"], "outside_root", "{}", refused.stdout);
    let missing = git(&project, &["repos", "nope"]).exits(1);
    assert!(missing.stdout.contains("nope"), "{}", missing.stdout);
}
