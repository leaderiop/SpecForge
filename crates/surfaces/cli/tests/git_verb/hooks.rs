//! `specforge git commit`'s hooks (ADR 0339 D5): the repository's own `pre-commit`, `prepare-commit-msg`,
//! `commit-msg` and `post-commit`, found where git finds them and run as git runs them.

use serde_json::json;
use specforge_git::testing::Repo;

use super::{committed, git};
use crate::support::Cli;

/// A hook `name` of `repo` that runs `body` under `/bin/sh`.
#[cfg(unix)]
fn hook(repo: &Repo, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt as _;
    let at = repo.git_dir().join("hooks").join(name);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn a_failing_pre_commit_hook_refuses_and_nothing_is_committed() {
    let (project, repo, first) = committed();
    hook(&repo, "pre-commit", "echo 'lint failed' >&2\nexit 1");
    repo.write("a.txt", "two\n");
    let refused = git(&project, &["commit", "--add", "a.txt", "-m", "change"]).exits(1);
    let error = refused.json();
    assert_eq!(error["code"], "hook_refused", "{}", refused.stdout);
    assert!(
        error["suggestion"]
            .as_str()
            .unwrap()
            .contains("--no-verify"),
        "{error}"
    );
    assert!(refused.stderr.contains("lint failed"), "{}", refused.stderr);
    assert_eq!(repo.sha("HEAD"), first, "no commit");
    // as git leaves it: the change is staged
    let staged = git(&project, &["diff", "--staged", "--pick", ".files[].path"]).exits(0);
    assert_eq!(staged.stdout, "a.txt\n");
    // --no-verify runs no hook
    let answer = git(&project, &["commit", "--no-verify", "-m", "change"])
        .exits(0)
        .answer();
    assert_eq!(answer["files"], json!(["a.txt"]));
}

/// A repository the port may not write runs none of its hooks: the refusal comes before the `pre-commit` hook,
/// with nothing to stage. A test cannot make a git directory another user owns without root, so a rebase in
/// progress stands in: both are `Git::writable`'s refusal, which the verb asks before anything runs.
#[cfg(unix)]
#[test]
fn a_repository_the_port_may_not_write_runs_no_hook() {
    let (project, repo, first) = committed();
    hook(&repo, "pre-commit", "echo ran > pre-commit.ran");
    std::fs::create_dir_all(repo.git_dir().join("rebase-merge")).unwrap();
    let refused = git(&project, &["commit", "--allow-empty", "-m", "x"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    assert!(
        !project.root().join("pre-commit.ran").exists(),
        "the hook ran"
    );
    assert_eq!(repo.sha("HEAD"), first);
}

#[cfg(unix)]
#[test]
fn the_commit_msg_hook_edits_the_message_and_post_commit_runs_after() {
    let (project, repo, _) = committed();
    hook(&repo, "pre-commit", "echo ran > pre-commit.ran");
    hook(
        &repo,
        "commit-msg",
        "printf '\\nSigned-off-by: Hook <h@example.com>\\n' >> \"$1\"",
    );
    hook(&repo, "post-commit", "echo ran > post-commit.ran\nexit 7");
    let answer = git(&project, &["commit", "--allow-empty", "-m", "subject"])
        .exits(0)
        .answer();
    let log = git(&project, &["log", "--max", "1", "--pick", ".commits[0]"])
        .exits(0)
        .json();
    assert_eq!(log["sha"], answer["sha"]);
    assert_eq!(
        log["trailers"],
        json!({"Signed-off-by": ["Hook <h@example.com>"]})
    );
    assert!(project.root().join("pre-commit.ran").is_file());
    assert!(
        project.root().join("post-commit.ran").is_file(),
        "post-commit's exit is not the commit's"
    );
}

/// As git: `prepare-commit-msg` is handed `COMMIT_EDITMSG` and `message` (a `-m` commit) even under `--no-verify`,
/// and the trailer it appends is committed; `COMMIT_EDITMSG` is written with no `commit-msg` hook.
#[cfg(unix)]
#[test]
fn prepare_commit_msg_runs_under_no_verify_and_its_trailer_is_committed_as_git_does() {
    let (project, repo, _) = committed();
    hook(&repo, "pre-commit", "exit 1");
    hook(
        &repo,
        "prepare-commit-msg",
        "[ \"$2\" = message ] && printf '\\nGsd-Task: 7\\n' >> \"$1\"",
    );
    let answer = git(
        &project,
        &["commit", "--allow-empty", "--no-verify", "-m", "subject"],
    )
    .exits(0)
    .answer();
    let log = git(&project, &["log", "--max", "1", "--pick", ".commits[0]"])
        .exits(0)
        .json();
    assert_eq!(log["sha"], answer["sha"]);
    assert_eq!(log["trailers"], json!({"Gsd-Task": ["7"]}));
    assert_eq!(
        std::fs::read_to_string(repo.git_dir().join("COMMIT_EDITMSG")).unwrap(),
        "subject\n\nGsd-Task: 7\n"
    );
}

/// `core.hooksPath` is interpolated as git interpolates a path: `~/` is the home directory.
#[cfg(unix)]
#[test]
fn hooks_path_reads_from_the_home_directory_as_git_does() {
    use std::os::unix::fs::PermissionsExt as _;
    let (project, repo, _) = committed();
    let home = tempfile::tempdir().unwrap();
    let at = home.path().join("my-hooks/pre-commit");
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(
        &at,
        "#!/bin/sh\necho refused by the home hook >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.config("[core]\n\thooksPath = ~/my-hooks");
    let empty = tempfile::tempdir().unwrap();
    let refused = Cli::on(&project)
        .env("PATH", empty.path())
        .env("HOME", home.path())
        .run(&["git", "commit", "--allow-empty", "-m", "subject"])
        .exits(1);
    assert!(refused.stdout.contains("pre-commit"), "{}", refused.stdout);
}

#[cfg(unix)]
#[test]
fn a_hook_that_cannot_run_refuses_the_commit() {
    let (project, repo, first) = committed();
    let at = repo.git_dir().join("hooks/pre-commit");
    hook(&repo, "pre-commit", "exit 0");
    std::fs::write(&at, "#!/nonexistent/interpreter\n").unwrap();
    let refused = git(&project, &["commit", "--allow-empty", "-m", "x"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "hook_cannot_run",
        "{}",
        refused.stdout
    );
    assert_eq!(repo.sha("HEAD"), first);
}
