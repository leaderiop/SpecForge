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

#[test]
fn a_branch_is_created_at_a_revision_and_deleted_once_head_holds_it() {
    let (project, repo, first) = committed();
    repo.write("b.txt", "b\n");
    let second = repo.commit_all("second");
    assert_eq!(
        git(&project, &["branch", "create", "topic", "--at", "HEAD~1"])
            .exits(0)
            .json(),
        json!({"name": "topic", "sha": first})
    );
    assert_eq!(repo.sha("topic"), first);
    let refused = git(&project, &["branch", "create", "topic", "--at", "HEAD~1"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    git(&project, &["branch", "delete", "topic"]).exits(0);
    assert_eq!(
        git(&project, &["rev", "topic", "--pick", ".revs[0].exists"])
            .exits(0)
            .stdout,
        "false\n"
    );
    // a branch with commits HEAD does not hold needs --force; HEAD's own is never deleted
    repo.branch("ahead", &second).detach(&first);
    let refused = git(&project, &["branch", "delete", "ahead"]).exits(1);
    assert_eq!(refused.json()["code"], "not_merged", "{}", refused.stdout);
    git(&project, &["branch", "delete", "ahead", "--force"]).exits(0);
    repo.head_on("main");
    let refused = git(&project, &["branch", "delete", "main"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "current_branch",
        "{}",
        refused.stdout
    );
}

#[test]
fn switch_create_moves_head_and_switch_checks_the_files_out() {
    let (project, repo, first) = committed();
    let head = git(&project, &["switch", "--create", "topic"])
        .exits(0)
        .json();
    assert_eq!(
        (head["branch"].as_str(), head["sha"].as_str()),
        (Some("topic"), Some(first.as_str()))
    );
    repo.write("t.txt", "topic\n");
    git(&project, &["commit", "--add", "t.txt", "-m", "t"]).exits(0);
    git(&project, &["switch", "main"]).exits(0);
    assert!(!project.root().join("t.txt").exists(), "main's files");
    git(&project, &["switch", "topic"]).exits(0);
    assert!(project.root().join("t.txt").is_file(), "topic's files");
    // from a detached HEAD too
    repo.detach(&first);
    let head = git(&project, &["switch", "main"]).exits(0).json();
    assert_eq!(head["detached"], false);
}

#[test]
fn switching_over_a_changed_file_refuses_and_keeps_the_change() {
    let (project, repo, first) = committed();
    repo.branch("side", &first).head_on("side");
    repo.write("a.txt", "side\n");
    repo.commit_all("side");
    repo.head_on("main");
    repo.write("a.txt", "one\n");
    repo.stage(&["a.txt"]);
    repo.write("a.txt", "my unsaved work\n");
    let refused = git(&project, &["switch", "side"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "my unsaved work\n"
    );
    assert_eq!(
        git(&project, &["head", "--pick", ".branch"])
            .exits(0)
            .stdout,
        "main\n"
    );
    // and a refused `switch --create` leaves no branch behind
    let refused = git(&project, &["switch", "--create", "other", "--at", "side"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    assert_eq!(
        git(&project, &["rev", "other", "--pick", ".revs[0].exists"])
            .exits(0)
            .stdout,
        "false\n"
    );
}

#[test]
fn an_annotated_tag_names_a_tag_object_and_a_plain_one_the_commit() {
    let (project, repo, first) = committed();
    repo.config("[user]\n\tname = Ada\n\temail = ada@example.com");
    let tag = git(&project, &["tag", "v1", "-m", "release\n"])
        .exits(0)
        .json();
    assert_ne!(tag["sha"], json!(first));
    assert_eq!(
        git(&project, &["rev", "v1", "--pick", ".revs[0].kind"])
            .exits(0)
            .stdout,
        "tag\n"
    );
    assert_eq!(repo.sha("v1^{commit}"), first);
    assert_eq!(
        git(&project, &["tag", "v0"]).exits(0).json(),
        json!({"name": "v0", "sha": first})
    );
    let refused = git(&project, &["tag", "v0"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
}

#[test]
fn a_worktree_is_added_listed_and_removed() {
    let (project, _repo, first) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let at = elsewhere.path().join("feature-wt");
    let path = at.to_string_lossy().into_owned();
    let added = git(
        &project,
        &["worktree", "add", &path, "--branch", "feature", "--create"],
    )
    .exits(0)
    .json();
    assert_eq!(
        (added["branch"].as_str(), added["sha"].as_str()),
        (Some("feature"), Some(first.as_str()))
    );
    assert!(at.join("a.txt").is_file(), "its files are checked out");
    assert_eq!(
        git(
            &project,
            &["worktrees", "--pick", ".worktrees[main=false].branch"]
        )
        .exits(0)
        .stdout,
        "feature\n"
    );
    // one with changes stays unless forced
    std::fs::write(at.join("a.txt"), "changed\n").unwrap();
    let refused = git(&project, &["worktree", "remove", &path]).exits(1);
    assert_eq!(refused.json()["code"], "dirty_tree", "{}", refused.stdout);
    git(&project, &["worktree", "remove", &path, "--force"]).exits(0);
    assert!(!at.exists());
    assert_eq!(
        git(&project, &["worktrees", "--pick", ".worktrees[].branch"])
            .exits(0)
            .stdout,
        "main\n"
    );
}

/// git's `worktree remove` validates the worktree first: one whose `.git` file is broken is refused without `--force`
/// (its files may be someone's work), and removed with it.
#[test]
fn a_worktree_whose_git_file_is_broken_is_removed_only_with_force() {
    let (project, _repo, _) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let at = elsewhere.path().join("wt");
    let path = at.to_string_lossy().into_owned();
    git(
        &project,
        &["worktree", "add", &path, "--branch", "wt", "--create"],
    )
    .exits(0);
    std::fs::write(at.join(".git"), "not a gitdir line\n").unwrap();
    let refused = git(&project, &["worktree", "remove", &path]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "broken_worktree",
        "{}",
        refused.stdout
    );
    assert!(at.join("a.txt").is_file(), "its files stay");
    git(&project, &["worktree", "remove", &path, "--force"]).exits(0);
    assert!(!at.exists());
}

#[test]
fn prune_drops_the_record_of_a_worktree_whose_directory_is_gone() {
    let (project, repo, first) = committed();
    let elsewhere = tempfile::tempdir().unwrap();
    let linked = repo.linked_worktree(&elsewhere.path().join("gone"), "gone", &first);
    std::fs::remove_dir_all(linked.dir()).unwrap();
    let pruned = git(&project, &["worktree", "prune", "--pick", ".pruned[]"]).exits(0);
    assert!(
        pruned.stdout.trim_end().ends_with("gone"),
        "{}",
        pruned.stdout
    );
    assert_eq!(
        git(&project, &["worktrees", "--pick", ".worktrees[].branch"])
            .exits(0)
            .stdout,
        "main\n"
    );
}

/// [`committed`], then a branch `side` from it with its own commit changing `file` to `side_text`, and `HEAD` back on
/// `main`.
fn side(file: &str, side_text: &str) -> (TestProject, Repo, String, String) {
    let (project, repo, first) = committed();
    repo.branch("side", &first).head_on("side");
    repo.write(file, side_text);
    let side = repo.commit_all("on side");
    repo.head_on("main");
    // back to main's files
    if file == "a.txt" {
        repo.write("a.txt", "one\n");
    } else {
        repo.delete(file);
    }
    repo.stage_all();
    (project, repo, first, side)
}

#[test]
fn a_merge_fast_forwards_when_history_allows() {
    let (project, repo, _first, side) = side("s.txt", "side\n");
    let merged = git(&project, &["merge", "side"]).exits(0).json();
    assert_eq!(
        merged,
        json!({"sha": side, "fast_forward": true, "conflicts": []})
    );
    assert_eq!(repo.sha("main"), side);
    assert!(
        project.root().join("s.txt").is_file(),
        "its files are checked out"
    );
}

#[test]
fn a_three_way_merge_records_a_commit_with_both_parents() {
    let (project, repo, first, side) = side("s.txt", "side\n");
    repo.write("m.txt", "main\n");
    let main = repo.commit_all("on main");
    let refused = git(&project, &["merge", "side", "--ff-only"]).exits(1);
    assert_eq!(refused.json()["code"], "git_refused", "{}", refused.stdout);
    let merged = git(&project, &["merge", "side"]).exits(0).json();
    assert_eq!(merged["fast_forward"], false);
    let log = git(&project, &["log", "--max", "1", "--pick", ".commits[0]"])
        .exits(0)
        .json();
    assert_eq!(log["parents"], json!([main, side]));
    assert_eq!(log["subject"], "Merge side");
    assert!(project.root().join("s.txt").is_file());
    let _ = first;
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "true\n"
    );
}

#[test]
fn a_conflicting_merge_answers_its_conflicts_and_leaves_them_to_resolve() {
    let (project, repo, first, _side) = side("a.txt", "one\nside\n");
    repo.write("a.txt", "one\nmain\n");
    let main = repo.commit_all("on main");
    let merged = git(&project, &["merge", "side"]).exits(1).json();
    assert_eq!(
        merged,
        json!({"sha": null, "fast_forward": false, "conflicts": [{"path": "a.txt", "kind": "both_modified"}]})
    );
    assert_eq!(repo.sha("HEAD"), main, "nothing recorded");
    let text = std::fs::read_to_string(project.root().join("a.txt")).unwrap();
    assert!(
        text.contains("<<<<<<<") && text.contains(">>>>>>>"),
        "{text}"
    );
    let _ = first;
    // a commit refuses while the conflict stands
    git(&project, &["commit", "-m", "x"]).exits(1);
}

#[test]
fn a_merge_over_a_changed_tracked_file_refuses() {
    let (project, repo, _first, _side) = side("s.txt", "side\n");
    repo.write("a.txt", "unsaved\n");
    let refused = git(&project, &["merge", "side"]).exits(1);
    assert_eq!(refused.json()["code"], "dirty_tree", "{}", refused.stdout);
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "unsaved\n"
    );
}

#[test]
fn reset_soft_mixed_and_hard_move_head_the_index_and_the_files() {
    let (project, repo, first) = committed();
    repo.write("a.txt", "two\n");
    repo.commit_all("second");
    git(&project, &["reset", "--soft", "HEAD~1"]).exits(0);
    assert_eq!(repo.sha("HEAD"), first);
    let staged = git(&project, &["diff", "--staged", "--pick", ".files[].path"]).exits(0);
    assert_eq!(staged.stdout, "a.txt\n", "soft keeps the index");
    git(&project, &["reset"]).exits(0);
    let staged = git(&project, &["diff", "--staged", "--pick", ".count"]).exits(0);
    assert_eq!(staged.stdout, "0\n", "mixed resets the index");
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "two\n"
    );
    let refused = git(&project, &["reset", "--hard"]).exits(1);
    assert_eq!(refused.json()["code"], "dirty_tree", "{}", refused.stdout);
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "two\n"
    );
    git(&project, &["reset", "--hard", "--yes"]).exits(0);
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "one\n"
    );
}

#[test]
fn restore_puts_a_file_back_and_staged_its_index_entry() {
    let (project, repo, _) = committed();
    repo.write("a.txt", "changed\n");
    repo.stage(&["a.txt"]);
    git(&project, &["restore", "--staged", "a.txt"]).exits(0);
    assert_eq!(
        git(&project, &["diff", "--staged", "--pick", ".count"])
            .exits(0)
            .stdout,
        "0\n"
    );
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "changed\n"
    );
    let answer = git(&project, &["restore", "a.txt"]).exits(0).json();
    assert_eq!(answer, json!({"restored": ["a.txt"]}));
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "one\n"
    );
}

#[test]
fn stash_push_cleans_the_tree_and_pop_brings_the_change_back() {
    let (project, repo, _) = committed();
    repo.write("a.txt", "work in progress\n");
    git(&project, &["stash", "push", "-m", "wip"]).exits(0);
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "true\n"
    );
    let list = git(&project, &["stash", "list", "--pick", ".stashes[].message"]).exits(0);
    assert!(list.stdout.contains("wip"), "{}", list.stdout);
    let popped = git(&project, &["stash", "pop"]).exits(0).json();
    assert_eq!(popped, json!({"stashes": []}));
    assert_eq!(
        std::fs::read_to_string(project.root().join("a.txt")).unwrap(),
        "work in progress\n"
    );
}

