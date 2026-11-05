//! `specforge git` (ADR 0339): the `Git` port for people and agents, on repositories gix builds, run with no git
//! program on `PATH`.

use serde_json::json;
use specforge_git::testing::Repo;

use crate::support::{Cli, TestProject};

mod diff;
mod hooks;
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
        git(&project, &["repo"]).exits(0).answer(),
        json!({
            "top": top,
            "git_dir": format!("{top}/.git"),
            "common_dir": format!("{top}/.git"),
            "linked": false,
            "shallow": false,
            "prefix": "",
            "superproject": null,
        })
    );
}

/// `--in <dir>` runs a verb as if typed in `dir`, as `git -C` does: the repository `dir` is in (a nested one
/// included), `dir`'s prefix, and the repository the nested one is a submodule of.
#[test]
fn in_runs_a_verb_as_if_typed_in_its_directory() {
    let (project, repo, _) = committed();
    let top = project.root().canonicalize().unwrap();
    let top = top.to_string_lossy().replace('\\', "/");
    std::fs::create_dir_all(project.root().join("docs/deep")).unwrap();
    let deep = git(&project, &["repo", "--in", "docs/deep"])
        .exits(0)
        .answer();
    assert_eq!(
        (deep["top"].clone(), deep["prefix"].clone()),
        (json!(top), json!("docs/deep/"))
    );
    let nested = Repo::init(&project.root().join("vendor/lib"));
    nested.write("x.txt", "x\n");
    let inner = nested.commit_all("inner");
    let answer = git(&project, &["repo", "--in", "vendor/lib"])
        .exits(0)
        .answer();
    assert_eq!(
        (
            answer["top"].clone(),
            answer["prefix"].clone(),
            answer["superproject"].clone()
        ),
        (json!(format!("{top}/vendor/lib")), json!(""), json!(null)),
        "a nested repository is a submodule only once the outer index records it"
    );
    repo.gitlink("vendor/lib", &inner);
    let picked = |args: &[&str]| git(&project, args).exits(0).stdout;
    assert_eq!(
        picked(&["repo", "--in", "vendor/lib", "--pick", ".superproject"]),
        format!("{top}\n")
    );
    assert_eq!(
        picked(&["log", "--in", "vendor/lib", "--pick", ".commits[].subject"]),
        "inner\n"
    );
    git(&project, &["repo", "--in", "nosuch"]).exits(2);
}

