//! `--except` agrees with git's `:!<path>`: it judges files, a rename is two sides, and it is read from the current
//! directory. `--except-spec` names the project's spec files, wherever the verb is typed.

use serde_json::json;

use super::{committed, git};
use crate::support::Cli;

fn status(project: &crate::support::TestProject, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["status"];
    all.extend_from_slice(args);
    git(project, &all).exits(0).json()
}

/// `git status -- ':!spec'` reads `D  src/b.txt` after a rename into `spec`: with one side cut off there is no rename.
#[test]
fn a_rename_is_left_out_only_when_both_sides_are() {
    let (project, repo, _) = committed();
    repo.write("src/b.txt", "the same text, long enough to be a rename\n");
    repo.commit_all("b");
    std::fs::create_dir_all(project.root().join("spec")).unwrap();
    std::fs::rename(
        project.root().join("src/b.txt"),
        project.root().join("spec/b.txt"),
    )
    .unwrap();
    git(&project, &["add", "-A"]).exits(0);

    let moved = status(&project, &[]);
    assert_eq!(moved["entries"][0]["kind"], "renamed", "{moved}");

    let cut = status(&project, &["--except", "spec"]);
    assert_eq!(cut["clean"], false, "{cut}");
    assert_eq!(cut["tracked"], json!(["src/b.txt"]));
    assert_eq!(cut["entries"][0]["index"], "deleted", "{cut}");

    let cut = status(&project, &["--except", "src"]);
    assert_eq!(cut["clean"], false, "{cut}");
    assert_eq!(cut["tracked"], json!(["spec/b.txt"]));
    assert_eq!(cut["entries"][0]["index"], "added", "{cut}");

    let both = status(&project, &["--except", "src", "--except", "spec"]);
    assert_eq!(both["clean"], true, "{both}");
}

/// git collapses an untracked directory only for showing it: `:!newd/a` leaves nothing of a `newd` holding only `a`.
#[test]
fn an_untracked_directory_is_judged_by_its_files() {
    let (project, repo, _) = committed();
    repo.write("newd/a", "a\n");
    assert_eq!(status(&project, &[])["untracked"], json!(["newd/"]));
    let only = status(&project, &["--except", "newd/a"]);
    assert_eq!(only["clean"], true, "{only}");

    repo.write("newd/b", "b\n");
    let other = status(&project, &["--except", "newd/a"]);
    assert_eq!(other["clean"], false, "{other}");
    assert_eq!(other["untracked"], json!(["newd/b"]));
}

/// As git's own `:!<path>`, `--except` is read from the current directory, while the answer's paths are from the top.
#[test]
fn except_reads_from_the_current_directory() {
    let (project, repo, _) = committed();
    repo.write("spec/a.spec", "a\n").write("sub/keep", "k\n");
    repo.commit_all("tracked");
    repo.write("spec/a.spec", "b\n").write("sub/keep", "l\n");
    let from_sub = |args: &[&str]| {
        let mut all = vec!["git", "status"];
        all.extend_from_slice(args);
        Cli::on(&project)
            .in_dir(project.root().join("sub"))
            .run(&all)
            .exits(0)
            .json()
    };
    assert_eq!(
        from_sub(&["--except", "spec"])["tracked"],
        json!(["spec/a.spec", "sub/keep"]),
    );
    assert_eq!(
        from_sub(&["--except", "../spec"])["tracked"],
        json!(["sub/keep"])
    );
    assert_eq!(
        from_sub(&["--except", "keep"])["tracked"],
        json!(["spec/a.spec"])
    );
}

/// `--except-spec` is what the project calls a spec file, whichever directory it is typed in, and however deep the
/// project lies in its repository.
#[test]
fn except_spec_names_the_projects_spec_files_from_anywhere() {
    let (project, repo, _) = committed();
    repo.write("spec/a.spec", "a\n").write("sub/keep", "k\n");
    repo.commit_all("tracked");
    repo.write("spec/a.spec", "b\n")
        .write("spec/new.spec", "n\n")
        .write("sub/keep", "l\n");
    for dir in [project.root().to_path_buf(), project.root().join("sub")] {
        let run = Cli::on(&project)
            .in_dir(&dir)
            .run(&["git", "status", "--except-spec"])
            .exits(0)
            .json();
        assert_eq!(run["tracked"], json!(["sub/keep"]), "{}", dir.display());
        assert_eq!(run["untracked"], json!([]), "{}", dir.display());
    }
    let added = Cli::on(&project)
        .in_dir(project.root().join("sub"))
        .run(&["git", "add", "-A", "--except-spec"])
        .exits(0)
        .answer();
    assert_eq!(added["tracked"], json!(["spec/a.spec", "sub/keep"]));
    assert_eq!(added["untracked"], json!(["spec/new.spec"]));
    let staged = status(&project, &[]);
    let entry = |path: &str| {
        staged["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["path"] == path)
            .cloned()
            .unwrap()
    };
    assert!(entry("spec/a.spec")["index"].is_null(), "{staged}");
    assert_eq!(entry("sub/keep")["index"], "modified", "{staged}");
}