#[test]
fn cherry_pick_keeps_the_author_and_revert_undoes_a_commit() {
    let (project, repo, _first, side) = side("s.txt", "side\n");
    repo.write("m.txt", "main\n");
    repo.commit_all("on main");
    repo.config("[user]\n\tname = Ada\n\temail = ada@example.com");
    let picked = git(&project, &["cherry-pick", "side"]).exits(0).json();
    let log = git(&project, &["log", "--max", "1", "--pick", ".commits[0]"])
        .exits(0)
        .json();
    assert_eq!(log["sha"], picked["sha"]);
    assert_eq!(log["subject"], "on side");
    assert_eq!(log["author"]["name"], "Test", "the picked commit's author");
    assert_eq!(log["committer"]["name"], "Ada");
    assert!(project.root().join("s.txt").is_file());
    let _ = side;
    git(&project, &["revert", "HEAD"]).exits(0);
    let log = git(
        &project,
        &["log", "--max", "1", "--pick", ".commits[0].subject"],
    )
    .exits(0);
    assert_eq!(log.stdout, "Revert \"on side\"\n");
    assert!(!project.root().join("s.txt").exists());
}

#[test]
fn a_conflicting_cherry_pick_answers_its_conflicts() {
    let (project, repo, _first, _side) = side("a.txt", "one\nside\n");
    repo.write("a.txt", "one\nmain\n");
    let main = repo.commit_all("on main");
    let picked = git(&project, &["cherry-pick", "side"]).exits(1).json();
    assert_eq!(
        picked["conflicts"],
        json!([{"path": "a.txt", "kind": "both_modified"}])
    );
    assert_eq!(repo.sha("HEAD"), main);
}

#[test]
fn a_commit_after_a_resolved_conflict_concludes_the_merge() {
    let (project, repo, _first, side) = side("a.txt", "one\nside\n");
    repo.write("a.txt", "one\nmain\n");
    let main = repo.commit_all("on main");
    git(&project, &["merge", "side"]).exits(1);
    repo.write("a.txt", "one\nboth\n");
    let answer = git(&project, &["commit", "--add", "a.txt", "-m", "Merge side"])
        .exits(0)
        .json();
    assert_eq!(answer["parents"], json!([main, side]));
    assert!(
        !repo.git_dir().join("MERGE_HEAD").exists(),
        "the merge is ended"
    );
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "true\n"
    );
}
