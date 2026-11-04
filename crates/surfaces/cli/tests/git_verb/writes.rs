//! `specforge git`'s writes (ADR 0339 D5): each on a repository gix builds, with no git program on `PATH`, so what
//! changes the repository is the host's own, and its hooks run through the Process port.

use serde_json::json;
use specforge_git::testing::Repo;

use super::{committed, git};
use crate::support::{Cli, TestProject};

/// `specforge git <args>` run from `dir` with no `--path`, and a `PATH` that holds no program.
fn git_in(project: &TestProject, dir: &std::path::Path, args: &[&str]) -> crate::support::Run {
    let empty = tempfile::tempdir().unwrap();
    let mut all = vec!["git"];
    all.extend_from_slice(args);
    Cli::on(project)
        .env("PATH", empty.path())
        .in_dir(dir)
        .bare(&all)
}

/// A hook `name` of `repo` that runs `body` under `/bin/sh`.
#[cfg(unix)]
fn hook(repo: &Repo, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt as _;
    let at = repo.git_dir().join("hooks").join(name);
    std::fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::fs::write(&at, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn commit_stages_the_paths_named_and_keeps_the_trailers_byte_for_byte() {
    let (project, repo, first) = committed();
    repo.write("spec/a.spec", "a\n")
        .write("loose.txt", "not staged\n");
    let message = "feat: spec\n\nGsd-Task: 7\nCo-Authored-By: A <a@example.com>\n";
    let answer = git(&project, &["commit", "--add", "spec", "-m", message])
        .exits(0)
        .json();
    let sha = answer["sha"].as_str().unwrap().to_string();
    assert_eq!(
        answer,
        json!({"sha": sha, "branch": "main", "parents": [first], "files": ["spec/a.spec"]})
    );
    assert_eq!(
        git(
            &project,
            &["log", "--max", "1", "--pick", ".commits[0].subject"]
        )
        .exits(0)
        .stdout,
        "feat: spec\n"
    );
    assert_eq!(
        repo.open()
            .rev_parse_single(sha.as_str())
            .unwrap()
            .object()
            .unwrap()
            .into_commit()
            .message_raw()
            .unwrap()
            .to_string(),
        message,
        "the message is kept byte for byte"
    );
    // the loose ref gsd-git's detection reads
    assert_eq!(
        std::fs::read_to_string(repo.git_dir().join("refs/heads/main")).unwrap(),
        format!("{sha}\n")
    );
    assert_eq!(
        git(&project, &["status", "--pick", ".untracked[]"])
            .exits(0)
            .stdout,
        "loose.txt\n",
        "a path not named is not staged"
    );
}

#[test]
fn commit_is_made_by_the_identity_the_repository_configures() {
    let (project, repo, _) = committed();
    repo.config("[user]\n\tname = Ada\n\temail = ada@example.com");
    repo.write("a.txt", "two\n");
    git(&project, &["commit", "--all", "-m", "change"]).exits(0);
    let author = git(
        &project,
        &["log", "--max", "1", "--pick", ".commits[0].author"],
    )
    .exits(0)
    .json();
    assert_eq!(
        (author["name"].as_str(), author["email"].as_str()),
        (Some("Ada"), Some("ada@example.com"))
    );
}

#[test]
fn commit_all_stages_tracked_changes_and_leaves_untracked_files_out() {
    let (project, repo, _) = committed();
    repo.write("b.txt", "b\n");
    repo.commit_all("b");
    repo.write("a.txt", "dirty\n")
        .delete("b.txt")
        .write("new.txt", "untracked\n");
    let answer = git(&project, &["commit", "-a", "-m", "tracked only"])
        .exits(0)
        .json();
    assert_eq!(answer["files"], json!(["a.txt", "b.txt"]));
    let status = git(&project, &["status"]).exits(0).json();
    assert_eq!(status["untracked"], json!(["new.txt"]));
}

#[test]
fn commit_with_nothing_staged_refuses_unless_empty_is_allowed() {
    let (project, _repo, first) = committed();
    let refused = git(&project, &["commit", "-m", "nothing"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    let empty = git(&project, &["commit", "--allow-empty", "-m", "marker"])
        .exits(0)
        .json();
    assert_eq!(empty["parents"], json!([first]));
    assert_eq!(empty["files"], json!([]));
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
        .json();
    assert_eq!(answer["files"], json!(["a.txt"]));
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
        .json();
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

#[test]
fn a_repository_that_signs_every_commit_refuses() {
    let (project, repo, first) = committed();
    repo.config("[commit]\n\tgpgsign = true");
    let refused = git(&project, &["commit", "--allow-empty", "-m", "x"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "signing_unsupported",
        "{}",
        refused.stdout
    );
    assert_eq!(repo.sha("HEAD"), first);
}

#[test]
fn a_ceiling_with_git_read_refuses_a_commit() {
    let (project, repo, first) = committed();
    let project = project.config(|config| {
        config["effects"] = json!({"git": {"ops": ["read"]}});
    });
    let refused = git(&project, &["commit", "--allow-empty", "-m", "x"]).exits(1);
    assert_eq!(refused.json()["code"], "E090", "{}", refused.stdout);
    assert_eq!(repo.sha("HEAD"), first);
    git(&project, &["head"]).exits(0);
}

#[test]
fn a_path_outside_the_repository_is_refused() {
    let (project, _repo, _) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let outside = elsewhere.path().to_string_lossy().into_owned();
    let refused = git(&project, &["commit", "--add", &outside, "-m", "x"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "outside_repository",
        "{}",
        refused.stdout
    );
}

#[test]
fn a_conflict_in_the_index_refuses_a_commit() {
    let (project, repo, first) = committed();
    repo.conflict("a.txt", "base\n", "ours\n", "theirs\n");
    let refused = git(&project, &["commit", "-m", "x"]).exits(1);
    let error = refused.json();
    assert_eq!(error["code"], "git_refused", "{}", refused.stdout);
    assert!(
        error["error"].as_str().unwrap().contains("conflict"),
        "{error}"
    );
    assert_eq!(repo.sha("HEAD"), first);
}

#[test]
fn a_commit_on_a_detached_head_moves_head_alone() {
    let (project, repo, first) = committed();
    repo.detach(&first);
    let answer = git(&project, &["commit", "--allow-empty", "-m", "detached"])
        .exits(0)
        .json();
    assert_eq!(answer["branch"], json!(null));
    assert_eq!(repo.sha("HEAD"), answer["sha"].as_str().unwrap());
    assert_eq!(repo.sha("main"), first, "no branch moved");
}

#[test]
fn a_commit_in_a_linked_worktree_moves_its_branch() {
    let (project, repo, first) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&elsewhere.path().join("wt"), "feature", &first);
    linked.write("f.txt", "feature\n");
    let answer = git_in(
        &project,
        linked.dir(),
        &["commit", "--add", "f.txt", "-m", "f"],
    )
    .exits(0)
    .json();
    assert_eq!(answer["branch"], "feature");
    assert_eq!(answer["files"], json!(["f.txt"]));
    assert_eq!(repo.sha("feature"), answer["sha"].as_str().unwrap());
    assert_eq!(repo.sha("main"), first);
}
