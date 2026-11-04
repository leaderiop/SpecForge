//! `specforge git` (ADR 0339): the `Git` port for people and agents, on repositories gix builds, run with no git
//! program on `PATH`.

use serde_json::json;
use specforge_git::testing::Repo;

use crate::support::{Cli, TestProject};

/// A project in a repository with one commit, `HEAD` on `main`.
fn committed() -> (TestProject, Repo, String) {
    let project = TestProject::new();
    let repo = Repo::init(project.root());
    repo.write(".gitignore", ".specforge/\n")
        .write("a.txt", "one\n");
    let sha = repo.commit_all("first");
    (project, repo, sha)
}

/// `specforge git <args>` in `project`, with a `PATH` that holds no program.
fn git(project: &TestProject, args: &[&str]) -> crate::support::Run {
    let empty = tempfile::tempdir().unwrap();
    let mut all = vec!["git"];
    all.extend_from_slice(args);
    Cli::on(project).env("PATH", empty.path()).run(&all)
}

#[test]
fn repo_names_the_top_and_the_git_directory() {
    let (project, _repo, _) = committed();
    let top = project.root().canonicalize().unwrap();
    let top = top.to_string_lossy().replace('\\', "/");
    assert_eq!(
        git(&project, &["repo"]).exits(0).json(),
        json!({
            "top": top,
            "git_dir": format!("{top}/.git"),
            "common_dir": format!("{top}/.git"),
            "linked": false,
            "shallow": false,
        })
    );
}

#[test]
fn head_names_the_branch_and_its_commit() {
    let (project, repo, sha) = committed();
    assert_eq!(
        git(&project, &["head"]).exits(0).json(),
        json!({
            "sha": sha,
            "branch": "main",
            "detached": false,
            "unborn": false,
            "upstream": null,
            "ahead": 0,
            "behind": 0,
        })
    );
    assert_eq!(
        git(&project, &["head", "--pick", ".branch"])
            .exits(0)
            .stdout,
        "main\n"
    );
    repo.detach(&sha);
    let detached = git(&project, &["head"]).exits(0).json();
    assert_eq!(
        (detached["detached"].clone(), detached["branch"].clone()),
        (json!(true), json!(null))
    );
    repo.head_on("next");
    let unborn = git(&project, &["head"]).exits(0).json();
    assert_eq!(
        (
            unborn["unborn"].clone(),
            unborn["sha"].clone(),
            unborn["branch"].clone()
        ),
        (json!(true), json!(null), json!("next"))
    );
}

#[test]
fn head_counts_the_commits_ahead_of_and_behind_the_upstream() {
    let (project, repo, first) = committed();
    repo.write("b.txt", "two\n");
    let theirs = repo.commit_all("theirs");
    repo.branch("main", &first);
    repo.delete("b.txt").write("c.txt", "three\n");
    repo.commit_all("ours");
    repo.remote("origin", "https://example.com/acme/widgets.git")
        .upstream("main", &theirs);
    let head = git(&project, &["head"]).exits(0).json();
    assert_eq!(
        (
            head["upstream"].clone(),
            head["ahead"].clone(),
            head["behind"].clone()
        ),
        (json!("origin/main"), json!(1), json!(1))
    );
}

#[test]
fn rev_names_each_revision_or_that_it_names_nothing() {
    let (project, repo, sha) = committed();
    let blob = repo.sha("HEAD:a.txt");
    assert_eq!(
        git(&project, &["rev", "HEAD", "main:a.txt", "nosuch"])
            .exits(0)
            .json(),
        json!({"revs": [
            {"rev": "HEAD", "sha": sha, "kind": "commit", "exists": true},
            {"rev": "main:a.txt", "sha": blob, "kind": "blob", "exists": true},
            {"rev": "nosuch", "sha": null, "kind": null, "exists": false},
        ]})
    );
    assert_eq!(
        git(&project, &["rev", "nosuch", "--pick", ".revs[0].exists"])
            .exits(0)
            .stdout,
        "false\n"
    );
}

#[test]
fn status_is_typed_with_clean_and_the_untracked_paths() {
    let (project, repo, sha) = committed();
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "true\n"
    );
    repo.write("a.txt", "changed\n").write("new.txt", "n");
    assert_eq!(
        git(&project, &["status"]).exits(0).json(),
        json!({
            "branch": {"oid": sha, "head": "main", "ahead": 0, "behind": 0},
            "entries": [
                {"kind": "changed", "path": "a.txt", "worktree": "modified"},
                {"kind": "untracked", "path": "new.txt"},
            ],
            "clean": false,
            "untracked": ["new.txt"],
        })
    );
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "false\n"
    );
}

#[test]
fn config_answers_every_value_and_the_one_git_applies() {
    let (project, repo, _) = committed();
    repo.config("[user]\n\tname = Ada");
    assert_eq!(
        git(&project, &["config", "user.name"]).exits(0).json(),
        json!({"key": "user.name", "value": "Ada", "values": ["Test", "Ada"]})
    );
    assert_eq!(
        git(&project, &["config", "no.such"]).exits(0).json(),
        json!({"key": "no.such", "value": null, "values": []})
    );
}

#[test]
fn a_directory_in_no_repository_is_refused() {
    let project = TestProject::new();
    let refused = git(&project, &["head"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "not_a_repository",
        "{}",
        refused.stdout
    );
}

#[test]
fn a_ceiling_with_git_off_refuses() {
    let (project, _repo, _) = committed();
    let project = project.config(|config| {
        config["effects"] = json!({"git": "off"});
    });
    let refused = git(&project, &["head"]).exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
}
