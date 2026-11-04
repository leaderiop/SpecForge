//! `specforge git` (ADR 0339): the `Git` port for people and agents, on repositories gix builds, run with no git
//! program on `PATH`.

use serde_json::json;
use specforge_git::testing::Repo;

use crate::support::{Cli, TestProject};

mod signing;
mod writes;

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

/// [`committed`], then a second commit on `main` with a body and trailers: `a.txt` changed, `b.txt` added.
fn two_commits() -> (TestProject, Repo, String, String) {
    let (project, repo, first) = committed();
    repo.write("a.txt", "one\ntwo\n").write("b.txt", "b\n");
    let second = repo.commit_all(
        "second\n\nWhy it changed.\n\nGsd-Task: 7\nCo-Authored-By: A <a@example.com>\nCo-Authored-By: B <b@example.com>\n",
    );
    (project, repo, first, second)
}

#[test]
fn log_answers_a_range_with_its_messages_and_trailers() {
    let (project, _repo, first, second) = two_commits();
    let log = git(&project, &["log", "HEAD~1..HEAD"]).exits(0).json();
    assert_eq!(log["count"], 1);
    let commit = &log["commits"][0];
    assert_eq!(
        (
            commit["sha"].clone(),
            commit["parents"].clone(),
            commit["subject"].clone(),
            commit["body"].clone(),
            commit["trailers"].clone(),
        ),
        (
            json!(second),
            json!([first]),
            json!("second"),
            json!(
                "Why it changed.\n\nGsd-Task: 7\nCo-Authored-By: A <a@example.com>\nCo-Authored-By: B <b@example.com>"
            ),
            json!({
                "Co-Authored-By": ["A <a@example.com>", "B <b@example.com>"],
                "Gsd-Task": ["7"],
            }),
        )
    );
    assert_eq!(
        (
            commit["author"]["name"].clone(),
            commit["author"]["email"].clone()
        ),
        (json!("Test"), json!("test@example.com"))
    );
    assert_eq!(
        git(&project, &["log", "--pick", ".count"]).exits(0).stdout,
        "2\n"
    );
    assert_eq!(
        git(
            &project,
            &["log", "--max", "1", "--pick", ".commits[0].sha"]
        )
        .exits(0)
        .stdout,
        format!("{second}\n")
    );
    // only the commits that change b.txt
    assert_eq!(
        git(
            &project,
            &["log", "--pick", ".commits[].subject", "--", "b.txt"]
        )
        .exits(0)
        .stdout,
        "second\n"
    );
    let refused = git(&project, &["log", "nosuch..HEAD"]);
    assert_eq!(
        refused.json()["code"],
        "unknown_revision",
        "{}",
        refused.stdout
    );
}

#[test]
fn log_with_three_dots_walks_what_either_side_reaches_and_not_both() {
    let (project, repo, first) = committed();
    repo.write("ours.txt", "o");
    repo.commit_all("ours");
    repo.branch("side", &first).head_on("side");
    repo.write("theirs.txt", "t");
    repo.commit_all("theirs");
    let subjects = git(
        &project,
        &["log", "main...side", "--pick", ".commits[].subject"],
    )
    .exits(0)
    .stdout;
    let mut subjects: Vec<&str> = subjects.lines().collect();
    subjects.sort_unstable();
    assert_eq!(subjects, ["ours", "theirs"]);
    assert_eq!(
        git(&project, &["merge-base", "main", "side"])
            .exits(0)
            .json(),
        json!({"sha": first})
    );
}