#[test]
fn head_names_the_branch_and_its_commit() {
    let (project, repo, sha) = committed();
    assert_eq!(
        git(&project, &["head"]).exits(0).answer(),
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
    let detached = git(&project, &["head"]).exits(0).answer();
    assert_eq!(
        (detached["detached"].clone(), detached["branch"].clone()),
        (json!(true), json!(null))
    );
    repo.head_on("next");
    let unborn = git(&project, &["head"]).exits(0).answer();
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
    let head = git(&project, &["head"]).exits(0).answer();
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
            .answer(),
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
fn status_is_typed_with_clean_and_the_tracked_and_untracked_paths() {
    let (project, repo, sha) = committed();
    assert_eq!(
        git(&project, &["status", "--pick", ".clean"])
            .exits(0)
            .stdout,
        "true\n"
    );
    repo.write("a.txt", "changed\n").write("new.txt", "n");
    assert_eq!(
        git(&project, &["status"]).exits(0).answer(),
        json!({
            "branch": {"oid": sha, "head": "main", "ahead": 0, "behind": 0},
            "entries": [
                {"kind": "changed", "path": "a.txt", "worktree": "modified"},
                {"kind": "untracked", "path": "new.txt"},
            ],
            "clean": false,
            "tracked": ["a.txt"],
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
        git(&project, &["config", "user.name"]).exits(0).answer(),
        json!({"key": "user.name", "value": "Ada", "values": ["Test", "Ada"]})
    );
    assert_eq!(
        git(&project, &["config", "no.such"]).exits(0).answer(),
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
    let log = git(&project, &["log", "HEAD~1..HEAD"]).exits(0).answer();
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
            .answer(),
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
        git(&project, &["show", "HEAD:a.txt"]).exits(0).answer(),
        json!({"sha": repo.sha("HEAD:a.txt"), "bytes": 8, "binary": false, "truncated": false, "text": "one\ntwo\n"})
    );
    let refused = git(&project, &["show", "HEAD"]);
    assert_eq!(refused.json()["code"], "not_a_blob", "{}", refused.stdout);
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
        git(&project, &["branches"]).exits(0).answer(),
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
        git(&project, &["worktrees"]).exits(0).answer(),
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
        out.answer()["remotes"],
        json!([
            {"name": "origin", "url": "https://github.com/acme/widgets.git", "push_url": "https://github.com/acme/widgets.git", "host": "github.com", "path": "acme/widgets", "head": null},
            {"name": "ssh", "url": "ssh://git@gitlab.com/group/tool.git", "push_url": "ssh://git@gitlab.com/group/tool.git", "host": "gitlab.com", "path": "group/tool", "head": null},
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
        git(&project, &["remotes"]).exits(0).answer(),
        json!({"remotes": [
            {"name": "local", "url": "/srv/git/mirror.git", "push_url": "/srv/git/mirror.git", "host": null, "path": "/srv/git/mirror.git", "head": null},
            {"name": "origin", "url": "https://github.com/acme/widgets.git", "push_url": "git@github.com:acme/widgets.git", "host": "github.com", "path": "acme/widgets", "head": null},
            {"name": "scp", "url": "git@gitlab.com:group/sub/tool.git", "push_url": "git@gitlab.com:group/sub/tool.git", "host": "gitlab.com", "path": "group/sub/tool", "head": null},
        ]})
    );
}

/// `log` keeps the commits git's flags keep: `--grep` (a regex per line), `--no-merges`, `--since`/`--until` (git's
/// dates), `-S` (a text, or a regex with `--pickaxe-regex`, within the paths), and lists them `--reverse`.
#[test]
fn log_filters_as_git_filters_and_lists_oldest_first() {
    let (project, repo, first) = committed();
    repo.write("plan.spec", "plan 7 \"x\" {}\n");
    let planned = repo.commit_all("feat(01-02): plan it\n\nbody");
    repo.write("b.txt", "plan 7 in prose\n");
    let red = repo.commit_all("test(01-02): red");
    let merge = repo.merge("Merge side", &first);
    let picked = |args: &[&str]| git(&project, args).exits(0).stdout;
    let shas = |shas: &[&str]| shas.iter().map(|s| format!("{s}\n")).collect::<String>();
    assert_eq!(
        picked(&[
            "log",
            "--grep",
            r"^feat\((0*1)-(0*2)\):",
            "--pick",
            ".commits[].sha"
        ]),
        shas(&[&planned])
    );
    assert_eq!(
        picked(&["log", "--grep", "^body$", "--pick", ".count"]),
        "1\n"
    );
    assert_eq!(picked(&["log", "--no-merges", "--pick", ".count"]), "3\n");
    assert_eq!(
        picked(&["log", "--reverse", "--pick", ".commits[].sha"]),
        shas(&[&first, &planned, &red, &merge])
    );
    assert_eq!(
        picked(&["log", "--reverse", "--max", "2", "--pick", ".commits[].sha"]),
        shas(&[&red, &merge]),
        "--max keeps the newest, as git applies -n before --reverse"
    );
    assert_eq!(
        picked(&["log", "-S", "plan 7", "--pick", ".commits[].sha"]),
        shas(&[&red, &planned])
    );
    assert_eq!(
        picked(&[
            "log",
            "-S",
            "^plan 7 ",
            "--pickaxe-regex",
            "--pick",
            ".commits[].sha",
            "--",
            "plan.spec"
        ]),
        shas(&[&planned])
    );
    // the test clock: the first commit at 1700000000 (22:13:20 UTC), each next one a minute later
    assert_eq!(
        picked(&[
            "log",
            "--since",
            "2023-11-14 22:14:00 +0000",
            "--until",
            "2023-11-14 22:15:30 +0000",
            "--pick",
            ".commits[].sha"
        ]),
        shas(&[&red, &planned])
    );
    assert_eq!(
        picked(&["log", "--since", "24 hours ago", "--pick", ".count"]),
        "0\n"
    );
    git(&project, &["log", "--since", "not a date"]).exits(1);
    git(&project, &["log", "--grep", "feat("]).exits(1);
}

/// `rev --sorted` orders commits by committer date as `rev-list --no-walk=sorted`, newest first or `--reverse`, reading
/// more revisions from `--stdin`; a revision that names no commit refuses them all.
#[test]
fn rev_sorted_orders_commits_by_their_date() {
    let (project, repo, first) = committed();
    repo.write("b.txt", "b");
    let second = repo.commit_all("second");
    repo.write("c.txt", "c");
    let third = repo.commit_all("third");
    let picked = |args: &[&str]| git(&project, args).exits(0).stdout;
    assert_eq!(
        picked(&[
            "rev",
            "--sorted",
            &second,
            &first,
            &third,
            "--pick",
            ".revs[].sha"
        ]),
        format!("{third}\n{second}\n{first}\n")
    );
    assert_eq!(
        picked(&[
            "rev",
            "--sorted",
            "--reverse",
            &third,
            &first,
            "--pick",
            ".revs[0].sha"
        ]),
        format!("{first}\n")
    );
    let empty = tempfile::tempdir().unwrap();
    let piped = Cli::on(&project)
        .env("PATH", empty.path())
        .stdin(format!("{first}\n{third}\n"))
        .run(&["git", "rev", "--sorted", "--stdin", "--pick", ".revs[].sha"])
        .exits(0)
        .stdout;
    assert_eq!(piped, format!("{third}\n{first}\n"));
    assert_eq!(picked(&["rev", "--sorted", "--pick", ".revs[].sha"]), "");
    let refused = git(&project, &["rev", "--sorted", &first, "nosuch"]).exits(1);
    assert_eq!(
        refused.json()["code"],
        "unknown_revision",
        "{}",
        refused.stdout
    );
}

/// `submodules` reads `.gitmodules` as git's config parser does, and lists no path that leaves the top.
#[test]
fn submodules_list_what_gitmodules_declares() {
    let (project, repo, _) = committed();
    assert_eq!(
        git(&project, &["submodules"]).exits(0).answer(),
        json!({"submodules": []})
    );
    repo.write(
        ".gitmodules",
        "[submodule \"lib\"]\n\tpath = vendor/lib\n\turl = https://bot:tok@example.com/acme/lib.git\n\
         [submodule \"out\"]\n\tpath = ../escape\n\turl = ../escape.git\n\
         [submodule \"docs\"]\n\tpath = docs/site\n",
    );
    assert_eq!(
        git(&project, &["submodules"]).exits(0).answer(),
        json!({"submodules": [
            {"name": "lib", "path": "vendor/lib", "url": "https://example.com/acme/lib.git"},
            {"name": "docs", "path": "docs/site", "url": null},
        ]})
    );
}

/// `origin/HEAD`, which a clone writes, names the remote's default branch.
#[test]
fn remotes_name_the_default_branch_their_head_points_at() {
    let (project, repo, sha) = committed();
    repo.remote("origin", "https://github.com/acme/widgets.git")
        .remote("fork", "https://github.com/me/widgets.git")
        .set_ref("refs/remotes/origin/trunk", &sha);
    std::fs::write(
        repo.git_dir().join("refs/remotes/origin/HEAD"),
        "ref: refs/remotes/origin/trunk\n",
    )
    .unwrap();
    assert_eq!(
        git(&project, &["remotes", "--pick", ".remotes[].head"])
            .exits(0)
            .stdout,
        "trunk\n",
        "a remote with no HEAD has none"
    );
}

/// A history read is local: `replay` with an empty journal performs it, and the answer says so.
#[test]
fn git_takes_every_effects_mode_and_performs_the_read_in_each() {
    let (project, _, sha) = committed();
    for mode in ["online", "cached", "replay"] {
        let head = git(&project, &["head", "--effects", mode]).exits(0).json();
        assert_eq!(head["sha"], sha.as_str(), "{mode}: {head}");
        assert_eq!(head["effect"]["from_journal"], false, "{mode}");
        assert_eq!(head["effect"]["digest"], serde_json::Value::Null, "{mode}");
    }
}