/// A criss-cross history (each side merged the other's first commit) has two merge bases; `A...B` hides what either
/// reaches, as git's symmetric difference does, so neither side's first commit is listed.
#[test]
fn log_with_three_dots_hides_every_merge_base_of_a_criss_cross() {
    let (project, repo, first) = committed();
    repo.write("a.txt", "a1");
    let a1 = repo.commit_all("a1");
    repo.branch("side", &first).head_on("side");
    repo.write("b.txt", "b1");
    let b1 = repo.commit_all("b1");
    repo.merge("mb", &a1);
    repo.head_on("main");
    repo.merge("ma", &b1);
    let subjects = git(
        &project,
        &["log", "main...side", "--pick", ".commits[].subject"],
    )
    .exits(0)
    .stdout;
    let mut subjects: Vec<&str> = subjects.lines().collect();
    subjects.sort_unstable();
    assert_eq!(subjects, ["ma", "mb"]);
}

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
    let diff = git(&project, &["diff", "--base", "main"]).exits(0).json();
    assert_eq!(diff["count"], 3);
    assert_eq!(
        diff["files"],
        json!([
            {"path": "a.txt", "old_path": null, "status": "modified", "added": 1, "removed": 0, "binary": false},
            {"path": "bin.dat", "old_path": null, "status": "added", "added": 0, "removed": 0, "binary": true},
            {"path": "c.txt", "old_path": null, "status": "added", "added": 1, "removed": 0, "binary": false},
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
fn show_answers_a_file_at_a_revision() {
    let (project, repo, _, _) = two_commits();
    assert_eq!(
        git(&project, &["show", "HEAD~1:a.txt", "--pick", ".text"])
            .exits(0)
            .stdout,
        "one\n\n"
    );
    assert_eq!(
        git(&project, &["show", "HEAD:a.txt"]).exits(0).json(),
        json!({"sha": repo.sha("HEAD:a.txt"), "bytes": 8, "binary": false, "truncated": false, "text": "one\ntwo\n"})
    );
    let refused = git(&project, &["show", "HEAD"]);
    assert_eq!(refused.json()["code"], "not_a_blob", "{}", refused.stdout);
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
        json!([{"path": "moved.txt", "old_path": "a.txt", "status": "renamed", "added": 0, "removed": 0, "binary": false}])
    );
}

#[test]
fn branches_name_their_upstream_and_how_far_each_is_ahead_and_behind() {
    let (project, repo, first) = committed();
    repo.write("b.txt", "b");
    let second = repo.commit_all("second");
    repo.remote("origin", "https://example.com/acme/widgets.git")
        .upstream("main", &first);
    repo.branch("side", &first);
    assert_eq!(
        git(&project, &["branches"]).exits(0).json(),
        json!({"branches": [
            {"name": "main", "sha": second, "current": true, "upstream": "origin/main", "ahead": 1, "behind": 0},
            {"name": "side", "sha": first, "current": false, "upstream": null, "ahead": 0, "behind": 0},
        ]})
    );
    // a clone's `origin/HEAD` names the default branch: listed once, under its own name
    std::fs::write(
        repo.git_dir().join("refs/remotes/origin/HEAD"),
        "ref: refs/remotes/origin/main\n",
    )
    .unwrap();
    assert_eq!(
        git(
            &project,
            &["branches", "--remote", "--pick", ".branches[].name"]
        )
        .exits(0)
        .stdout,
        "origin/main\n"
    );
}

#[test]
fn worktrees_list_the_main_one_then_the_linked_ones() {
    let (project, repo, sha) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&elsewhere.path().join("wt"), "feature", &sha);
    let slashed = |p: &std::path::Path| {
        p.canonicalize()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
    };
    assert_eq!(
        git(&project, &["worktrees"]).exits(0).json(),
        json!({"worktrees": [
            {"path": slashed(project.root()), "branch": "main", "sha": sha, "main": true, "locked": false, "current": true},
            {"path": slashed(linked.dir()), "branch": "feature", "sha": sha, "main": false, "locked": false, "current": false},
        ]})
    );
}

/// A URL's credentials never reach the answer: an http URL loses its user and password (a token rides in either), an
/// ssh one keeps its login name and loses a password.
#[test]
fn remotes_never_print_the_credentials_in_a_url() {
    let (project, repo, _) = committed();
    repo.remote(
        "origin",
        "https://bot:ghp_secret@github.com/acme/widgets.git",
    )
    .config("[remote \"origin\"]\n\tpushurl = https://ghp_secret@github.com/acme/widgets.git")
    .remote("ssh", "ssh://git:hunter2@gitlab.com/group/tool.git");
    let out = git(&project, &["remotes"]).exits(0);
    assert!(
        !out.stdout.contains("ghp_secret") && !out.stdout.contains("hunter2"),
        "{}",
        out.stdout
    );
    assert_eq!(
        out.json()["remotes"],
        json!([
            {"name": "origin", "url": "https://github.com/acme/widgets.git", "push_url": "https://github.com/acme/widgets.git", "host": "github.com", "path": "acme/widgets"},
            {"name": "ssh", "url": "ssh://git@gitlab.com/group/tool.git", "push_url": "ssh://git@gitlab.com/group/tool.git", "host": "gitlab.com", "path": "group/tool"},
        ])
    );
}

#[test]
fn remotes_split_each_url_into_its_host_and_path() {
    let (project, repo, _) = committed();
    repo.remote("origin", "https://github.com/acme/widgets.git")
        .config("[remote \"origin\"]\n\tpushurl = git@github.com:acme/widgets.git")
        .remote("scp", "git@gitlab.com:group/sub/tool.git")
        .remote("local", "/srv/git/mirror.git");
    assert_eq!(
        git(&project, &["remotes"]).exits(0).json(),
        json!({"remotes": [
            {"name": "local", "url": "/srv/git/mirror.git", "push_url": "/srv/git/mirror.git", "host": null, "path": "/srv/git/mirror.git"},
            {"name": "origin", "url": "https://github.com/acme/widgets.git", "push_url": "git@github.com:acme/widgets.git", "host": "github.com", "path": "acme/widgets"},
            {"name": "scp", "url": "git@gitlab.com:group/sub/tool.git", "push_url": "git@gitlab.com:group/sub/tool.git", "host": "gitlab.com", "path": "group/sub/tool"},
        ]})
    );
}
